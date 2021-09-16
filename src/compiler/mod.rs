pub mod ast;
mod error;
pub mod import;
pub mod lexer;
pub mod llvm;
pub mod parser;
pub mod semantics;
pub mod stringtable;

pub use error::CompilerError;

pub use lexer::lexer::Lexer;

use crate::{StringTable, StringTableError};

/**
Format trait for rendering any Compiler value into a human readable form.
Specifically, this will handle converting any [`StringId`]s into their associated
string value.
*/
pub trait CompilerDisplay {
    /// Uses the given [`StringTable`] to render the associated Compiler type into a
    /// human readable format.
    fn fmt(&self, st: &StringTable) -> Result<String, CompilerDisplayError>;
}

/// Error that gets thrown if formatting a Compiler value for human readability
/// fails.
#[derive(Debug)]
pub enum CompilerDisplayError {
    StringIdNotFound,
}

impl From<StringTableError> for CompilerDisplayError {
    fn from(ste: StringTableError) -> Self {
        match ste {
            StringTableError::NotFound => Self::StringIdNotFound,
        }
    }
}
