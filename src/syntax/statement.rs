use super::{ast::Ast, ty::Type};

#[derive(Clone, Debug, PartialEq)]
pub enum Statement<M> {
    Bind(Box<Ast<M>>),
    Mutate(Box<Ast<M>>),
    Return(Box<Ast<M>>),
    Yield(Box<Ast<M>>),
    YieldReturn(Box<Ast<M>>),
    Printi(Box<Ast<M>>),
    Printiln(Box<Ast<M>>),
    Printbln(Box<Ast<M>>),
    Prints(Box<Ast<M>>),
    Expression(Box<Ast<M>>),
}

impl<M> Statement<M> {
    pub fn get_metadata(&self) -> &M {
        use Statement::*;

        match self {
            Mutate(x) | Return(x) | Yield(x) | YieldReturn(x) | Printi(x) | Printiln(x)
            | Printbln(x) | Prints(x) => x.get_metadata(),
            Expression(e) => e.get_metadata(),
            Bind(b) => b.get_metadata(),
        }
    }

    pub fn from_ast(ast: Ast<M>) -> Option<Statement<M>> {
        match ast {
            Ast::Printi(_, _) => Some(Statement::Printi(Box::new(ast))),
            Ast::Prints(_, _) =>  Some(Statement::Prints(Box::new(ast))),
            Ast::Printiln(_, _) =>  Some(Statement::Printiln(Box::new(ast))),
            Ast::Printbln(_, _) =>  Some(Statement::Printbln(Box::new(ast))),
            Ast::Statement(s) => Some(s),
            Ast::Bind(_, _, _, _, _) => Some(Statement::Bind(Box::new(ast))),
            Ast::Mutate(_, _, _) =>  Some(Statement::Mutate(Box::new(ast))),
            Ast::Return(_, _) =>  Some(Statement::Return(Box::new(ast))),
            Ast::Yield(_, _) =>  Some(Statement::Yield(Box::new(ast))),
            Ast::YieldReturn(_, _) =>  Some(Statement::YieldReturn(Box::new(ast))),
            _ => Some(Statement::Expression(Box::new(ast))),
        }
    }

    pub fn get_metadata_mut(&mut self) -> &mut M {
        use Statement::*;

        match self {
            Mutate(x) | Return(x) | Yield(x) | YieldReturn(x) | Printi(x) | Printiln(x)
            | Printbln(x) | Prints(x) => x.get_metadata_mut(),
            Expression(e) => e.get_metadata_mut(),
            Bind(b) => b.get_metadata_mut(),
        }
    }

    pub fn root_str(&self) -> String {
        use Statement::*;

        match self {
            Mutate(x) | Return(x) | Yield(x) | YieldReturn(x) | Printi(x) | Printiln(x)
            | Printbln(x) | Prints(x) => x.root_str(),
            Expression(e) => e.root_str(),
            Bind(b) => b.root_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bind<M> {
    metadata: M,
    id: String,
    ty: Type,
    mutable: bool,
    rhs: Ast<M>,
}

impl<M> Bind<M> {
    pub fn new(metadata: M, id: &str, ty: Type, mutable: bool, rhs: Ast<M>) -> Bind<M> {
        Bind {
            metadata,
            id: id.into(),
            ty,
            mutable,
            rhs,
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_metadata(&self) -> &M {
        &self.metadata
    }

    pub fn get_metadata_mut(&mut self) -> &mut M {
        &mut self.metadata
    }

    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    pub fn get_rhs(&self) -> &Ast<M> {
        &self.rhs
    }

    pub fn get_type(&self) -> &Type {
        &self.ty
    }

    pub fn root_str(&self) -> String {
        format!("bind {}", self.id)
    }
}
