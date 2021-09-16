use crate::compiler::{
    ast::AstError,
    lexer::{
        stringtable::StringId,
        tokens::{Lex, Token},
    },
    CompilerDisplay, CompilerError,
};

/// Compiler errors that happen within the Parser stage of compilation.
/// These errors may also cover [AstError]s thrown by the AST during construction;
/// such errors will be transformed into [ParserError]s.
#[derive(Clone, Debug, PartialEq)]
pub enum ParserError {
    Locked(Option<Token>),
    ModExpectedName,
    ModAlreadyContains(StringId),
    ExternInvalidVarArgs,
    ExternExpectedFnDecl,
    StructExpectedIdentifier,
    FnExpectedIdentifierAfterFn,
    FnVarArgsNotAllowed,
    FnExpectedTypeAfterArrow,
    FnExpectedReturn(Option<Token>),
    FnCallExpectedParams,
    CoExpectedIdentifierAfterCo,
    ArrayExpectedIntLiteral,
    ArrayDeclExpectedType,
    ArrayDeclExpectedSize,
    IdDeclExpectedType,
    ExpectedButFound(Vec<Lex>, Option<Lex>),
    ExpectedIdDeclAfterLet,
    ExpectedTypeInIdDecl,
    ExpectedExpressionOnRhs,
    ExpectedParams,
    ExpectedIdAfterInit,
    NotAUnaryOp(Lex),
    NotABinaryOp(Lex),
    IfExpectedConditional,
    IfTrueArmMissingExpr,
    IfElseExpectedIfExpr,
    IfFalseArmMissingExpr,
    WhileExpectedConditional,
    WhileMissingBody,
    PathExpectedIdentifier,
    YieldExpectedIdentifier,
    StructExpectedFieldExpr(StringId),
    ExpectedExprAfter(Lex),
    ExpectedTermAfter(Lex),
    MemberAccessExpectedField,
    IndexOpInvalidExpr,
}

impl CompilerDisplay for ParserError {
    /// Format a ParserError into a human readable message and replace any [`StringId`]s
    /// with their respective string values.
    fn format(&self, st: &crate::StringTable) -> Result<String, String> {
        let msg = match self {
            ParserError::Locked(token) => {
                let ts = token_to_string(token);
                format!("Parser cannot advance past {}", ts)
            }
            ParserError::ModExpectedName => format!("Identifier expected after mod keyword"),
            ParserError::ModAlreadyContains(sid) => {
                format!("Module already contains {}", st.get(*sid)?)
            }
            ParserError::ExternInvalidVarArgs => format!(
                "An extern declaration must have at least one \
                    parameter before a VarArgs (...) parameter"
            ),
            ParserError::ExternExpectedFnDecl => {
                format!("Expected function declaration after extern keyword")
            }
            ParserError::StructExpectedIdentifier => {
                format!("Expected identifier after struct keyword")
            }
            ParserError::FnExpectedIdentifierAfterFn => {
                format!("Expected identifier after fn keyword")
            }
            ParserError::FnVarArgsNotAllowed => {
                format!("Varargs are not allowed in Braid functions (only in externs)")
            }
            ParserError::FnExpectedTypeAfterArrow => format!("Type expected after ->"),
            ParserError::FnExpectedReturn(token) => {
                format!(
                    "Routines must end with a return statement, but found {}",
                    token_to_string(token)
                )
            }
            ParserError::FnCallExpectedParams => {
                format!("Expected parameters after function call point")
            }
            ParserError::CoExpectedIdentifierAfterCo => {
                format!("Expected identifier after co keyword")
            }
            ParserError::ArrayExpectedIntLiteral => {
                format!("Expected integer literal for array size")
            }
            ParserError::ArrayDeclExpectedType => {
                format!("Expected type in array type declaration")
            }
            ParserError::ArrayDeclExpectedSize => {
                format!("Expected size to be specified in array type declaration")
            }
            ParserError::IdDeclExpectedType => {
                format!("Expected type after : in variable declaration")
            }
            ParserError::ExpectedButFound(expected, actual) => {
                format!(
                    "Expected {}, but found {}",
                    lex_set_to_string(expected),
                    lex_to_string(actual)
                )
            }
            ParserError::ExpectedIdDeclAfterLet => {
                format!("Expected identifier declaration (`<id> : <type>`) after let")
            }
            ParserError::ExpectedTypeInIdDecl => {
                format!("Expected type specification in let binding")
            }
            ParserError::ExpectedExpressionOnRhs => format!("Expected expression after :="),
            ParserError::ExpectedParams => format!("Expected parameter list after identifier"),
            ParserError::ExpectedIdAfterInit => format!("Expected identifer after init"),
            ParserError::NotAUnaryOp(op) => format!("{} is not a unary operator", op),
            ParserError::NotABinaryOp(op) => format!("{} is not a binary operator", op),
            ParserError::IfExpectedConditional => {
                format!("Expected conditional expression after if")
            }
            ParserError::IfTrueArmMissingExpr => {
                format!("Expected expression block in true arm of if expression")
            }
            ParserError::IfElseExpectedIfExpr => format!("Expected expression block after else if"),
            ParserError::IfFalseArmMissingExpr => format!("Expected expression block after else"),
            ParserError::WhileExpectedConditional => {
                format!("Expected conditional after while keyword")
            }
            ParserError::WhileMissingBody => {
                format!("Expected expression block for while loop body")
            }
            ParserError::PathExpectedIdentifier => format!("Expected identifier after ::"),
            ParserError::YieldExpectedIdentifier => format!("Expected identifier after yield"),
            ParserError::StructExpectedFieldExpr(sid) => format!(
                "Expected an expression to be assigned to field {}",
                st.get(*sid)?
            ),
            ParserError::ExpectedExprAfter(lex) => {
                format!("Expected expression after {}", lex_to_string(&Some(*lex)))
            }
            ParserError::ExpectedTermAfter(lex) => {
                format!("Expected term after {}", lex_to_string(&Some(*lex)))
            }
            ParserError::MemberAccessExpectedField => {
                format!("Expected member name after . operator.")
            }
            ParserError::IndexOpInvalidExpr => {
                format!("Index operator must contain valid expression")
            }
        };
        Ok(msg)
    }
}

fn token_to_string(token: &Option<Token>) -> String {
    token
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or("EOF".into())
}

fn lex_to_string(lex: &Option<Lex>) -> String {
    lex.as_ref().map(|t| t.to_string()).unwrap_or("EOF".into())
}

fn lex_set_to_string(set: &[Lex]) -> String {
    set.iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join(" or ")
}

impl From<CompilerError<AstError>> for CompilerError<ParserError> {
    fn from(ce: CompilerError<AstError>) -> Self {
        let (line, ae) = ce.take();
        match ae {
            AstError::ModuleAlreadyContains(sid) => {
                CompilerError::new(line, ParserError::ModAlreadyContains(sid))
            }
            AstError::PathTooSuper => todo!(), // TODO: Investigate why this error is in the AST?
        }
    }
}
