{
  inputs = {
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

          # Extract wasm-bindgen version from Cargo.lock so we don't need to
          # keep nipkgs and Cargo.lock exactly in sync, even for dependents.
          wasmBindgenVersion =
            let
              lockFile = builtins.fromTOML (builtins.readFile ./Cargo.lock);
              wasmBindgen = builtins.head (builtins.filter (p: p.name == "wasm-bindgen") lockFile.package);
            in
            wasmBindgen.version;

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

          devPackages =
            with pkgs;
            [
              cargo-dist
              cargo-edit
              cargo-nextest
              just
              pkg-config
              openssl
              sqlx-cli
              tombi
            ]
            ++ [ rustDev ];

          package = pkgs.rustPlatform.buildRustPackage {
            pname = "cookit";
            version = "0.1.0";
            src = ./.;
            strictDeps = true;
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.dioxus-cli
              wasmBindgenCli
              pkgs.binaryen
              rustMinimal
            ];
            buildInputs = [ pkgs.openssl ];
            SQLX_OFFLINE = "true";
            buildPhase = ''
              export HOME=$(mktemp -d)
              dx build --release --platform web --package web
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp target/dx/web/release/web/web $out/bin/
              cp -r target/dx/web/release/web/public $out/bin/
            '';
            cargoLock.lockFile = ./Cargo.lock;
          };

        in
        {
          packages.default = package;
          devShells.default = pkgs.mkShell {
            env = {
              # TODO: Remove all this.
              DATABASE_URL = "sqlite:///home/paho/src/cookit/dev/cookit.db";
              OIDC_ISSUER_URL = "http://localhost:8090/admin";
              OIDC_CLIENT_ID = "cookit";
              OIDC_CLIENT_SECRET = "anything";
              OIDC_REDIRECT_URL = "http://localhost:8080/auth/callback";
              OIDC_INSECURE_TLS = "true";
              SESSION_SECRET = "ZGV2LXNlc3Npb24tc2VjcmV0LWRldi1zZXNzaW9uLXNlY3JldC1kZXYtc2Vzc2lvbi1zZWNyZXQtZGV2LXNlc3Npb24tc2VjcmV0LWQK";
              SESSION_COOKIE_SECURE = "false";
            };
            packages = devPackages;
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
