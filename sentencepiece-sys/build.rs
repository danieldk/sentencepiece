fn main() {
    let lib = pkg_config::Config::new().probe("sentencepiece").unwrap();

    let mut builder = cc::Build::new();

    for i in &lib.include_paths {
        builder.include(i);
    }

    builder
        .file("src/ffi/sentencepiece.cpp")
        .cpp(true)
        .flag_if_supported("-std=c++11")
        .compile("sentencepiece_wrap");

    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.cpp");
}
