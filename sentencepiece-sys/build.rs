use cc::Build;

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

    if !find_sentencepiece(&mut builder) {
        build_sentencepiece(&mut builder);
    }

    builder
        .file("src/ffi/sentencepiece.cpp")
        .cpp(true)
        .flag_if_supported("-std=c++11")
        .compile("sentencepiece_wrap");

    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.cpp");
}
