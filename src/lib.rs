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
use std::path::Path;
use std::slice;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use thiserror::Error;

use sentencepiece_sys::{
    spp_bos_id, spp_encode_as_serialized_proto, spp_eos_id, spp_free, spp_from_serialized_proto,
    spp_is_unknown, spp_load, spp_new, spp_piece_to_id, spp_unknown_id,
    SentencePieceProcessor as CSentencePieceProcessor,
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

    #[error("Encoded text did not contain {0}")]
    MissingData(String),
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

    /// Open a sentencepiece model.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SentencePieceError> {
        let spp = SentencePieceProcessor {
            inner: unsafe { spp_new() },
        };

        // Note: `as_bytes` is not available on Windows. If we port to Windows, check
        // what the expectations of sentencepiece are.
        let c_filename = CString::new(path.as_ref().as_os_str().as_bytes()).unwrap();
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

    /// Tokenizer a sentence.
    pub fn encode(&self, sentence: &str) -> Result<Vec<PieceWithId>, SentencePieceError> {
        let mut len = 0u64;
        let c_proto = unsafe {
            spp_encode_as_serialized_proto(
                self.inner,
                sentence.as_bytes().as_ptr() as *const i8,
                sentence.as_bytes().len() as u64,
                &mut len,
            )
        };
        let c_proto = CData { data: c_proto, len };

        // Errors are communicated as empty data.
        if len == 0 {
            return Err(SentencePieceError::EncodeError);
        }

        let proto: Vec<u8> = c_proto.to_owned();
        let sp_text: SentencePieceText = prost::Message::decode(proto.as_slice())
            .expect("Received invalid protobuf from sentencepiece");

        // Most fields in the sentencepiece protobuf are optionals. Let's be
        // defensive about absent fields for a piece.
        Ok(sp_text
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
            .collect::<Result<_, _>>()?)
    }

    pub fn eos_id(&self) -> Option<u32> {
        let eos_id = unsafe { spp_eos_id(self.inner) };
        if eos_id < 0 {
            None
        } else {
            Some(eos_id as u32)
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

    pub fn unknown_id(&self) -> u32 {
        unsafe { spp_unknown_id(self.inner) as u32 }
    }
}

// sentencepiece is thread-safe:
// https://github.com/google/sentencepiece/issues/207

unsafe impl Send for SentencePieceProcessor {}

unsafe impl Sync for SentencePieceProcessor {}

#[cfg(test)]
mod tests {
    use crate::{CSentencePieceError, PieceWithId, SentencePieceError, SentencePieceProcessor};

    fn toy_model() -> Result<SentencePieceProcessor, SentencePieceError> {
        let model_data = include_bytes!("../testdata/toy.model");
        SentencePieceProcessor::from_serialized_proto(model_data)
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
    fn can_lookup_unknown_id() {
        let toy_model = toy_model().unwrap();
        assert_eq!(toy_model.unknown_id(), 0);
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
    fn can_lookup_unknown_id() {
        let albert_model = albert_model().unwrap();
        assert_eq!(albert_model.unknown_id(), 1);
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
