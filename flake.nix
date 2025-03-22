{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, crane, ... }:
    let inherit (nixpkgs) lib; in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rust-toolchain =
          pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        python = pkgs.python312.withPackages (p: with p; [
          scipy
        ]);

        craneLib = (crane.mkLib pkgs).overrideToolchain (p: (
          (p.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml).override {
            extensions = [ "clippy" "rustfmt" ];
          }
        ));

        src = lib.cleanSourceWith {
          src = ./.;
          name = "source";
          filter = path: type: (craneLib.filterCargoSources path type)
            || (builtins.any (pattern: ((builtins.match pattern path) != null)) [
            ".*README\.md$"
            ".*/migrations/.*\.sql$"
            ".*/\.sqlx/query-.*\.json$"
          ]);
        };

        commonArgs = {
          inherit src;
          strictDeps = true;
          env = {
            SQLX_OFFLINE = true;
            PYO3_PYTHON = lib.getExe python;
          };
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        cs2kz-api = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { src = ./.; }) version;
          pname = "cs2kz-api";
          nativeBuildInputs = [ pkgs.makeWrapper ];
          preFixup = ''
            wrapProgram $out/bin/cs2kz-api \
              --prefix PATH : ${python}/bin
          '';
        });
      in
      {
        formatter = pkgs.treefmt;
        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            (with pkgs; [ treefmt nixpkgs-fmt shfmt taplo ])
            ++ [ rust-toolchain pkgs.sqlx-cli pkgs.docker-client ]
            ++ [ python pkgs.depotdownloader ];

          PYO3_PYTHON = lib.getExe python;
        };
        checks = {
          inherit cs2kz-api;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--no-deps --workspace --all-features --all-targets -- -Dwarnings";
          });

          clippy-tests = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--no-deps --workspace --all-features --tests -- -Dwarnings";
          });

          rustfmt = craneLib.cargoFmt {
            inherit src;
          };
        };
        packages = {
          inherit cs2kz-api;

          default = cs2kz-api;

          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = cs2kz-api.pname;
            tag = cs2kz-api.version;
            config = {
              Cmd = [
                "${cs2kz-api}/bin/cs2kz-api"
                "--config"
                "/etc/cs2kz-api.toml"
                "--depot-downloader-path"
                "${lib.getExe pkgs.depotdownloader}"
              ];
            };
          };
        };
      });
}
