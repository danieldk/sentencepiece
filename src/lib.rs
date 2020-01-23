use std::ffi::{c_void, CString};
use std::ops::{Deref, Drop};
use std::slice;

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use sentencepiece_sys::{
    spp_encode_as_serialized_proto, spp_free, spp_load, spp_new,
    SentencePieceProcessor as CSentencePieceProcessor,
};

mod sentencepiece;
use crate::sentencepiece::SentencePieceText;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PieceWithId {
    piece: String,
    id: u32,
    span: (u32, u32),
}

#[derive(Clone, Copy, Debug, Eq, FromPrimitive, PartialEq)]
pub enum SentencePieceError {
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    Unauthenticated = 16,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
}

/// Small wrapper struct to deallocate data automatically.
struct CData {
    data: *const u8,
    len: usize,
}

impl Deref for CData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.data, self.len) }
    }
}

impl Drop for CData {
    fn drop(&mut self) {
        unsafe { libc::free(self.data as *mut c_void) }
    }
}

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
    pub fn load(filename: &str) -> Result<Self, SentencePieceError> {
        let spp = SentencePieceProcessor {
            inner: unsafe { spp_new() },
        };

        let c_filename = CString::new(filename).unwrap();
        let result = unsafe { spp_load(spp.inner, c_filename.as_ptr()) };
        if result == 0 {
            Ok(spp)
        } else {
            Err(match FromPrimitive::from_i32(result as i32) {
                Some(error) => error,
                None => unreachable!(),
            })
        }
    }

    pub fn encode(&self, sentence: &str) -> Result<Vec<PieceWithId>, SentencePieceError> {
        let c_sentence = CString::new(sentence).unwrap();

        let mut len = 0usize;
        let c_proto =
            unsafe { spp_encode_as_serialized_proto(self.inner, c_sentence.as_ptr(), &mut len) };
        let c_proto = CData { data: c_proto, len };

        // Errors are communicated as empty data.
        if len == 0 {
            return Err(SentencePieceError::Internal);
        }

        let proto: Vec<u8> = c_proto.to_owned();
        let proto_pieces = protobuf::parse_from_bytes::<SentencePieceText>(&proto)
            .expect("Received invalid protobuf from sentencepiece");

        Ok(proto_pieces
            .get_pieces()
            .iter()
            .map(|proto_piece| PieceWithId {
                piece: proto_piece.get_piece().to_owned(),
                id: proto_piece.get_id(),
                span: (proto_piece.get_begin(), proto_piece.get_end()),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::{PieceWithId, SentencePieceError, SentencePieceProcessor};

    fn roberta_model() -> SentencePieceProcessor {
        SentencePieceProcessor::load("testdata/xlm-roberta-base-sentencepiece.bpe.model").unwrap()
    }

    #[test]
    fn encodes_sentence() {
        let model = roberta_model();
        assert_eq!(
            model.encode("Veruntreute die AWO Spendengeld?").unwrap(),
            vec![
                PieceWithId {
                    piece: "▁V".to_owned(),
                    id: 309,
                    span: (0, 1)
                },
                PieceWithId {
                    piece: "erunt".to_owned(),
                    id: 23450,
                    span: (1, 6)
                },
                PieceWithId {
                    piece: "re".to_owned(),
                    id: 106,
                    span: (6, 8)
                },
                PieceWithId {
                    piece: "ute".to_owned(),
                    id: 6742,
                    span: (8, 11)
                },
                PieceWithId {
                    piece: "▁die".to_owned(),
                    id: 67,
                    span: (11, 15)
                },
                PieceWithId {
                    piece: "▁A".to_owned(),
                    id: 61,
                    span: (15, 17)
                },
                PieceWithId {
                    piece: "WO".to_owned(),
                    id: 43788,
                    span: (17, 19)
                },
                PieceWithId {
                    piece: "▁Spenden".to_owned(),
                    id: 207125,
                    span: (19, 27)
                },
                PieceWithId {
                    piece: "geld".to_owned(),
                    id: 49003,
                    span: (27, 31)
                },
                PieceWithId {
                    piece: "?".to_owned(),
                    id: 31,
                    span: (31, 32)
                }
            ]
        );
    }

    #[test]
    fn loads_model() {
        assert!(
            SentencePieceProcessor::load("testdata/xlm-roberta-base-sentencepiece.bpe.model")
                .is_ok()
        );
    }

    #[test]
    fn fails_loading_nonexisting_model() {
        assert_eq!(
            SentencePieceProcessor::load("non-existing").unwrap_err(),
            SentencePieceError::NotFound
        );
    }
}
