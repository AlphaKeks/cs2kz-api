{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.11";
    utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , utils
    , rust-overlay
    , crane
    , disko
    , ...
    }@inputs:
    let
      inherit (nixpkgs) lib;
      getPkgs = system: import nixpkgs {
        inherit system;
        overlays = [ (import inputs.rust-overlay) ];
      };
    in
    (utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
    (getPkgs system).callPackage ./nix/cs2kz-api.nix {
      inherit crane;
    })) // {
      nixosConfigurations = {
        production =
          let
            system = "aarch64-linux";
            pkgs = getPkgs system;
          in
          lib.nixosSystem {
            specialArgs = {
              inherit system disko;
              cs2kz-api = (pkgs.callPackage ./nix/cs2kz-api.nix {
                inherit crane;
              }).packages.cs2kz-api;
            };
            modules = [
              ./nix/NixOS/configuration.nix
              ./nix/NixOS/disks.nix
            ];
          };
      };
    };
}
