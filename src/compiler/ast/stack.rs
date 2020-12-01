use crate::ast::RoutineDef;
use crate::compiler::ast::ast::CompilerNode;
use crate::compiler::ast::scope::{Symbol, Level};

#[derive(Debug)]
pub struct ScopeStack<'a> {
    stack: Vec<&'a CompilerNode>,
}

impl<'a> ScopeStack<'a> {
    pub fn new() -> ScopeStack<'a> {
        ScopeStack { stack: vec![] }
    }

    /// Push a new scope onto the stack.
    pub fn push(&mut self, scope: &'a CompilerNode) {
        self.stack.push(scope);
    }

    /// Pop the current scope off of the stack
    pub fn pop(&mut self) -> Option<&'a CompilerNode> {
        self.stack.pop()
    }

    /// Searches through the stack, starting at the top and going to the bottom, for a
    /// variable with the given name.  This will not search past the root scope of the
    /// current function.
    pub fn find(&self, name: &str) -> Option<&'a Symbol> {
        for node in self.stack.iter().rev() {
            let scope = node.get_metadata();
            let t = scope.get(name);
            if t.is_some() {
                return t;
            }
            match scope.ty {
                Level::Block => (),
                Level::Routine { .. } => return None,
            }
        }

        None
    }

    /// Searched from the top of the stack to the bottom for a Node whose symbol table
    /// contains the given name.  If found it will return a reference to that node.
    /// Unlike `find` this function will not stop at Function boundaries. This allows
    /// it to be used for searching for functions defined in parent scopes all the way up
    /// to the module containing the current node.
    fn find_global(&self, name: &str) -> Option<&'a CompilerNode> {
        for node in self.stack.iter().rev() {
            let scope = node.get_metadata();
            if scope.get(name).is_some() {
                return Some(node);
            }
        }

        None
    }

    pub fn find_func(&self, name: &str) -> Option<&CompilerNode> {
        match self.find_global(name) {
            Some(ref node) => {
                match node {
                    CompilerNode::Module{functions, ..} =>
                        functions.iter().find(|v| match v {CompilerNode::RoutineDef(_, RoutineDef::Function, n, _, _, _) => n == name, _ => false}),
                    _ => None,
                }
            },
            None => None,
        }
    }

    pub fn find_coroutine(&self, name: &str) -> Option<&CompilerNode> {
        match self.find_global(name) {
            Some(ref node) => {
                match node {
                    CompilerNode::Module{coroutines, ..}=>
                        coroutines.iter().find(|v| match v {CompilerNode::RoutineDef(_, RoutineDef::Coroutine, n, _, _, _) => n == name, _ => false}),
                    _ => None,
                }
            },
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::compiler::ast::scope::Scope;
    use super::*;
    use crate::syntax::ast::Type;

    #[test]
    fn test_find_symbol_in_current_scope() {
        let mut scope = Scope::new(Level::Block);
        scope.insert("x", 4, 4);
        let node = CompilerNode::ExpressionBlock(scope, vec![]);
        let mut stack = ScopeStack::new();
        stack.push(&node);

        let sym = stack.find("x").unwrap();
        assert_eq!(sym.name, "x");
        assert_eq!(sym.size, 4);
        assert_eq!(sym.offset, 8);
    }

    #[test]
    fn test_find_symbol_in_outer_scope() {
        let mut stack = ScopeStack::new();

        let mut outer_scope = Scope::new(Level::Block);
        outer_scope.insert("x", 4, 4);
        let outer_node = CompilerNode::ExpressionBlock(outer_scope, vec![]);
        stack.push(&outer_node);
        let inner_scope = Scope::new(Level::Block);
        let inner_node = CompilerNode::ExpressionBlock(inner_scope, vec![]);
        stack.push(&inner_node);

        let sym = stack.find("x").unwrap();
        assert_eq!(sym.name, "x");
        assert_eq!(sym.size, 4);
        assert_eq!(sym.offset, 8);
    }

    #[test]
    fn test_find_symbol_defined_in_both_scopes() {
        let mut stack = ScopeStack::new();

        let mut outer_scope = Scope::new(Level::Block);
        outer_scope.insert("x", 4, 4);
        let outer_node = CompilerNode::ExpressionBlock(outer_scope, vec![]);
        stack.push(&outer_node);
        let mut inner_scope = Scope::new(Level::Block);
        inner_scope.insert("x", 4, 16);
        let inner_node = CompilerNode::ExpressionBlock(inner_scope, vec![]);
        stack.push(&inner_node);

        let sym = stack.find("x").unwrap();
        assert_eq!(sym.name, "x");
        assert_eq!(sym.size, 4);
        assert_eq!(sym.offset, 20);
    }

    #[test]
    fn test_find_symbol_does_not_exist() {
        let mut stack = ScopeStack::new();

        let mut outer_scope = Scope::new(Level::Block);
        outer_scope.insert("x", 4, 4);
        let outer_node = CompilerNode::ExpressionBlock(outer_scope, vec![]);
        stack.push(&outer_node);
        let inner_scope = Scope::new(Level::Block);
        let inner_node = CompilerNode::ExpressionBlock(inner_scope, vec![]);
        stack.push(&inner_node);

        assert_eq!(stack.find("y").is_none(), true);
    }

    #[test]
    fn test_find_symbol_does_not_pass_function() {

        let mut stack = ScopeStack::new();

        let mut outer_scope = Scope::new(Level::Block);
        outer_scope.insert("nope", 4, 4);
        let outer_node = CompilerNode::ExpressionBlock(outer_scope, vec![]);
        stack.push(&outer_node);

        let mut fun_scope = Scope::new(Level::Routine {
            next_label: 0,
            allocation: 8,
        });
        fun_scope.insert("y", 4, 4);
        fun_scope.insert("z", 4, 8);
        let outer_node = CompilerNode::RoutineDef(fun_scope, RoutineDef::Function, "func".into(), vec![], Type::I32, vec![]);
        stack.push(&outer_node);

        let mut inner_scope = Scope::new(Level::Block);
        inner_scope.insert("x", 4, 4);
        let inner_node = CompilerNode::ExpressionBlock(inner_scope, vec![]);
        stack.push(&inner_node);

        assert_eq!(stack.find("x").is_some(), true);
        assert_eq!(stack.find("y").is_some(), true);
        assert_eq!(stack.find("nope").is_some(), false);
    }

    #[test]
    fn test_get_routine_parameters() {
        let mut stack = ScopeStack::new();

        let mut fun_scope = Scope::new(Level::Routine {
            next_label: 0,
            allocation: 8,
        });
        fun_scope.insert("y", 4, 0);
        fun_scope.insert("z", 4, 4);
        let fun_node = CompilerNode::RoutineDef(fun_scope, RoutineDef::Function, "func".into(), vec![("y".into(), Type::I32)], Type::I32, vec![]);

        let mut module_scope = Scope::new(Level::Block);
        module_scope.insert("func", 0, 0);
        let module_node = CompilerNode::Module{meta: module_scope, functions: vec![fun_node], coroutines: vec![], structs: vec![]};
        stack.push(&module_node);

        let fun2_scope = Scope::new(Level::Routine {
            next_label: 0,
            allocation: 0,
        });
        let fun2_node = CompilerNode::RoutineDef(fun2_scope, RoutineDef::Function, "func2".into(), vec![], Type::I32, vec![]);
        stack.push(&fun2_node);
        
        assert_eq!(stack.find("func").is_none(), true);

        let node = stack.find_func("func").unwrap();
        match node {
            CompilerNode::RoutineDef(meta, RoutineDef::Function, name, params, _, _) => {
                assert_eq!(name, "func");
                assert_eq!(params.len(), 1);
                let y_param = meta.get("y").unwrap();
                assert_eq!(y_param.offset, 4);
                assert_eq!(y_param.size, 4);
            },
            _ => assert!(false),
        }
    }

    #[test]
    fn test_get_coroutine_parameters() {
        let mut stack = ScopeStack::new();

        let mut cor_scope = Scope::new(Level::Routine {
            next_label: 0,
            allocation: 8,
        });
        cor_scope.insert("y", 4, 20);
        cor_scope.insert("z", 4, 24);
        let cor_node = CompilerNode::RoutineDef(cor_scope, RoutineDef::Coroutine, "cor".into(), vec![("y".into(), Type::I32)], Type::I32, vec![]);

        let mut module_scope = Scope::new(Level::Block);
        module_scope.insert("cor", 0, 0);
        let module_node = CompilerNode::Module{meta: module_scope, functions: vec![], coroutines: vec![cor_node], structs: vec![]};
        stack.push(&module_node);

        let fun2_scope = Scope::new(Level::Routine {
            next_label: 0,
            allocation: 0,
        });
        let fun2_node = CompilerNode::RoutineDef(fun2_scope, RoutineDef::Coroutine, "func2".into(), vec![], Type::I32, vec![]);
        stack.push(&fun2_node);

        let node = stack.find_coroutine("cor").unwrap();
        match node {
            CompilerNode::RoutineDef(meta, RoutineDef::Coroutine, name, params, _, _) => {
                assert_eq!(name, "cor");
                assert_eq!(params.len(), 1);
                let y_param = meta.get("y").unwrap();
                assert_eq!(y_param.offset, 24);
                assert_eq!(y_param.size, 4);
            },
            _ => assert!(false),
        }
    }
}