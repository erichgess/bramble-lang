use super::{ParserError, ParserErrorKind};
use crate::compiler::lexer::{
    stringtable::StringId,
    tokens::{Lex, Token},
};
use crate::compiler::CompilerError;
//use crate::result::Result;

/// Helper macro to get rid of repitition of boilerplate code.
macro_rules! err {
    ($ln: expr, $kind: expr) => {
        Err(CompilerError::new($ln, $kind))
    };
}

pub struct TokenStream<'a> {
    tokens: &'a Vec<Token>,
    index: usize,
}

impl<'a> TokenStream<'a> {
    pub fn new(tokens: &'a Vec<Token>) -> TokenStream {
        TokenStream { tokens, index: 0 }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn set_index(&mut self, i: usize) {
        self.index = i;
    }

    pub fn next(&mut self) -> Option<Token> {
        if self.index >= self.tokens.len() {
            None
        } else {
            self.index += 1;
            Some(self.tokens[self.index - 1].clone())
        }
    }

    pub fn next_if(&mut self, test: &Lex) -> Option<Token> {
        if self.test_if(test) {
            self.next()
        } else {
            None
        }
    }

    // TODO: return the line # and the ID name
    pub fn next_if_id(&mut self) -> Option<(u32, StringId)> {
        match self.next_if(&Lex::Identifier(StringId::new())) {
            Some(Token {
                l,
                s: Lex::Identifier(id),
                ..
            }) => Some((l, id)),
            Some(_) => None,
            None => None,
        }
    }

    pub fn next_must_be(&mut self, test: &Lex) -> Result<Token, ParserError> {
        let (line, found) = match self.peek() {
            Some(t) => (t.l, t.s),
            None => return err!(0, ParserErrorKind::EOF),
        };
        match self.next_if(test) {
            Some(t) => Ok(t),
            None =>
            //Err(format!("L{}: Expected {}, but found {}", line, test, found)),
            {
                err!(line, ParserErrorKind::ExpectedButFound(test.clone(), found))
            }
        }
    }

    pub fn next_ifn(&mut self, test: Vec<Lex>) -> Option<Vec<Token>> {
        let end = self.index + test.len();
        if self.test_ifn(test) {
            let v: Vec<Token> = self.tokens[self.index..end].into();
            self.index = end;
            Some(v)
        } else {
            None
        }
    }

    pub fn next_if_one_of(&mut self, set: Vec<Lex>) -> Option<Token> {
        if self.test_if_one_of(set) {
            self.next()
        } else {
            None
        }
    }

    pub fn peek(&self) -> Option<&Token> {
        if self.index < self.tokens.len() {
            Some(&self.tokens[self.index])
        } else {
            None
        }
    }

    pub fn peek_at(&self, i: usize) -> Option<&Token> {
        if self.index + i < self.tokens.len() {
            Some(&self.tokens[self.index + i])
        } else {
            None
        }
    }

    pub fn test_if(&self, test: &Lex) -> bool {
        match self.peek() {
            None => false,
            Some(t) => t.token_eq(test),
        }
    }

    pub fn test_ifn(&self, test: Vec<Lex>) -> bool {
        for i in 0..test.len() {
            match self.peek_at(i) {
                None => return false,
                Some(token) => {
                    if !token.token_eq(&test[i]) {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn test_if_one_of(&self, set: Vec<Lex>) -> bool {
        match self.peek() {
            None => false,
            Some(t) => set.iter().find(|l| t.token_eq(l)).is_some(),
        }
    }
}

#[cfg(test)]
mod test_tokenstream {
    use super::TokenStream;
    use crate::compiler::lexer::stringtable::StringTable;
    use crate::compiler::lexer::tokens::{Lex, Token};
    use crate::compiler::Lexer;

    #[test]
    fn test_peek() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let ts = TokenStream::new(&tokens);
        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 0,
                s: Lex::LParen
            }
        );
    }

    #[test]
    fn test_peek_empty() {
        let tokens = vec![];
        let ts = TokenStream::new(&tokens);
        let p = ts.peek();
        assert_eq!(p, None);
    }

    #[test]
    fn test_peek_at() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let ts = TokenStream::new(&tokens);
        let p = ts.peek_at(0).unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 0,
                s: Lex::LParen
            }
        );

        let p = ts.peek_at(1).unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );

        let p = ts.peek_at(8);
        assert_eq!(p, None);
    }

    #[test]
    fn test_next() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let mut ts = TokenStream::new(&tokens);
        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 0,
                s: Lex::LParen
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 3,
                s: Lex::Add
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 5,
                s: Lex::I64(4)
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 6,
                s: Lex::RParen
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 8,
                s: Lex::Mul
            }
        );

        let p = ts.next().unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 10,
                s: Lex::I64(3)
            }
        );

        let p = ts.next();
        assert_eq!(p, None);
    }

    #[test]
    fn test_next_if() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let mut ts = TokenStream::new(&tokens);
        let p = ts.next_if(&Lex::LParen).unwrap(); // should I really use a borrow for this?  If not then gotta do clones and BS i think.
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 0,
                s: Lex::LParen
            }
        );

        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );

        let p = ts.next_if(&Lex::LParen); // should I really use a borrow for this?  If not then gotta do clones and BS i think.
        assert_eq!(p, None);

        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );
    }

    #[test]
    fn test_next_ifn() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let mut ts = TokenStream::new(&tokens);
        let p = ts.next_ifn(vec![Lex::LParen, Lex::I64(0)]).unwrap();
        assert_eq!(
            *p,
            vec![
                Token {
                    l: 1,
                    o: 0,
                    s: Lex::LParen
                },
                Token {
                    l: 1,
                    o: 1,
                    s: Lex::I64(2)
                }
            ]
        );

        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 3,
                s: Lex::Add
            }
        );
    }

    #[test]
    fn test_if_one_of() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let ts = TokenStream::new(&tokens);
        let p = ts.test_if_one_of(vec![Lex::LParen, Lex::I64(0)]);
        assert_eq!(p, true);

        let p = ts.test_if_one_of(vec![Lex::RParen, Lex::I64(0)]);
        assert_eq!(p, false);
    }

    #[test]
    fn test_next_if_on_of() {
        let text = "(2 + 4) * 3";
        let mut table = StringTable::new();
        let tokens: Vec<Token> = Lexer::new(&mut table, &text)
            .tokenize()
            .into_iter()
            .collect::<Result<_, _>>()
            .unwrap();

        let mut ts = TokenStream::new(&tokens);
        let p = ts.next_if_one_of(vec![Lex::LParen, Lex::I64(0)]).unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 0,
                s: Lex::LParen
            }
        );
        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );

        let p = ts.next_if_one_of(vec![Lex::LParen, Lex::I64(0)]).unwrap();
        assert_eq!(
            p,
            Token {
                l: 1,
                o: 1,
                s: Lex::I64(2)
            }
        );
        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 3,
                s: Lex::Add
            }
        );

        let p = ts.next_if_one_of(vec![Lex::LParen, Lex::I64(0)]).is_none();
        assert_eq!(p, true);
        let p = ts.peek().unwrap();
        assert_eq!(
            *p,
            Token {
                l: 1,
                o: 3,
                s: Lex::Add
            }
        );
    }
}
