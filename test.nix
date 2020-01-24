{ pkgs ? import (import ./nix/sources.nix).nixpkgs {} }:

let
  sources = import ./nix/sources.nix;
  danieldk = pkgs.callPackage sources.danieldk {};
  crateOverrides = with pkgs; defaultCrateOverrides // {
    sentencepiece-sys = attrs: {
      nativeBuildInputs = [
        clang
        pkgconfig
      ];

      buildInputs = [ sentencepiece ];

      LIBCLANG_PATH = with llvmPackages; "${libclang}/lib";
    };
  };
  buildRustCrate = pkgs.buildRustCrate.override { defaultCrateOverrides = crateOverrides; };
  cargo_nix = pkgs.callPackage ./nix/Cargo.nix { inherit buildRustCrate; };
in cargo_nix.rootCrate.build.override { runTests = true; }
