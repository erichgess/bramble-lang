use crate::compiler::{
    source::SourceChar, CompilerDisplay, CompilerDisplayError, SourceError, SourceMap,
};

use super::tokens::Primitive;

/// Errors which can be encountered while tokenizing a compilation unit
#[derive(Clone, PartialEq, Debug)]
pub enum LexerError {
    Locked(Option<SourceChar>),
    InvalidEscapeSequence(SourceChar),
    ExpectedEscapeCharacter,
    InvalidInteger,
    UnexpectedSuffixType(Primitive),
    SourceError, // TODO: make this more descriptive
}

impl CompilerDisplay for LexerError {
    fn fmt(&self, sm: &SourceMap, st: &crate::StringTable) -> Result<String, CompilerDisplayError> {
        use LexerError::*;
        let msg = match self {
            Locked(None) => format!("Lexer locked on EOF"),
            Locked(Some(c)) => format!("Lexer locked on {}", c),
            InvalidEscapeSequence(c) => format!("Invalid escape sequence \\{}", c),
            ExpectedEscapeCharacter => format!("Expected an escape character after \\"),
            InvalidInteger => format!("Invalid integer"),
            UnexpectedSuffixType(ref prim) => format!("Invalid type suffix: {}", prim.fmt(sm, st)?),
            SourceError => format!("Error reading characters from source code"),
        };

        Ok(format!("{}", msg))
    }
}

impl From<SourceError> for LexerError {
    fn from(_: SourceError) -> Self {
        Self::SourceError
    }
}
