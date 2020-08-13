# sentencepiece

This Rust crate is a binding for the
[sentencepiece](https://github.com/google/sentencepiece) unsupervised
text tokenizer.

Building the crate requires:

* A Rust toolchain;
* either:
  * CMake to build sentencepiece as part of this crate; or
  * `libsentencepiece` and `pkg-config` to link against an existing
    sentencepiece library.
	
## Linking against an existing sentencepiece library

To link against an existing sentencepiece library, the library should
be discoverable through `pkg-config`. Then the
`sentencepiece-sys/system` feature should be user to activate linking
against an external library.

## Documentation

The [crate documentation](https://rustdoc.danieldk.eu/sentencepiece)
is available online.
