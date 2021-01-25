{ pkgs ? import (import ./nix/sources.nix).nixpkgs {} }:

let
  sources = import ./nix/sources.nix;
  danieldk = pkgs.callPackage sources.danieldk {};
  albertBase = builtins.fetchurl {
    url = "https://s3.amazonaws.com/models.huggingface.co/bert/albert-base-v1-spiece.model";
    sha256 = "0dh35nh493bwiqw6yzcwp1mgca1lzgjjhbb04zzc5id6cyv05yzy";
  };
  crateOverrides = with pkgs; defaultCrateOverrides // {
    sentencepiece = attrs: {
      buildInputs = [ sentencepiece ];

      # Model path for model tests.
      ALBERT_BASE_MODEL = "${albertBase}";

      # Work around incorrect library ordering.
      EXTRA_RUSTC_FLAGS = [ "-lsentencepiece" ];
    };

    sentencepiece-sys = attrs: {
      features = [ "system" ];

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
  buildRustCrate = pkgs.buildRustCrate.override {
    defaultCrateOverrides = crateOverrides;
  };
  crateTools = import "${sources.crate2nix}/tools.nix" {};
  cargoNix = pkgs.callPackage (crateTools.generatedCargoNix {
    name = "sentencepiece";
    src = pkgs.nix-gitignore.gitignoreSource [ ".git/" "nix/" "*.nix" ] ./.;
  }) {
    inherit buildRustCrate;
  };
in cargoNix.rootCrate.build.override {
  features = [ "albert-tests" ];
  runTests = true;
}
