{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { nixpkgs
    , utils
    , rust-overlay
    , ...
    }: utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
      };

      rust = {
        stable = pkgs.rust-bin.stable."1.82.0";
        nightly = pkgs.rust-bin.nightly.latest;
      };

      mkRustToolchain = components: rust.stable.minimal.override {
        extensions = components;
      };
    in
    {
      devShells =
        let
          commonPackages = with pkgs; [
            just
            docker-client
            mycli
            sqlx-cli
            cargo-nextest
          ];
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = commonPackages ++ [
              (mkRustToolchain [
                "rust-src"
                "clippy"
                "rust-analyzer"
              ])

              rust.nightly.rustfmt
            ];
          };
        };
    });
}
