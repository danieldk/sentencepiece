fn main() {
    let lib = pkg_config::Config::new().probe("sentencepiece").unwrap();

    let mut builder = cc::Build::new();

    for i in &lib.include_paths {
        builder.include(i);
    }

    builder
        .file("src/ffi/sentencepiece.cpp")
        .cpp(true)
        .compile("sentencepiece_wrap");

    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.cpp");
}
