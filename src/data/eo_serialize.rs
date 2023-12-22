use super::{eo_reader::EoReaderError, eo_writer::EoWriterError, EoReader, EoWriter};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EoSerializeError {
    #[error("Field can not be null: {0}")]
    Null(String),
    #[error("{0}")]
    WriteError(EoWriterError),
}

impl From<EoWriterError> for EoSerializeError {
    fn from(e: EoWriterError) -> Self {
        Self::WriteError(e)
    }
}

pub trait EoSerialize: Sized {
    fn deserialize(reader: &EoReader) -> Result<Self, EoReaderError>;
    fn serialize(&self, writer: &mut EoWriter) -> Result<(), EoSerializeError>;
}
