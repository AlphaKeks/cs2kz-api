{ lib, pkgs, crane, ... }:

let
  python = pkgs.python311.withPackages (p: with p; [
    scipy
  ]);

  rust-toolchain =
    pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml;

  craneLib = (crane.mkLib pkgs).overrideToolchain (p:
    ((p.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml).override {
      extensions = [ "clippy" "rustfmt" ];
    }));

  mkFileSet = files: lib.fileset.toSource {
    root = ./..;
    fileset = lib.fileset.unions (files ++ [
      (craneLib.fileset.commonCargoSources ./..)
      ../crates/cs2kz/migrations
      ../.sqlx
      ../.example.env
    ]);
  };

  fileSetForCrate = crate: mkFileSet [
    (craneLib.fileset.commonCargoSources crate)
  ];

  src = mkFileSet [ ];

  commonArgs = {
    inherit src;
    strictDeps = true;
    buildInputs = [ python ];
    env = {
      PYO3_PYTHON = "${python}/bin/python";
      SQLX_OFFLINE = true;
    };
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
  crateArgs = commonArgs // {
    inherit cargoArtifacts;
    inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
  };

  cs2kz-api = craneLib.buildPackage (crateArgs // {
    pname = "cs2kz-api";
    src = fileSetForCrate ../crates/cs2kz-api;
    cargoExtraArgs = "--bin=cs2kz-api";
  });

  openapi-schema = craneLib.buildPackage (crateArgs // {
    pname = "openapi";
    src = fileSetForCrate ../crates/cs2kz-api;
    cargoExtraArgs = "--bin=openapi";
  });
in

{
  checks = {
    inherit cs2kz-api openapi-schema;

    clippy = craneLib.cargoClippy (commonArgs // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--no-deps --all-targets -- -Dwarnings";
    });

    clippy-tests = craneLib.cargoClippy (commonArgs // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--no-deps --tests -- -Dwarnings";
    });

    rustfmt = craneLib.cargoFmt {
      inherit src;
    };
  };

  packages = {
    inherit cs2kz-api openapi-schema;

    dockerImage = pkgs.dockerTools.buildLayeredImage {
      name = cs2kz-api.pname;
      tag = cs2kz-api.version;
      config = {
        Cmd = [
          "${cs2kz-api}/bin/cs2kz-api"
          "--config"
          "/etc/cs2kz-api.toml"
          "--depot-downloader-path"
          "${pkgs.depotdownloader}/bin/DepotDownloader"
        ];
      };
    };
  };

  devShells.default = pkgs.mkShell {
    nativeBuildInputs = [ rust-toolchain python ] ++ (with pkgs; [
      docker-client
      lazydocker
      mycli
      sqlx-cli
      tokio-console
      depotdownloader
      oha
    ]);

    PYO3_PYTHON = "${python}/bin/python";
  };
}
