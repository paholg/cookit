{
  inputs = {
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
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
          };

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

          # Keep migrations (sqlx::migrate!), .sqlx (offline data), and
          # assets (asset! macro) alongside the cargo sources.
          src =
            let
              extraFilter =
                path: _type:
                builtins.match ".*/(migrations|\\.sqlx|assets)(/.*)?" path != null
                || builtins.match ".*\\.(sql|css|js|png|svg|ico)$" path != null;
              cargoFilter = craneLib.filterCargoSources;
            in
            pkgs.lib.cleanSourceWith {
              src = ./.;
              filter = path: type: (extraFilter path type) || (cargoFilter path type);
              name = "source";
            };

          # Extract wasm-bindgen and dioxus-cli from Cargo.lock so we don't have
          # to worry about version drift.
          lockFile = builtins.fromTOML (builtins.readFile ./Cargo.lock);
          wasmBindgenVersion =
            (builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lockFile.package)).version;
          dioxusVersion = (builtins.head (builtins.filter (p: p.name == "dioxus") lockFile.package)).version;

          wasmBindgenCli = pkgs.rustPlatform.buildRustPackage rec {
            pname = "wasm-bindgen-cli";
            version = wasmBindgenVersion;
            src = pkgs.fetchCrate {
              inherit pname version;
              hash = "sha256-vO4RSxi/sMWxmsEs3GuljdMfIRSu75A+Q+c5wgYToRU=";
            };
            cargoHash = "sha256-Inup6vvJSG5ghNyeDPyZbfZo4d0LsMG2OJfStoaeDBs=";
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [
              pkgs.openssl
            ];
          };

          dioxusCli = pkgs.dioxus-cli.overrideAttrs (
            old:
            let
              src = pkgs.fetchCrate {
                pname = "dioxus-cli";
                version = dioxusVersion;
                hash = "sha256-tLMtUlohSJt3okdJh+ARweQNGmzj/vYiNl8iZhDbSAc=";
              };
            in
            {
              version = dioxusVersion;
              inherit src;
              # Regenerate the vendored deps from the new source rather than
              # overriding the old derivation, which drops the recursive
              # output-hash mode and fails with an "output path ... should be a
              # non-executable regular file" error.
              cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
                inherit src;
                name = "dioxus-cli-${dioxusVersion}-vendor";
                hash = "sha256-h5wkxHP8ehZLHqcUsro08/dpqSPnPuBbZuUGG8i4nBc=";
              };
            }
          );

          devPackages =
            with pkgs;
            [
              cargo-dist
              cargo-edit
              cargo-nextest
              diesel-cli
              just
              libpq
              litecli
              nodejs_latest
              openssl
              pkg-config
              rust-bin.nightly.latest.rustfmt
              sqlite
              sqlx-cli
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
            buildInputs = [ pkgs.openssl ];
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

        in
        {
          packages.default = package;
          checks = {
            inherit cookit-tests;
          };
          devShells.default = pkgs.mkShell {
            packages = devPackages;

            shellHook = ''
              export DATABASE_URL="postgres://postgres:postgres@$(devconcurrent show workspace).postgres.test:5432/cookit_dev"
              export DATABASE_TEST_URL="postgres://postgres:postgres@$(devconcurrent show workspace).postgres.test:5432/cookit_test"
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
          imports = [ ./nix/module.nix ];

          services.cookit.package = lib.mkDefault self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        };
    };
}
