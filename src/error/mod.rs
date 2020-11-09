use thiserror::Error;

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
}