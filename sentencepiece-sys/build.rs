use std::env;

use cc::Build;

macro_rules! feature(($name:expr) => (env::var(concat!("CARGO_FEATURE_", $name)).is_ok()));

fn build_sentencepiece(builder: &mut Build) {
    let dst = cmake::build("source");
    println!(
        "cargo:rustc-link-search=native={}",
        dst.join("build").join("src").display()
    );
    println!("cargo:rustc-link-lib=static=sentencepiece");

    builder.include("source/src");
}

fn find_sentencepiece(builder: &mut Build) -> bool {
    let lib = match pkg_config::Config::new().probe("sentencepiece") {
        Ok(lib) => lib,
        Err(_) => return false,
    };

    // Add include paths
    for i in &lib.include_paths {
        builder.include(i);
    }

    true
}

fn main() {
    let mut builder = Build::new();

    if feature!("SYSTEM") {
        find_sentencepiece(&mut builder);
    } else if feature!("STATIC") || !find_sentencepiece(&mut builder) {
        build_sentencepiece(&mut builder);
    }

    builder.file("src/ffi/sentencepiece.cpp").cpp(true);

    if builder.get_compiler().is_like_msvc() {
        builder.flag("/std:c++17");
    } else {
        builder.flag("-std=c++17");
    }

    builder.compile("sentencepiece_wrap");

    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.cpp");
}
