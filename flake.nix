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

      rust = rec {
        packages = pkgs.rust-bin.stable."1.82.0";
        mkToolchain = components: packages.minimal.override {
          extensions = components;
        };
      };
    in
    {
      devShells =
        let
          shellPackages = with pkgs; [
            just
            docker-client
            mycli
            sqlx-cli
          ];
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = shellPackages ++ [
              (rust.mkToolchain [
                "rust-src"
                "clippy"
                "rust-analyzer"
              ])

              pkgs.rust-bin.nightly.latest.rustfmt
            ];
          };
        };
    });
}
