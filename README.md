# sentencepiece

This Rust crate is a binding for the
[sentencepiece](https://github.com/google/sentencepiece) unsupervised
text tokenizer. The [crate
documentation](https://docs.rs/sentencepiece/) is available
online.

## `libsentencepiece` dependency

This crate depends on the `sentencepiece` C++ library. By default,
this dependency is treated as follows:

* If `sentencepiece` could be found with `pkg-config`, the crate will
  link against the library found through `pkg-config`. **Warning:**
  dynamic linking only works correctly with sentencepiece 0.1.95
  or later, due to
  [a bug in earlier versions](https://github.com/google/sentencepiece/issues/579).
* Otherwise, the crate's build script will do a static build of the
  `sentencepiece` library. This requires that `cmake` is available.

If you wish to override this behavior, the `sentencepiece-sys` crate
offers two features:

* `system`: always attempt to link to the `sentencepiece` library
  found with `pkg-config`.
* `static`: always do a static build of the `sentencepiece` library
  and link against that.
