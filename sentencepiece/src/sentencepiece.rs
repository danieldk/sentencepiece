use prost_derive::Message;

#[derive(Clone, PartialEq, Message)]
pub struct SentencePieceText {
    /// User input or postprocessed text.
    #[prost(string, optional, tag = "1")]
    pub piece: Option<String>,

    /// A sequence of sentence pieces.
    #[prost(message, repeated, tag = "2")]
    pub pieces: Vec<SentencePiece>,

    /// Score (usually log probability).
    #[prost(float, optional, tag = "3")]
    pub score: Option<f32>,
}

#[derive(Clone, Eq, PartialEq, Message)]
pub struct SentencePiece {
    /// Internal representation for the decoder.
    #[prost(string, optional, tag = "1")]
    pub piece: Option<String>,

    /// Vocabulary id.
    #[prost(uint32, optional, tag = "2")]
    pub id: Option<u32>,

    /// External representation for the client.
    #[prost(string, optional, tag = "3")]
    pub surface: Option<String>,

    /// Starting position.
    #[prost(uint32, optional, tag = "4")]
    pub begin: Option<u32>,

    /// End position.
    #[prost(uint32, optional, tag = "5")]
    pub end: Option<u32>,
}
