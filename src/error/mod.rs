use std::io;

use thiserror::Error;

use crate::data::models::file::FileType;

#[derive(Error, Debug)]
pub enum RomstError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    #[error("Unexpected Tag close, was expecting `{expected}`, found `{found}` at position {position}")]
    UnexpectedTagClose {
        expected: String,
        found: String,
        position: usize
    },

    #[error("Unexpected XML tag at position {position}")]
    UnexpectedXMLTag {
        position: usize
    },

    #[error("Parsing error: {message}")]
    ParsingError {
        message: String
    },

    #[error("Unexpected End of File")]
    UnexpectedEOF,

    #[error("Wrong argument")]
    WrongArgument,

    #[error("ERROR: {message}")]
    GenericError {
        message: String
    },

}

#[derive(Error, Debug)]
pub enum RomstIOError {
    #[error("IO Error")]
    Io {
        #[from]
        source: io::Error,
    },

    #[error("Not a valid file, file {0} is not a {1}")]
    NotValidFileError(String, FileType),

    #[error("File not found {0}")]
    FileNotFound(String),
}