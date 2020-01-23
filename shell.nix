# We pin nixpkgs to improve reproducability. We don't pin Rust to a
# specific version, but use the latest stable release.

let
  sources = import ./nix/sources.nix;
  nixpkgs = import sources.nixpkgs {};
  danieldk = nixpkgs.callPackage sources.danieldk {};
  mozilla = nixpkgs.callPackage "${sources.mozilla}/package-set.nix" {};
in with nixpkgs; mkShell {
  nativeBuildInputs = with nixpkgs; [
    clang
    mozilla.latest.rustChannels.stable.rust
    pkgconfig
  ];

  buildInputs = with nixpkgs; [
    sentencepiece
  ];

  LIBCLANG_PATH = with nixpkgs.llvmPackages; "${libclang}/lib";
}
