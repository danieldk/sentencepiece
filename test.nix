{ pkgs ? import (import ./nix/sources.nix).nixpkgs {} }:

let
  sources = import ./nix/sources.nix;
  danieldk = pkgs.callPackage sources.danieldk {};
  albertBase = builtins.fetchurl {
    url = "https://s3.amazonaws.com/models.huggingface.co/bert/albert-base-spiece.model";
    sha256 = "0dh35nh493bwiqw6yzcwp1mgca1lzgjjhbb04zzc5id6cyv05yzy";
  };
  crateOverrides = with pkgs; defaultCrateOverrides // {
    sentencepiece = attrs: {
      # Model path for model tests.
      ALBERT_BASE_MODEL = "${albertBase}";

      # Work around incorrect library ordering.
      EXTRA_RUSTC_FLAGS = [ "-lsentencepiece" ];
    };

    sentencepiece-sys = attrs: {
      nativeBuildInputs = [
        pkgconfig
      ];

      buildInputs = [
        clang
        sentencepiece
      ];

      LIBCLANG_PATH = with llvmPackages; "${libclang}/lib";
    };
  };
  buildRustCrate = pkgs.buildRustCrate.override { defaultCrateOverrides = crateOverrides; };
  cargo_nix = pkgs.callPackage ./nix/Cargo.nix { inherit buildRustCrate; };
in cargo_nix.rootCrate.build.override {
  features = [ "model-tests" ];
  runTests = true;
}
