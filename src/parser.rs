// AST - a type(s) which is used to construct an AST representing the logic of the
// program
// Each type of node represents an expression and the only requirement is that at the
// end of computing an expression its result is in EAX
use crate::lexer::{self, Symbol};
use crate::Token;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Primitive {
    I32,
    Bool,
    Unit,
    Unknown,
}

#[derive(Debug)]
pub enum Node {
    Integer(i32),
    Boolean(bool),
    Identifier(String, Primitive),
    Primitive(Primitive),
    Mul(Box<Node>, Box<Node>),
    Add(Box<Node>, Box<Node>),
    BAnd(Box<Node>, Box<Node>),
    BOr(Box<Node>, Box<Node>),
    Gr(Box<Node>, Box<Node>),
    GrEq(Box<Node>, Box<Node>),
    Ls(Box<Node>, Box<Node>),
    LsEq(Box<Node>, Box<Node>),
    Eq(Box<Node>, Box<Node>),
    NEq(Box<Node>, Box<Node>),
    Bind(String, Primitive, Box<Node>),
    Return(Option<Box<Node>>),
    FunctionDef(String, Vec<(String, Primitive)>, Primitive, Vec<Node>),
    FunctionCall(String, Vec<Node>),
    CoroutineDef(String, Vec<(String, Primitive)>, Primitive, Vec<Node>),
    CoroutineInit(String, Vec<Node>),
    Yield(Box<Node>),
    YieldReturn(Option<Box<Node>>),
    If(Box<Node>, Box<Node>, Box<Node>),
    Module(Vec<Node>, Vec<Node>),
    Printi(Box<Node>),
    Printiln(Box<Node>),
    Printbln(Box<Node>),
}

type TokenIter<'a> = std::iter::Peekable<core::slice::Iter<'a, Token>>;
impl Node {
    /*
        Grammar
        PRIMITIVE := i32 | bool
        IDENTIFIER := A-Za-z*
        ID_DEC := IDENTIFIER COLON PRIMITIVE
        NUMBER := 0-9*
        FUNCTION_CALL := IDENTIFIER LPAREN EXPRESSION [, EXPRESSION] RPAREN
        YIELD := yield IDENTIFIER
        IF := if EXPRESSION LBRACE EXPRESSION RBRACE else LBRACE EXPRESSION RBRACE
        FACTOR := FUNCTION_CALL | YIELD | NUMBER | IDENTIFIER | IF
        TERM := FACTOR [* TERM]
        EXPRESSION :=  TERM [+ EXPRESSION]
        INIT_CO := init IDENTIFIER
        BIND := ID_DEC := (EXPRESSION|INIT_CO)
        PRINTLN := println EXPRESSION ;
        RETURN := return [EXPRESSION] SEMICOLON
        YIELD_RETURN := yield return [EXPRESSION] SEMICOLON
        STATEMENT := [BIND] SEMICOLON
        BLOCK := STATEMENT*
        COBLOCK := [STATEMENT | YIELD_RETURN]*
        FUNCTION := fn IDENTIFIER LPAREN [ID_DEC [, ID_DEC]*] RPAREN  [LARROW PRIMITIVE] LBRACE BLOCK RETURN RBRACE
        COROUTINE := co IDENTIFIER LPAREN [ID_DEC [, ID_DEC]*] RPAREN [LARROW PRIMITIVE] LBRACE COBLOCK RETURN RBRACE
        MODULES := [FUNCTION|COROUTINE]*

        tokenize - takes a string of text and converts it to a string of tokens
        parse - takes a string of tokens and converts it into an AST
        compile - takes an AST and converts it to assembly
    */
    pub fn parse(tokens: Vec<Token>) -> Option<Node> {
        let mut iter = tokens.iter().peekable();
        //Node::function(&mut iter)
        Node::module(&mut iter)
    }

    fn module(iter: &mut TokenIter) -> Option<Node> {
        let mut functions = vec![];
        let mut coroutines = vec![];

        while iter.peek().is_some() {
            match Node::function_def(iter) {
                Some(f) => functions.push(f),
                None => match Node::coroutine_def(iter) {
                    Some(co) => coroutines.push(co),
                    None => break,
                },
            }
        }

        if functions.len() > 0 {
            Some(Node::Module(functions, coroutines))
        } else {
            None
        }
    }

    fn function_def(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::FunctionDef}) => {
                iter.next();
                match iter.peek() {
                    Some(Token{symbol: Symbol::Identifier(id)}) => {
                        iter.next();

                        let params = Node::fn_def_params(iter);

                        let fn_type = match iter.peek() {
                            Some(Token{symbol: Symbol::LArrow}) => {
                                iter.next();
                                Node::primitive(iter).expect(
                                    "Expected primitive type after -> in function definition",
                                )
                            }
                            _ => Primitive::Unit,
                        };

                        match iter.peek() {
                            Some(Token{symbol: Symbol::LBrace}) => {
                                iter.next();
                                let mut stmts = Node::block(iter);

                                match Node::return_stmt(iter) {
                                    Some(ret) => stmts.push(ret),
                                    None => panic!(
                                        "Function must end with a return statement, got {:?}",
                                        iter.peek()
                                    ),
                                }

                                match iter.peek() {
                                    Some(Token{symbol: Symbol::RBrace}) => {
                                        iter.next();
                                    }
                                    _ => panic!("Expected } at end of function definition"),
                                }
                                Some(Node::FunctionDef(id.clone(), params, fn_type, stmts))
                            }
                            _ => panic!("Expected { after function declaration"),
                        }
                    }
                    _ => panic!("Expected function name after fn"),
                }
            }
            _ => None,
        }
    }

    fn coroutine_def(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::CoroutineDef}) => {
                iter.next();
                match iter.peek() {
                    Some(Token{symbol: Symbol::Identifier(id)}) => {
                        iter.next();

                        let params = Node::fn_def_params(iter);

                        let co_type = match iter.peek() {
                            Some(Token{symbol: Symbol::LArrow}) => {
                                iter.next();
                                Node::primitive(iter).expect(
                                    "Expected primitive type after -> in function definition",
                                )
                            }
                            _ => Primitive::Unit,
                        };

                        match iter.peek() {
                            Some(Token{symbol: Symbol::LBrace}) => {
                                iter.next();
                                let mut stmts = Node::co_block(iter);

                                match Node::return_stmt(iter) {
                                    Some(ret) => stmts.push(ret),
                                    None => panic!("Coroutine must end with a return statement"),
                                }

                                match iter.peek() {
                                    Some(Token{symbol: Symbol::RBrace}) => {
                                        iter.next();
                                    }
                                    _ => panic!("Expected } at end of function definition"),
                                }
                                Some(Node::CoroutineDef(id.clone(), params, co_type, stmts))
                            }
                            _ => panic!("Expected { after function declaration"),
                        }
                    }
                    _ => panic!("Expected function name after fn"),
                }
            }
            _ => None,
        }
    }

    fn fn_def_params(iter: &mut TokenIter) -> Vec<(String, Primitive)> {
        match iter.peek() {
            Some(Token{symbol: Symbol::LParen}) => {
                iter.next();
            }
            _ => panic!("Parser: expected an ( after function name in function definition"),
        }

        let mut params = vec![];

        while let Some(param) = Node::identifier_declare(iter) {
            match param {
                Node::Identifier(id, id_type) => {
                    params.push((id, id_type));
                    match iter.peek() {
                        Some(Token{symbol: Symbol::Comma}) => {
                            iter.next();
                        }
                        Some(Token{symbol: Symbol::RParen}) => break,
                        Some(t) => panic!("Unexpected token in function definition: {:?}", t),
                        None => panic!("Parser: unexpected EOF"),
                    };
                }
                _ => panic!("Parser: invalid parameter declaration in function definition"),
            }
        }

        match iter.peek() {
            Some(Token{symbol: Symbol::RParen}) => {
                iter.next();
            }
            _ => panic!("Parser: expected )"),
        }

        params
    }

    fn block(iter: &mut TokenIter) -> Vec<Node> {
        let mut stmts = vec![];
        while iter.peek().is_some() {
            match Node::statement(iter) {
                Some(s) => stmts.push(s),
                None => break,
            }
        }
        stmts
    }

    fn co_block(iter: &mut TokenIter) -> Vec<Node> {
        let mut stmts = vec![];
        while iter.peek().is_some() {
            match Node::statement(iter) {
                Some(s) => stmts.push(s),
                None => match Node::yield_return_stmt(iter) {
                    Some(s) => stmts.push(s),
                    None => break,
                },
            }
        }
        stmts
    }

    fn return_stmt(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Return}) => {
                iter.next();
                let exp = Node::expression(iter);
                match iter.peek() {
                    Some(Token{symbol: Symbol::Semicolon}) => iter.next(),
                    _ => panic!("Expected ; after return statement"),
                };
                match exp {
                    Some(exp) => Some(Node::Return(Some(Box::new(exp)))),
                    None => Some(Node::Return(None)),
                }
            }
            _ => None,
        }
    }

    fn yield_return_stmt(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::YieldReturn}) => {
                iter.next();
                let exp = Node::expression(iter);
                match iter.peek() {
                    Some(Token{symbol: Symbol::Semicolon}) => iter.next(),
                    _ => panic!("Expected ; after yield return statement"),
                };
                match exp {
                    Some(exp) => Some(Node::YieldReturn(Some(Box::new(exp)))),
                    None => Some(Node::YieldReturn(None)),
                }
            }
            _ => None,
        }
    }

    fn statement(iter: &mut TokenIter) -> Option<Node> {
        let stm = match Node::bind(iter) {
            Some(b) => Some(b),
            None => match Node::println_stmt(iter) {
                Some(p) => Some(p),
                _ => None,
            },
        };

        if stm.is_some() {
            match iter.peek() {
                Some(Token{symbol: Symbol::Semicolon}) => {
                    iter.next();
                }
                _ => panic!(format!(
                    "Exected ; after statement, found {:?}",
                    iter.peek()
                )),
            }
        }

        stm
    }

    fn println_stmt(iter: &mut TokenIter) -> Option<Node> {
        let tk = iter.peek();
        match tk {
            Some(Token{symbol: Symbol::Printiln}) => {
                iter.next();
                let exp = Node::expression(iter);
                match exp {
                    Some(exp) => Some(Node::Printiln(Box::new(exp))),
                    None => panic!("Parser: Expected expression after println"),
                }
            }
            Some(Token{symbol: Symbol::Printbln}) => {
                iter.next();
                let exp = Node::expression(iter);
                match exp {
                    Some(exp) => Some(Node::Printbln(Box::new(exp))),
                    None => panic!("Parser: Expected expression after println"),
                }
            }
            _ => None,
        }
    }

    fn bind(iter: &mut TokenIter) -> Option<Node> {
        match Node::identifier_declare(iter) {
            Some(Node::Identifier(id, id_type)) => {
                let pt = iter.peek();
                match pt {
                    Some(Token{symbol: Symbol::Assign}) => {
                        iter.next();
                        match iter.peek() {
                            Some(Token{symbol: Symbol::Init}) => {
                                let co_init =
                                    Node::co_init(iter).expect("Parser: Invalid coroutine init");
                                Some(Node::Bind(id, id_type, Box::new(co_init)))
                            }
                            _ => {
                                let exp = Node::expression(iter).expect(&format!(
                                    "Expected an expression or coroutine init after :=, found {:?}",
                                    iter.peek()
                                ));
                                Some(Node::Bind(id, id_type, Box::new(exp)))
                            }
                        }
                    }
                    _ => {
                        panic!("Expected := after identifer in bind statement");
                    }
                }
            }
            Some(_) => panic!("Parser: invalid LHS in bind expresion"),
            None => None,
        }
    }

    fn co_init(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Init}) => {
                iter.next();
                match iter.peek() {
                    Some(Token{symbol: Symbol::Identifier(id)}) => {
                        iter.next();
                        let params = Node::fn_call_params(iter)
                            .expect("Expected parameters after coroutine name");
                        Some(Node::CoroutineInit(id.clone(), params))
                    }
                    _ => {
                        panic!("Parser: expected identifier after init");
                    }
                }
            }
            _ => None,
        }
    }

    fn expression(iter: &mut TokenIter) -> Option<Node> {
        Node::logical_or(iter)
    }

    fn logical_or(iter: &mut TokenIter) -> Option<Node> {
        match Node::logical_and(iter) {
            Some(n) => match iter.peek() {
                Some(Token{symbol: Symbol::BOr}) => {
                    iter.next();
                    let n2 = Node::logical_or(iter).expect("An expression after ||");
                    Some(Node::BOr(Box::new(n), Box::new(n2)))
                }
                _ => Some(n),
            },
            None => None,
        }
    }

    fn logical_and(iter: &mut TokenIter) -> Option<Node> {
        match Node::comparison(iter) {
            Some(n) => match iter.peek() {
                Some(Token{symbol: Symbol::BAnd}) => {
                    iter.next();
                    let n2 = Node::logical_and(iter).expect("An expression after ||");
                    Some(Node::BAnd(Box::new(n), Box::new(n2)))
                }
                _ => Some(n),
            },
            None => None,
        }
    }

    fn comparison(iter: &mut TokenIter) -> Option<Node> {
        match Node::sum(iter) {
            Some(n) => match iter.peek() {
                Some(Token{symbol: Symbol::Eq}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after ==");
                    Some(Node::Eq(Box::new(n), Box::new(n2)))
                }
                Some(Token{symbol: Symbol::NEq}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after !=");
                    Some(Node::NEq(Box::new(n), Box::new(n2)))
                }
                Some(Token{symbol: Symbol::Gr}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after >");
                    Some(Node::Gr(Box::new(n), Box::new(n2)))
                }
                Some(Token{symbol: Symbol::GrEq}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after >=");
                    Some(Node::GrEq(Box::new(n), Box::new(n2)))
                }
                Some(Token{symbol: Symbol::Ls}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after <");
                    Some(Node::Ls(Box::new(n), Box::new(n2)))
                }
                Some(Token{symbol: Symbol::LsEq}) => {
                    iter.next();
                    let n2 = Node::comparison(iter).expect("An expression after <=");
                    Some(Node::LsEq(Box::new(n), Box::new(n2)))
                }
                _ => Some(n),
            },
            None => None,
        }
    }

    fn sum(iter: &mut TokenIter) -> Option<Node> {
        match Node::term(iter) {
            Some(n) => match iter.peek() {
                Some(Token{symbol: Symbol::Add}) => {
                    iter.next();
                    let n2 = Node::sum(iter).expect("An expression after +");
                    Some(Node::Add(Box::new(n), Box::new(n2)))
                }
                _ => Some(n),
            },
            None => None,
        }
    }

    fn term(iter: &mut TokenIter) -> Option<Node> {
        match Node::factor(iter) {
            Some(n) => match iter.peek() {
                Some(Token{symbol: Symbol::Mul}) => {
                    iter.next();
                    let n2 = Node::term(iter).expect("a valid term after *");
                    Some(Node::Mul(Box::new(n), Box::new(n2)))
                }
                _ => Some(n),
            },
            None => None,
        }
    }

    fn factor(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::If}) => Node::if_expression(iter),
            Some(Token{symbol: Symbol::LParen}) => {
                iter.next();
                let exp = Node::expression(iter);
                match iter.peek() {
                    Some(Token{symbol: Symbol::RParen}) => iter.next(),
                    x => panic!("Parser: exected ) but found {:?}", x),
                };
                exp
            }
            _ => match Node::constant(iter) {
                Some(n) => Some(n),
                None => match Node::function_call_or_variable(iter) {
                    Some(n) => Some(n),
                    None => match Node::co_yield(iter) {
                        Some(n) => Some(n),
                        None => None,
                    },
                },
            },
        }
    }

    fn if_expression(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::If}) => {
                iter.next();
                // expression
                let cond =
                    Node::expression(iter).expect("Expected conditional expressoin after if");
                // lbrace
                iter.next()
                    .map(|t| *t == Token{symbol: Symbol::LBrace})
                    .expect("Expected {");
                // expression
                let true_arm =
                    Node::expression(iter).expect("Expression in true arm of if expression");
                // rbrace
                iter.next()
                    .map(|t| *t == Token{symbol: Symbol::RBrace})
                    .expect("Expected }");
                // else
                iter.next()
                    .map(|t| *t == Token{symbol: Symbol::Else})
                    .expect("Expected else arm of if expression");

                // check for `else if`
                let false_arm = match iter.peek() {
                    Some(Token{symbol: Symbol::If}) => {
                        Node::if_expression(iter).expect("Expected if expression after else if")
                    }
                    _ => {
                        iter.next()
                            .map(|t| *t == Token{symbol: Symbol::LBrace})
                            .expect("Expected {");
                        // expression
                        let false_arm = Node::expression(iter)
                            .expect("Expression in false arm of if expression");
                        // rbrace
                        iter.next()
                            .map(|t| *t == Token{symbol: Symbol::RBrace})
                            .expect("Expected }");
                        false_arm
                    }
                };
                Some(Node::If(
                    Box::new(cond),
                    Box::new(true_arm),
                    Box::new(false_arm),
                ))
            }
            _ => None,
        }
    }

    fn function_call_or_variable(iter: &mut TokenIter) -> Option<Node> {
        match Node::identifier(iter) {
            Some(Node::Identifier(id, _id_type)) => match Node::fn_call_params(iter) {
                Some(params) => {
                    // this is a function call
                    Some(Node::FunctionCall(id, params))
                }
                _ => Some(Node::Identifier(id, _id_type)),
            },
            Some(_) => panic!("Parser: expected identifier"),
            None => None,
        }
    }

    /// LPAREN [EXPRESSION [, EXPRESSION]*] RPAREN
    fn fn_call_params(iter: &mut TokenIter) -> Option<Vec<Node>> {
        match iter.peek() {
            Some(Token{symbol: Symbol::LParen}) => {
                // this is a function call
                iter.next();

                let mut params = vec![];
                while let Some(param) = Node::expression(iter) {
                    match param {
                        exp => {
                            params.push(exp);
                            match iter.peek() {
                                Some(Token{symbol: Symbol::Comma}) => {
                                    iter.next();
                                }
                                Some(Token{symbol: Symbol::RParen}) => break,
                                Some(t) => panic!("Unexpected token in function call: {:?}", t),
                                None => panic!("Parser: unexpected EOF"),
                            };
                        }
                    }
                }

                match iter.peek() {
                    Some(Token{symbol: Symbol::RParen}) => {
                        iter.next();
                    }
                    _ => panic!("Parser: expected ) after function call"),
                }
                Some(params)
            }
            _ => None,
        }
    }

    fn co_yield(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Yield}) => {
                iter.next();
                match Node::identifier(iter) {
                    Some(id) => Some(Node::Yield(Box::new(id))),
                    _ => panic!("Parser: expected an identifier after yield"),
                }
            }
            _ => None,
        }
    }

    fn primitive(iter: &mut TokenIter) -> Option<Primitive> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Primitive(primitive)}) => {
                iter.next();
                match primitive {
                    lexer::Primitive::I32 => Some(Primitive::I32),
                    lexer::Primitive::Bool => Some(Primitive::Bool),
                }
            }
            _ => None,
        }
    }

    fn identifier_declare(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Identifier(id)}) => {
                iter.next();
                match iter.peek() {
                    Some(Token{symbol: Symbol::Colon}) => {
                        iter.next();
                        match Node::primitive(iter) {
                            Some(p) => Some(Node::Identifier(id.clone(), p)),
                            None => panic!("Parser: Invalid primitive type: {:?}", iter.peek()),
                        }
                    }
                    _ => panic!(
                        "Parser: Expected type after variable declaration, found: {:?}",
                        iter.peek()
                    ),
                }
            }
            _ => None,
        }
    }

    fn identifier(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(token) => match token {
                Token{symbol: Symbol::Identifier(id)} => {
                    iter.next();
                    Some(Node::Identifier(id.clone(), Primitive::Unknown))
                }
                _ => None,
            },
            None => None,
        }
    }

    fn constant(iter: &mut TokenIter) -> Option<Node> {
        match Node::number(iter) {
            None => match Node::boolean(iter) {
                None => None,
                Some(t) => Some(t),
            },
            Some(i) => Some(i),
        }
    }

    fn number(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(token) => match token {
                Token{symbol: Symbol::Integer(i)} => {
                    iter.next();
                    Some(Node::Integer(*i))
                }
                _ => None,
            },
            None => None,
        }
    }

    fn boolean(iter: &mut TokenIter) -> Option<Node> {
        match iter.peek() {
            Some(Token{symbol: Symbol::Bool(b)}) => {
                iter.next();
                Some(Node::Boolean(*b))
            }
            _ => None,
        }
    }
}
