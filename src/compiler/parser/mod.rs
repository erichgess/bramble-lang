use super::{
    ast::AstError,
    lexer::{stringtable::StringId, tokens::Lex},
    CompilerError, CompilerErrorDisplay,
};

mod statement;
mod tests;
mod tokenstream;

pub mod expression;
pub mod parser;

#[derive(Clone, Debug, PartialEq)]
pub enum ParserError {
    Locked(Option<super::lexer::tokens::Token>),
    ModExpectedName,
    ModAlreadyContains(StringId),
    ExternInvalidVarArgs,
    ExternExpectedFnDecl,
    StructExpectedIdentifier,
    FnExpectedIdentifierAfterFn,
    FnVarArgsNotAllowed,
    FnExpectedTypeAfterArrow,
    FnExpectedReturn(Option<super::lexer::tokens::Token>),
    FnCallExpectedParams,
    CoExpectedIdentifierAfterCo,
    ArrayExpectedIntLiteral,
    ArrayDeclExpectedType,
    ArrayDeclExpectedSize,
    IdDeclExpectedType,
    ExpectedButFound(Lex, Option<Lex>),
    ExpectedIdDeclAfterLet,
    ExpectedTypeAfter,
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

impl CompilerErrorDisplay for ParserError {
    fn format(&self, _st: &crate::StringTable) -> Result<String, String> {
        Ok(format!("Parser error"))
    }
}

impl From<CompilerError<AstError>> for CompilerError<ParserError> {
    fn from(ce: CompilerError<AstError>) -> Self {
        let line = ce.line();
        let ae = ce.inner();
        match ae {
            AstError::ModuleAlreadyContains(sid) => {
                CompilerError::new(line, ParserError::ModAlreadyContains(sid))
            }
        }
    }
}
