{
  inputs = {
    claude-code = {
      url = "github:sadjow/claude-code-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      claude-code,
      crane,
      flake-utils,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      systemOutputs = flake-utils.lib.eachDefaultSystem (
        system:
        let
          pkgs_overlay = system: final: prev: {
            external.claude-code = claude-code.packages.${system}.default;
          };
          overlays = [
            (pkgs_overlay system)
            (import rust-overlay)
          ];
          pkgs = import nixpkgs {
            inherit system overlays;
            config.allowUnfree = true;
          };
          inherit (pkgs) lib;

          rustMinimal = pkgs.rust-bin.stable.latest.minimal.override {
            targets = [ "wasm32-unknown-unknown" ];
          };
          rustDev = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-analyzer"
              "rust-src"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain rustMinimal;

          src =
            let
              extraFilter =
                path: _type:
                builtins.match ".*/migrations(/.*)?" path != null
                || builtins.match ".*/crates/ui/assets(/.*)?" path != null;
              cargoFilter = craneLib.filterCargoSources;
            in
            pkgs.lib.cleanSourceWith {
              src = ./.;
              filter = path: type: (extraFilter path type) || (cargoFilter path type);
              name = "source";
            };

          # `dx` shells out to `wasm-bindgen` and requires the exact version the
          # project builds against, so read that out of Cargo.lock.
          lockFile = builtins.fromTOML (builtins.readFile ./Cargo.lock);
          wasmBindgenVersion =
            (builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lockFile.package)).version;
          dioxusVersion = (builtins.head (builtins.filter (p: p.name == "dioxus") lockFile.package)).version;

          # nixpkgs ships one attribute per wasm-bindgen release. Fall back to
          # the newest it has when Cargo.lock runs ahead, so a version gap can
          # never break eval and block `just up` from closing it.
          wasmBindgenCli =
            let
              attr = "wasm-bindgen-cli_${builtins.replaceStrings [ "." ] [ "_" ] wasmBindgenVersion}";
              available = builtins.filter (lib.hasPrefix "wasm-bindgen-cli_0_") (builtins.attrNames pkgs);
              versionOf = n: builtins.replaceStrings [ "_" ] [ "." ] (lib.removePrefix "wasm-bindgen-cli_" n);
              newest = lib.last (
                builtins.sort (a: b: builtins.compareVersions (versionOf a) (versionOf b) < 0) available
              );
            in
            pkgs.${attr} or (lib.warn "nixpkgs has no ${attr}; falling back to ${newest}" pkgs.${newest});

          # nixpkgs is the source of truth for the `dx` version; `just up` pins
          # the `dioxus` crate to match. Warn rather than assert, because during
          # `just up` nixpkgs moves first and Cargo.lock trails it by a step.
          dioxusCli =
            lib.warnIf (pkgs.dioxus-cli.version != dioxusVersion)
              "nixpkgs dioxus-cli is ${pkgs.dioxus-cli.version} but Cargo.lock wants dioxus ${dioxusVersion}; run `just up`"
              pkgs.dioxus-cli;

          # NOTE: These are used both by the devShell and the devcontainer.
          devPackages =
            with pkgs;
            [
              atlas
              atuin
              bat
              binaryen
              caddy
              cargo-dist
              cargo-edit
              cargo-machete
              cargo-nextest
              external.claude-code
              diesel-cli
              dig
              fzf
              just
              libpq
              litecli
              netcat
              nodejs_latest
              openssl
              pgcli
              pkg-config
              postgresql_18
              rust-bin.nightly.latest.rustfmt
              sccache
              shellcheck
              sleek
              sqlite
              tombi
            ]
            ++ [
              rustDev
              dioxusCli
              wasmBindgenCli
            ];

          commonArgs = {
            inherit src;
            pname = "cookit";
            version = "0.1.0";
            strictDeps = true;
            nativeBuildInputs = [ pkgs.pkg-config ];
            # libpq: diesel's `postgres` feature links pq-sys (-lpq) even though
            # the runtime path uses diesel-async/tokio-postgres.
            buildInputs = [
              pkgs.libpq
              pkgs.openssl
            ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          cookit-tests = craneLib.cargoNextest (
            commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            }
          );

          package = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              nativeBuildInputs = commonArgs.nativeBuildInputs ++ [
                pkgs.binaryen
                dioxusCli
                rustMinimal
                wasmBindgenCli
              ];
              doCheck = false;
              doNotPostBuildInstallCargoBinaries = true;
              buildPhaseCargoCommand = ''
                dx build --release --platform web --package web
              '';
              installPhaseCommand = ''
                mkdir -p $out/bin
                cp target/dx/web/release/web/server $out/bin/web
                cp -r target/dx/web/release/web/public $out/bin/
              '';
            }
          );

          # Production OCI image. Layered so the (large, slow-changing) runtime
          # closure stays cached across rebuilds while only the app layer churns.
          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = "cookit";
            tag = "latest";

            # `fakeNss` gives us /etc/passwd + /etc/group so we can run as a
            # non-root user; `cacert` provides CA roots for any outbound TLS.
            contents = [
              pkgs.cacert
              pkgs.dockerTools.fakeNss
            ];

            # The server needs a writable /tmp; nothing else mutates the rootfs.
            extraCommands = ''
              mkdir -p tmp
              chmod 1777 tmp
            '';

            config = {
              # Full path: the server locates `public/` next to its own exe, so
              # it must run from $out/bin regardless of PATH or working dir.
              Cmd = [ "${package}/bin/web" ];

              # Drop root: 65534 is `nobody` from fakeNss. Port 8080 and startup
              # migrations need no privileges.
              User = "65534:65534";

              Env = [
                # Bind all interfaces; the default 127.0.0.1 is unreachable from
                # outside the container.
                "IP=0.0.0.0"
                "PORT=8080"
                "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              ];

              ExposedPorts = {
                "8080/tcp" = { };
              };
            };
          };

        in
        {
          packages = {
            default = package;
            docker = dockerImage;
            # Exposed so `just up` can read the version to pin `dioxus` to.
            inherit dioxusCli;
          };
          checks = {
            inherit cookit-tests;
          };
          devShells.default = pkgs.mkShell {
            packages = devPackages;

            shellHook = ''
              # Inside the devcontainer, these are already set based on the
              # docker network, so we don't want to overwrite them.
              if [ -z "''${DATABASE_URL:-}" ] && command -v devconcurrent >/dev/null; then
                export WORKSPACE="$(devconcurrent show workspace)"
                export DATABASE_URL="postgres://postgres:postgres@$WORKSPACE.postgres.test:5432/cookit_dev"
                export BASE_DOMAIN="$WORKSPACE.test"
              fi

              # Pin Playwright to the nix-provided browsers so e2e never
              # downloads them.
              export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
              export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
              export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true

              # A shared `CARGO_TARGET_DIR` does not play nicely with worktrees;
              # see https://github.com/rust-lang/cargo/issues/12516
              unset CARGO_TARGET_DIR

              # Configure sccache.
              export RUSTC_WRAPPER=sccache
              export SCCACHE_DIR="$HOME/.cache/sccache"
            '';
          };
        }
      );
    in
    systemOutputs
    // {
      nixosModules.default =
        { lib, pkgs, ... }:
        {
          services.cookit.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
    };
}
