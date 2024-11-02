{ pkgs, crane, ... }:

let
  rust-stable = pkgs.rust-bin.stable."1.82.0";
  rust-nightly = pkgs.rust-bin.nightly."2024-11-01";

  mkToolchain = extensions: rust-stable.minimal.override {
    inherit extensions;
  };

  craneLib = (crane.mkLib pkgs).overrideToolchain (mkToolchain [ "rust-src" ]);

in
{
  inherit rust-stable rust-nightly mkToolchain craneLib;
}
