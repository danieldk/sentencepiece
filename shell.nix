# We pin nixpkgs to improve reproducability. We don't pin Rust to a
# specific version, but use the latest stable release.

let
  sources = import ./nix/sources.nix;
  nixpkgs = import sources.nixpkgs {};
  danieldk = nixpkgs.callPackage sources.danieldk {};
in with nixpkgs; mkShell {
  nativeBuildInputs = with nixpkgs; [
    cargo
    clang
    clippy
    pkgconfig
    protobuf
    rust-bindgen
  ];

  buildInputs = with nixpkgs; [
    sentencepiece
  ];

  LIBCLANG_PATH = with nixpkgs.llvmPackages; "${libclang}/lib";
}
