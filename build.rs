fn main() {
    #[cfg(feature = "proto-compile")]
    protoc_rust::run(protoc_rust::Args {
        out_dir: "src",
        input: &["protos/sentencepiece.proto"],
        includes: &["protos"],
        customize: protoc_rust::Customize {
            ..Default::default()
        },
    })
    .expect("protoc");
}
