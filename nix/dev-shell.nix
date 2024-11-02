{ pkgs, rust-nightly, mkToolchain, ... }:

pkgs.mkShell {
  nativeBuildInputs = [
    (mkToolchain [ "rust-src" "clippy" "rust-analyzer" ])
    rust-nightly.rustfmt
  ] ++ (with pkgs; [
    just
    cargo-nextest
    cargo-expand
    docker-client
    mariadb_110
    sqlx-cli
    tokio-console
    depotdownloader
    oha
    (python3.withPackages (p: [ p.scipy ]))
  ]);

  shellHook = ''
    export IN_DEV_SHELL=1
    export CARGO_NIGHTLY="${rust-nightly.cargo}/bin/cargo"
  '';
}
