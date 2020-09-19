fn main() {
    #[cfg(feature = "proto-compile")]
    protoc_rust::Codegen::new()
        .out_dir("src")
        .inputs(&["protos/sentencepiece.proto"])
        .include("protos")
        .run()
        .expect("protoc");
}
