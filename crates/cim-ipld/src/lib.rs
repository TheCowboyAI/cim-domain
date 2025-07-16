//! CIM-IPLD: Content-addressed storage for the Composable Information Machine

pub mod chain;
pub mod codec;
pub mod content_types;
pub mod error;
pub mod traits;
pub mod types;
pub mod object_store;

// Re-exports for convenience
pub use cid::Cid;
pub use multihash::Multihash;

pub use chain::{ChainedContent, ContentChain};
pub use codec::{CimCodec, CodecRegistry};
pub use codec::ipld_codecs::{
    standard, cim_json,
    DagCborCodec, DagJsonCodec, RawCodec, JsonCodec,
    AlchemistJsonCodec, WorkflowGraphJsonCodec, ContextGraphJsonCodec,
    CodecOperations, types as codec_types,
};
pub use error::{Error, Result};
pub use traits::TypedContent;
pub use types::ContentType;

// Re-export content types
pub use content_types::{
    // Document types
    PdfDocument, DocxDocument, MarkdownDocument, TextDocument,
    DocumentMetadata,
    
    // Image types  
    JpegImage, PngImage,
    ImageMetadata,
    
    // Audio types
    WavAudio, Mp3Audio, AacAudio, FlacAudio, OggAudio,
    AudioMetadata,
    
    // Video types
    MovVideo, MkvVideo, Mp4Video,
    VideoMetadata,
    
    // Utilities
    detect_content_type, content_type_name,
};
