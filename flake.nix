{
  inputs =
    {
      nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
      utils.url = "github:numtide/flake-utils";
      rust-overlay.url = "github:oxalica/rust-overlay";
      crane.url = "github:ipetkov/crane";
    };

  outputs =
    { nixpkgs
    , utils
    , rust-overlay
    , crane
    , ...
    }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rust =
        {
          stable = pkgs.rust-bin.stable."1.81.0";
          nightly = pkgs.rust-bin.nightly."2024-08-24";
        };

      mkToolchain = components: rust.stable.minimal.override {
        extensions = components;
      };

      craneLib = (crane.mkLib pkgs).overrideToolchain (mkToolchain [ "rust-src" ]);

      src = pkgs.lib.cleanSourceWith {
        src = ./.;
        name = "source";
        filter = path: type: (craneLib.filterCargoSources path type)
          || (builtins.any (pattern: ((builtins.match pattern path) != null)) [
          # required by sqlx macros
          ".*sqlx/query-.*json"
          ".*database/migrations/.*sql"
          ".*database/fixtures/.*sql"
        ]);
      };

      cargoArtifacts = craneLib.buildDepsOnly {
        inherit src;
        pname = "workspace";
        version = "0.0.0";
        nativeBuildInputs = [ (mkToolchain [ "rust-src" ]) ];
      };

      mkCrate = crate:
        let
          inherit (craneLib.crateNameFromCargoToml {
            cargoToml = ./crates/${crate}/Cargo.toml;
          }) pname version;
        in
        {
          inherit src cargoArtifacts pname version;

          cargoExtraArgs = "--package=${pname}";
          nativeBuildInputs = [ (mkToolchain [ "rust-src" "clippy" ]) ];
        };

      api = craneLib.buildPackage ((mkCrate "cs2kz-api") // {
        # A lot of tests require a running database, which isn't available
        # during nix builds.
        # These tests run in CI anyway, so it isn't critical for them to run
        # when building with nix as well.
        doCheck = false;

        # Force using the query cache instead of a live database.
        SQLX_OFFLINE = "1";
      });

      cli = craneLib.buildPackage (mkCrate "cs2kz-api-cli");

      commonShellPackages = with pkgs; [
        rust.nightly.rustfmt
        just
        cargo-nextest
        sqlx-cli
        tokio-console
        depotdownloader
        docker-client
        mariadb_110
      ];
    in
    {
      packages =
        {
          default = api;
          inherit api cli;
        };

      devShells =
        {
          default = pkgs.mkShell {
            nativeBuildInputs = commonShellPackages ++ (with pkgs; [
              (mkToolchain [ "rust-src" "clippy" "rust-analyzer" ])
            ]);
          };

          miri = pkgs.mkShell {
            nativeBuildInputs = commonShellPackages ++ (with pkgs; [
              (rust.nightly.minimal.override {
                extensions = [ "rust-src" "clippy" "rust-analyzer" "miri" ];
              })
            ]);
          };
        };
    });
}
