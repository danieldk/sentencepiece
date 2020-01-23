use std::env;
use std::path::PathBuf;

fn main() {
    let lib = pkg_config::Config::new().probe("sentencepiece").unwrap();

    let mut builder = cc::Build::new();

    for i in &lib.include_paths {
        builder.include(i);
    }

    builder
        .file("src/ffi/sentencepiece.cpp")
        .cpp(true)
        .compile("csentencepiece");

    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.h");
    println!("cargo:rerun-if-changed=src/ffi/sentencepiece.cpp");

    let bindings = bindgen::Builder::default()
        .header("src/ffi/sentencepiece.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
