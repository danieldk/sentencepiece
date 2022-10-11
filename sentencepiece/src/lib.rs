//! This crate binds the
//! [sentencepiece](https://github.com/google/sentencepiece)
//! library. sentencepiece is an unsupervised text tokenizer.
//!
//! The main data structure of this crate is `SentencePieceProcessor`,
//! which is used to tokenize sentences:
//!
//! ```
//! use sentencepiece::SentencePieceProcessor;
//!
//! let spp = SentencePieceProcessor::open("testdata/toy.model").unwrap();
//! let pieces = spp.encode("I saw a girl with a telescope.").unwrap()
//!   .into_iter().map(|p| p.piece).collect::<Vec<_>>();
//! assert_eq!(pieces, vec!["▁I", "▁saw", "▁a", "▁girl", "▁with",
//!   "▁a", "▁t", "el", "es", "c", "o", "pe", "."]);
//! ```

use std::ffi::{c_void, CString, NulError};
use std::ops::{Deref, Drop};
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::slice;

use num_derive::FromPrimitive;
use num_traits::{FromPrimitive, Signed};
use thiserror::Error;

use sentencepiece_sys::{
    size_t, spp_bos_id, spp_decode_piece_ids, spp_decode_pieces, spp_encode_as_serialized_proto,
    spp_eos_id, spp_free, spp_from_serialized_proto, spp_is_unknown, spp_load, spp_new, spp_pad_id,
    spp_piece_size, spp_piece_to_id, spp_sample_encode_as_serialized_proto,
    spp_to_serialized_proto, spp_unk_id, SentencePieceProcessor as CSentencePieceProcessor,
};

mod sentencepiece;
use crate::sentencepiece::SentencePieceText;

/// Sentence piece with its identifier and string span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PieceWithId {
    /// The sentence piece as a string.
    pub piece: String,

    /// The vocabulary identifier of the sentence piece.
    pub id: u32,

    /// The span of the sentence piece in the tokenized string.
    ///
    /// The span is encoded as the byte offsets *[begin, end)*.
    pub span: (u32, u32),
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum SentencePieceError {
    #[error("sentencepiece error: {0}")]
    CError(CSentencePieceError),

    #[error("sentencepiece could not encode the text")]
    EncodeError,

    #[error("Filename contains nul: {0}")]
    FilenameContainsNul(PathBuf),

    #[error("Encoded text did not contain {0}")]
    MissingData(String),

    #[error("Piece contains nul byte")]
    PieceContainsNul,
}

/// Errors that returned by the `sentencepiece` library.
#[derive(Clone, Copy, Debug, Eq, Error, FromPrimitive, PartialEq)]
#[non_exhaustive]
pub enum CSentencePieceError {
    #[error("Cancelled")]
    Cancelled = 1,
    #[error("Unknown")]
    Unknown = 2,
    #[error("Invalid argument")]
    InvalidArgument = 3,
    #[error("Deadline exceeded")]
    DeadlineExceeded = 4,
    #[error("Not found")]
    NotFound = 5,
    #[error("Already exists")]
    AlreadyExists = 6,
    #[error("Permission denied")]
    PermissionDenied = 7,
    #[error("Unauthenticated")]
    Unauthenticated = 16,
    #[error("Resource exhausted")]
    ResourceExhausted = 8,
    #[error("Failed precondition")]
    FailedPrecondition = 9,
    #[error("Aborted")]
    Aborted = 10,
    #[error("Out of range")]
    OutOfRange = 11,
    #[error("Unimplemented")]
    Unimplemented = 12,
    #[error("Internal error")]
    Internal = 13,
    #[error("Unavailable")]
    Unavailable = 14,
    #[error("Data loss")]
    DataLoss = 15,
}

/// Small wrapper struct to deallocate data automatically.
struct CData {
    data: *const u8,
    len: u64,
}

impl Deref for CData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.data, self.len as usize) }
    }
}

impl Drop for CData {
    fn drop(&mut self) {
        unsafe { libc::free(self.data as *mut c_void) }
    }
}

/// Sentence piece tokenizer.
///
/// Instances of `SentencePieceProcessor` can be used to tokenizer a
/// sentence using a sentencepiece model.
#[derive(Debug)]
pub struct SentencePieceProcessor {
    inner: *mut CSentencePieceProcessor,
}

impl Drop for SentencePieceProcessor {
    fn drop(&mut self) {
        unsafe { spp_free(self.inner) }
    }
}

impl SentencePieceProcessor {
    pub fn from_serialized_proto(data: &[u8]) -> Result<Self, SentencePieceError> {
        let spp = SentencePieceProcessor {
            inner: unsafe { spp_new() },
        };

        let result = unsafe {
            spp_from_serialized_proto(spp.inner, data.as_ptr() as *const c_char, data.len() as u64)
        };

        if result == 0 {
            Ok(spp)
        } else {
            let c_error = match FromPrimitive::from_i32(result as i32) {
                Some(error) => error,
                None => unreachable!(),
            };
            Err(SentencePieceError::CError(c_error))
        }
    }

    /// Serialize the model to protobuf.
    pub fn to_serialized_proto(&self) -> Vec<u8> {
        let mut len = 0;
        let data = unsafe { spp_to_serialized_proto(self.inner, &mut len) };

        let c_str = CData { data, len };

        c_str.to_owned()
    }

    /// Open a sentencepiece model.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SentencePieceError> {
        let spp = SentencePieceProcessor {
            inner: unsafe { spp_new() },
        };

        // Note: `as_bytes` is not available on Windows. If we port to Windows, check
        // what the expectations of sentencepiece are.
        let c_filename = CString::new(path.as_ref().as_os_str().as_bytes())
            .map_err(|_| SentencePieceError::FilenameContainsNul(path.as_ref().to_owned()))?;

        let result = unsafe { spp_load(spp.inner, c_filename.as_ptr()) };
        if result == 0 {
            Ok(spp)
        } else {
            let c_error = match FromPrimitive::from_i32(result as i32) {
                Some(error) => error,
                None => unreachable!(),
            };
            Err(SentencePieceError::CError(c_error))
        }
    }

    pub fn bos_id(&self) -> Option<u32> {
        let bos_id = unsafe { spp_bos_id(self.inner) };
        if bos_id < 0 {
            None
        } else {
            Some(bos_id as u32)
        }
    }

    /// Decode a sentence from piece identifiers.
    pub fn decode_piece_ids(&self, pieces: &[u32]) -> Result<String, SentencePieceError> {
        let mut decoded = std::ptr::null_mut::<u8>();
        let mut decoded_len: size_t = 0;

        let status = unsafe {
            spp_decode_piece_ids(
                self.inner,
                pieces.as_ptr(),
                pieces.len() as size_t,
                &mut decoded,
                &mut decoded_len,
            )
        };

        let c_str = CData {
            data: decoded,
            len: decoded_len,
        };

        if status == 0 {
            let decoded_string = String::from_utf8(c_str.to_owned())
                .expect("Decoded sentence is not UTF-8, please report this bug.");

            Ok(decoded_string)
        } else {
            let c_error = match FromPrimitive::from_i32(status as i32) {
                Some(error) => error,
                None => unreachable!(),
            };
            Err(SentencePieceError::CError(c_error))
        }
    }

    pub fn decode_pieces(&self, pieces: &[impl AsRef<str>]) -> Result<String, SentencePieceError> {
        let mut decoded = std::ptr::null_mut::<u8>();
        let mut decoded_len: size_t = 0;

        let owned_c_pieces = pieces
            .iter()
            .map(|piece| CString::new(piece.as_ref()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| SentencePieceError::PieceContainsNul)?;
        let c_pieces = owned_c_pieces
            .iter()
            .map(|piece| piece.as_ptr())
            .collect::<Vec<_>>();

        let status = unsafe {
            spp_decode_pieces(
                self.inner,
                c_pieces.as_ptr(),
                c_pieces.len() as size_t,
                &mut decoded,
                &mut decoded_len,
            )
        };

        let c_str = CData {
            data: decoded,
            len: decoded_len,
        };

        if status == 0 {
            let decoded_string = String::from_utf8(c_str.to_owned())
                .expect("Decoded sentence is not UTF-8, please report this bug.");

            Ok(decoded_string)
        } else {
            let c_error = match FromPrimitive::from_i32(status as i32) {
                Some(error) => error,
                None => unreachable!(),
            };
            Err(SentencePieceError::CError(c_error))
        }
    }

    /// Encode a sentence as sentence pieces and their identifiers.
    pub fn encode(&self, sentence: &str) -> Result<Vec<PieceWithId>, SentencePieceError> {
        let mut len = 0u64;
        let c_proto = unsafe {
            spp_encode_as_serialized_proto(
                self.inner,
                sentence.as_ptr() as *const c_char,
                sentence.as_bytes().len() as u64,
                &mut len,
            )
        };

        Self::process_encode_protobuf(CData { data: c_proto, len })
    }

    pub fn eos_id(&self) -> Option<u32> {
        let eos_id = unsafe { spp_eos_id(self.inner) };
        if eos_id < 0 {
            None
        } else {
            Some(eos_id as u32)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        let len = unsafe { spp_piece_size(self.inner) };
        assert!(len >= 0);
        len as usize
    }

    pub fn pad_id(&self) -> Option<u32> {
        let pad_id = unsafe { spp_pad_id(self.inner) };
        if pad_id < 0 {
            None
        } else {
            Some(pad_id as u32)
        }
    }

    /// Get the identifier of a sentence piece.
    pub fn piece_to_id(&self, piece: &str) -> Result<Option<u32>, NulError> {
        let c_piece = CString::new(piece.as_bytes())?;
        let id = unsafe { spp_piece_to_id(self.inner, c_piece.as_ptr()) };

        if unsafe { spp_is_unknown(self.inner, id) } {
            Ok(None)
        } else {
            Ok(Some(id as u32))
        }
    }

    fn process_encode_protobuf(c_proto: CData) -> Result<Vec<PieceWithId>, SentencePieceError> {
        // Errors are communicated as empty data.
        if c_proto.len() == 0 {
            return Err(SentencePieceError::EncodeError);
        }

        let proto: Vec<u8> = c_proto.to_owned();
        let sp_text: SentencePieceText = prost::Message::decode(proto.as_slice())
            .expect("Received invalid protobuf from sentencepiece");

        // Most fields in the sentencepiece protobuf are optionals. Let's be
        // defensive about absent fields for a piece.
        sp_text
            .pieces
            .into_iter()
            .map(|proto_piece| {
                Ok(PieceWithId {
                    piece: proto_piece
                        .piece
                        .ok_or_else(|| SentencePieceError::MissingData("piece".to_string()))?,
                    id: proto_piece
                        .id
                        .ok_or_else(|| SentencePieceError::MissingData("id".to_string()))?,
                    span: (
                        proto_piece
                            .begin
                            .ok_or_else(|| SentencePieceError::MissingData("begin".to_string()))?,
                        proto_piece
                            .end
                            .ok_or_else(|| SentencePieceError::MissingData("end".to_string()))?,
                    ),
                })
            })
            .collect::<Result<_, _>>()
    }

    /// Encode a sentence using sampling (subword regularization).
    ///
    /// Sample for the `n_best` segmentations, where alpha controls the
    /// smoothness of the distribution.
    ///
    /// This method panics when `n_best > 512` or when alpha is not a (normal)
    /// positive floating point number.
    pub fn sample_encode(
        &self,
        sentence: &str,
        n_best: usize,
        alpha: f32,
    ) -> Result<Vec<PieceWithId>, SentencePieceError> {
        assert!(n_best <= 512);
        assert!(alpha.is_normal() && alpha.is_positive());

        let mut len = 0u64;
        let c_proto = unsafe {
            spp_sample_encode_as_serialized_proto(
                self.inner,
                sentence.as_ptr() as *const c_char,
                sentence.as_bytes().len() as u64,
                &mut len,
                n_best as size_t,
                alpha,
            )
        };

        Self::process_encode_protobuf(CData { data: c_proto, len })
    }

    pub fn unk_id(&self) -> u32 {
        let unk_id = unsafe { spp_unk_id(self.inner) };
        // unk_id must always be present.
        assert!(unk_id >= 0);
        unk_id as u32
    }
}

// sentencepiece is thread-safe:
// https://github.com/google/sentencepiece/issues/207

unsafe impl Send for SentencePieceProcessor {}

unsafe impl Sync for SentencePieceProcessor {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{CSentencePieceError, PieceWithId, SentencePieceError, SentencePieceProcessor};

    fn toy_model_proto() -> &'static [u8] {
        include_bytes!("../testdata/toy.model")
    }

    fn toy_model() -> Result<SentencePieceProcessor, SentencePieceError> {
        SentencePieceProcessor::from_serialized_proto(toy_model_proto())
    }

    #[test]
    fn decodes_piece_ids_with_toy_model() {
        let model = toy_model().unwrap();
        let decoded = model
            .decode_piece_ids(&[8, 465, 10, 947, 41, 10, 170, 168, 110, 28, 20, 143, 4])
            .unwrap();
        assert_eq!(decoded, "I saw a girl with a telescope.");
    }

    #[test]
    fn decodes_pieces_with_toy_model() {
        let model = toy_model().unwrap();
        let pieces = vec![
            "▁I", "▁saw", "▁a", "▁girl", "▁with", "▁a", "▁t", "el", "es", "c", "o", "pe", ".",
        ];
        let decoded = model.decode_pieces(&pieces).unwrap();
        assert_eq!(decoded, "I saw a girl with a telescope.");
    }

    #[test]
    fn decode_with_incorrect_identifier_fails() {
        let model = toy_model().unwrap();
        assert_eq!(
            model.decode_piece_ids(&[8, 1000]),
            Err(SentencePieceError::CError(CSentencePieceError::OutOfRange))
        );
    }

    #[test]
    #[should_panic]
    fn decodes_pieces_with_null_fails() {
        let model = toy_model().unwrap();
        let pieces = vec!["▁I", "▁s\0aw"];
        model.decode_pieces(&pieces).unwrap();
    }

    #[test]
    fn encodes_sentence_with_toy_model() {
        let model = toy_model().unwrap();
        assert_eq!(
            model.encode("I saw a girl with a telescope.").unwrap(),
            vec![
                PieceWithId {
                    piece: "▁I".to_string(),
                    id: 8,
                    span: (0, 1)
                },
                PieceWithId {
                    piece: "▁saw".to_string(),
                    id: 465,
                    span: (1, 5)
                },
                PieceWithId {
                    piece: "▁a".to_string(),
                    id: 10,
                    span: (5, 7)
                },
                PieceWithId {
                    piece: "▁girl".to_string(),
                    id: 947,
                    span: (7, 12)
                },
                PieceWithId {
                    piece: "▁with".to_string(),
                    id: 41,
                    span: (12, 17)
                },
                PieceWithId {
                    piece: "▁a".to_string(),
                    id: 10,
                    span: (17, 19)
                },
                PieceWithId {
                    piece: "▁t".to_string(),
                    id: 170,
                    span: (19, 21)
                },
                PieceWithId {
                    piece: "el".to_string(),
                    id: 168,
                    span: (21, 23)
                },
                PieceWithId {
                    piece: "es".to_string(),
                    id: 110,
                    span: (23, 25)
                },
                PieceWithId {
                    piece: "c".to_string(),
                    id: 28,
                    span: (25, 26)
                },
                PieceWithId {
                    piece: "o".to_string(),
                    id: 20,
                    span: (26, 27)
                },
                PieceWithId {
                    piece: "pe".to_string(),
                    id: 143,
                    span: (27, 29)
                },
                PieceWithId {
                    piece: ".".to_string(),
                    id: 4,
                    span: (29, 30)
                }
            ]
        );
    }

    #[test]
    fn sample_encodes_sentence_with_toy_model() {
        let model = toy_model().unwrap();
        let pieces = model
            .sample_encode("I saw a girl with a telescope.", 10, 0.5)
            .unwrap();
        // Since sampling is randomized, we cannot check the output,
        // instead check that we can decode the result.
        let pieces = pieces.iter().map(|p| p.id).collect::<Vec<_>>();
        assert_eq!(
            model.decode_piece_ids(&pieces).unwrap(),
            "I saw a girl with a telescope."
        );
    }

    #[test]
    #[should_panic]
    fn sample_encode_with_incorrect_alpha_fails() {
        let model = toy_model().unwrap();
        model
            .sample_encode("I saw a girl with a telescope.", 10, 0.0)
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn sample_encode_with_incorrect_n_best_fails() {
        let model = toy_model().unwrap();
        model
            .sample_encode("I saw a girl with a telescope.", 513, 0.1)
            .unwrap();
    }

    #[test]
    fn errors_on_path_with_nul() {
        let test_path = Path::new("test\0path");
        assert_eq!(
            SentencePieceProcessor::open(test_path).unwrap_err(),
            SentencePieceError::FilenameContainsNul(test_path.to_owned())
        );
    }

    #[test]
    fn fails_loading_nonexisting_model() {
        assert_eq!(
            SentencePieceProcessor::open("non-existing").unwrap_err(),
            SentencePieceError::CError(CSentencePieceError::NotFound)
        );
    }

    #[test]
    fn handles_nul_character() {
        let model = toy_model().unwrap();
        assert_eq!(
            model.encode("Test\0 nul").unwrap(),
            vec![
                PieceWithId {
                    piece: "▁T".to_string(),
                    id: 239,
                    span: (0, 1)
                },
                PieceWithId {
                    piece: "est".to_string(),
                    id: 382,
                    span: (1, 4)
                },
                PieceWithId {
                    piece: "\u{0}".to_string(),
                    id: 0,
                    span: (4, 5)
                },
                PieceWithId {
                    piece: "▁".to_string(),
                    id: 7,
                    span: (5, 6)
                },
                PieceWithId {
                    piece: "n".to_string(),
                    id: 24,
                    span: (6, 7)
                },
                PieceWithId {
                    piece: "ul".to_string(),
                    id: 231,
                    span: (7, 9)
                }
            ]
        );
    }

    #[test]
    fn loads_model_from_serialized_protobuf() {
        assert!(toy_model().is_ok());
    }

    #[test]
    fn can_lookup_piece_id() {
        let toy_model = toy_model().unwrap();
        assert_eq!(toy_model.piece_to_id("pe"), Ok(Some(143)));
        assert_eq!(toy_model.piece_to_id("unknown"), Ok(None));
    }

    #[test]
    fn can_lookup_bos_id() {
        let toy_model = toy_model().unwrap();
        assert_eq!(toy_model.bos_id(), Some(1));
    }

    #[test]
    fn can_lookup_eos_id() {
        let toy_model = toy_model().unwrap();
        assert_eq!(toy_model.eos_id(), Some(2));
    }

    #[test]
    fn can_lookup_pad_id() {
        let toy_model = toy_model().unwrap();
        // Fixme: the toy model was trained without a padding index.
        assert_eq!(toy_model.pad_id(), None);
    }

    #[test]
    fn can_lookup_unk_id() {
        let toy_model = toy_model().unwrap();
        assert_eq!(toy_model.unk_id(), 0);
    }

    #[test]
    fn model_has_correct_len() {
        let model = toy_model().unwrap();
        assert_eq!(model.len(), 1000);
    }

    #[test]
    fn protobuf_roundtrip_is_identical() {
        let protobuf = toy_model_proto();
        let spp = SentencePieceProcessor::from_serialized_proto(protobuf).unwrap();
        let protobuf_roundtrip = spp.to_serialized_proto();
        assert_eq!(protobuf, protobuf_roundtrip);
    }
}

#[cfg(feature = "albert-tests")]
#[cfg(test)]
mod albert_tests {
    use crate::{PieceWithId, SentencePieceError, SentencePieceProcessor};

    fn albert_model() -> Result<SentencePieceProcessor, SentencePieceError> {
        let model_path = env!("ALBERT_BASE_MODEL");
        SentencePieceProcessor::open(model_path)
    }

    #[test]
    fn can_lookup_bos_id() {
        let albert_model = albert_model().unwrap();
        assert_eq!(albert_model.bos_id(), None);
    }

    #[test]
    fn can_lookup_eos_id() {
        let albert_model = albert_model().unwrap();
        assert_eq!(albert_model.eos_id(), None);
    }

    #[test]
    fn can_lookup_unk_id() {
        let albert_model = albert_model().unwrap();
        assert_eq!(albert_model.unk_id(), 1);
    }

    #[test]
    fn decodes_sentence_with_albert_model() {
        let model = albert_model().unwrap();
        let decoded = model
            .decode_piece_ids(&[
                13, 1, 1514, 102, 1276, 3066, 20, 25277, 21000, 16689, 18, 26, 3784, 9,
            ])
            .unwrap();
        assert_eq!(
            decoded,
            " ⁇ ardly anyone attempted to decipher hieroglyphs for decades."
        );
    }

    #[test]
    fn encodes_sentence_with_albert_model() {
        let model = albert_model().unwrap();
        assert_eq!(
            model
                .encode("Hardly anyone attempted to decipher hieroglyphs for decades.")
                .unwrap(),
            vec![
                PieceWithId {
                    piece: "▁".to_string(),
                    id: 13,
                    span: (0, 0)
                },
                PieceWithId {
                    piece: "H".to_string(),
                    id: 1,
                    span: (0, 1)
                },
                PieceWithId {
                    piece: "ard".to_string(),
                    id: 1514,
                    span: (1, 4)
                },
                PieceWithId {
                    piece: "ly".to_string(),
                    id: 102,
                    span: (4, 6)
                },
                PieceWithId {
                    piece: "▁anyone".to_string(),
                    id: 1276,
                    span: (6, 13)
                },
                PieceWithId {
                    piece: "▁attempted".to_string(),
                    id: 3066,
                    span: (13, 23)
                },
                PieceWithId {
                    piece: "▁to".to_string(),
                    id: 20,
                    span: (23, 26)
                },
                PieceWithId {
                    piece: "▁decipher".to_string(),
                    id: 25277,
                    span: (26, 35)
                },
                PieceWithId {
                    piece: "▁hiero".to_string(),
                    id: 21000,
                    span: (35, 41)
                },
                PieceWithId {
                    piece: "glyph".to_string(),
                    id: 16689,
                    span: (41, 46)
                },
                PieceWithId {
                    piece: "s".to_string(),
                    id: 18,
                    span: (46, 47)
                },
                PieceWithId {
                    piece: "▁for".to_string(),
                    id: 26,
                    span: (47, 51)
                },
                PieceWithId {
                    piece: "▁decades".to_string(),
                    id: 3784,
                    span: (51, 59)
                },
                PieceWithId {
                    piece: ".".to_string(),
                    id: 9,
                    span: (59, 60)
                }
            ]
        );
    }

    #[test]
    fn loads_model() {
        assert!(albert_model().is_ok());
    }
}
