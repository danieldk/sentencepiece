{ pkgs ? import (import ./nix/sources.nix).nixpkgs {} }:

let
  albertBase = pkgs.fetchurl {
    url = "https://s3.amazonaws.com/models.huggingface.co/bert/albert-base-v1-spiece.model";
    sha256 = "0dh35nh493bwiqw6yzcwp1mgca1lzgjjhbb04zzc5id6cyv05yzy";
  };
in with pkgs; rustPlatform.buildRustPackage {
  pname = "sentencepiece";
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;

  src = builtins.path {
    name = "sentencepiece";
    path = ./.;
  };

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [
    cmake
    pkg-config
  ];

  buildInputs = [
    sentencepiece
  ];

  cargoTestFlags = [
    "--features=albert-tests"
  ];

  # Model path for model tests.
  ALBERT_BASE_MODEL = "${albertBase}";
}
