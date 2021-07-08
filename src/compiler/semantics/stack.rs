use std::collections::HashMap;

use crate::compiler::ast::{Module, Node, Path, StructDef, Type, CANONICAL_ROOT};
use braid_lang::result::Result;

use super::{
    semanticnode::SemanticAnnotations,
    symbol_table::{ScopeType, Symbol, SymbolTable},
};

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolTableScopeStack {
    root: *const Module<SemanticAnnotations>,

    stack: Vec<SymbolTable>,
    head: SymbolTable,
    imported_symbols: HashMap<String, Symbol>, // TODO: change this to a SymbolTable?
}

impl<'a> std::fmt::Display for SymbolTableScopeStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut i = 0;
        f.write_fmt(format_args!("{}: {}\n", i, self.head))?;
        for scope in self.stack.iter().rev() {
            i += 1;
            f.write_fmt(format_args!("{}: {}\n", i, scope))?;
        }
        Ok(())
    }
}

impl<'a> SymbolTableScopeStack {
    pub fn new(root: &'a Module<SemanticAnnotations>) -> SymbolTableScopeStack {
        SymbolTableScopeStack {
            stack: vec![],
            head: SymbolTable::new(),
            root,
            imported_symbols: HashMap::new(),
        }
    }

    pub fn get_root(&self) -> &'a Module<SemanticAnnotations> {
        unsafe { self.root.as_ref().unwrap() }
    }

    /// Add a function from another module to this symbol table
    /// So that calls to external functions can be type checked.
    pub fn import_function(
        &mut self,
        canonical_name: Path,
        params: Vec<Type>,
        return_ty: Type,
    ) -> Option<Symbol> {
        match canonical_name.item() {
            Some(item) => self.imported_symbols.insert(
                canonical_name.to_string(),
                Symbol {
                    name: item.into(),
                    ty: Type::FunctionDef(params, Box::new(return_ty)),
                    mutable: false,
                    is_extern: false,
                },
            ),
            None => None,
        }
    }

    /// Add a function from another module to this symbol table
    /// So that calls to external functions can be type checked.
    pub fn import_structdef(&mut self, sd: &StructDef<SemanticAnnotations>) -> Option<Symbol> {
        let canon_path = sd.annotation().get_canonical_path();
        match canon_path.item() {
            Some(item) => self.imported_symbols.insert(
                canon_path.to_string(),
                Symbol {
                    name: item.into(),
                    ty: Type::StructDef(
                        sd.get_fields()
                            .iter()
                            .map(|p| (p.name.clone(), p.ty.clone()))
                            .collect(),
                    ),
                    mutable: false,
                    is_extern: false,
                },
            ),
            None => None,
        }
    }

    fn get_imported_symbol(&self, canonical_name: &Path) -> Option<&Symbol> {
        self.imported_symbols.get(&canonical_name.to_string())
    }

    pub fn enter_scope(&mut self, sym: &SymbolTable) {
        self.stack.push(self.head.clone());
        self.head = sym.clone();
    }

    pub fn leave_scope(&mut self) -> SymbolTable {
        let tmp = self.head.clone();
        self.head = self
            .stack
            .pop()
            .expect("SymbolTable stack should never be empty on a pop");
        tmp
    }

    pub fn get_current_fn(&self) -> Option<&str> {
        // Check if the top of the stack is a routine
        if let ScopeType::Routine { name } = self.head.scope_type() {
            return Some(name);
        } else {
            // Search through the rest of the stack for the Routine closest to the top
            for scope in self.stack.iter().rev() {
                if let ScopeType::Routine { name } = scope.scope_type() {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Searches SymbolStack, starting at the top of the stack and moving down,
    /// for a symbol that matches `name`.
    ///
    /// Returns the first match and the canonical path to that match.  
    /// Returns `None` if no matching symbol was found.
    fn get_symbol(&self, name: &str) -> Option<(&Symbol, Path)> {
        let mut cpath = self.to_path()?;
        //println!("get_symbol: cpath: {}", cpath);
        let s = self.head.get(name).or_else(|| {
            self.stack.iter().rev().find_map(|scope| {
                let s = scope.get(name);
                if s.is_none() && scope.scope_type().is_boundary() {
                    // If we reach the end of the canonical path, there can be no more locations
                    // for the symbol to exist and so we should return None
                    cpath.pop()?;
                }
                s
            })
        });

        s.map(|s| {
            cpath.push(name);
            (s, cpath)
        })
    }

    /// Add a new symbol to the current symbol table (the SymbolTable that is at the
    /// top of the stack).
    pub fn add(&mut self, name: &str, ty: Type, mutable: bool, is_extern: bool) -> Result<()> {
        self.head.add(name, ty, mutable, is_extern)
    }

    /// Finds the given variable in the current symbol table or in the symbol table history
    /// Follows scoping rules, so when a boundary scope is reached (e.g. a Routine) it will
    /// stop searching
    pub fn lookup_var(&'a self, id: &str) -> Result<&'a Symbol> {
        let (symbol, _) = &self.lookup_symbol_by_path(&vec![id].into())?;
        match symbol.ty {
            Type::FunctionDef(..)
            | Type::CoroutineDef(..)
            | Type::ExternDecl(..)
            | Type::StructDef { .. }
            | Type::Unknown => return Err(format!("{} is not a variable", id)),
            Type::Custom(..)
            | Type::Coroutine(_)
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::Bool
            | Type::StringLiteral
            | Type::Array(_, _)
            | Type::Unit => Ok(symbol),
        }
    }

    /// Specifically looks for a routine (function or coroutine) with the given ID.  Will search upward through the scope
    /// hierarchy until a symbol is found that matches `id`. If that symbol is a routine it is returned
    /// if the symbol is not a routine `Err` is returned.  If no symbol is found `Err` is returned.
    pub fn lookup_func_or_cor(&'a self, id: &str) -> Result<(&Vec<Type>, &Type)> {
        match self.lookup_symbol_by_path(&vec![id].into())?.0 {
            Symbol {
                ty: Type::CoroutineDef(params, p),
                ..
            }
            | Symbol {
                ty: Type::FunctionDef(params, p),
                ..
            } => Ok((params, p)),
            _ => return Err(format!("{} is not a coroutine or function", id)),
        }
    }

    /// Specifically looks for a coroutine with the given ID.  Will search upward through the scope
    /// hierarchy until a symbol is found that matches `id`. If that symbol is a coroutine it is returned
    /// if the symbol is not a coroutine `Err` is returned.  If no symbol is found `Err` is returned.
    pub fn lookup_coroutine(&'a self, id: &str) -> Result<(&Vec<Type>, &Type)> {
        match self.lookup_symbol_by_path(&vec![id].into())?.0 {
            Symbol {
                ty: Type::CoroutineDef(params, p),
                ..
            } => Ok((params, p)),
            _ => return Err(format!("{} is not a coroutine", id)),
        }
    }

    /// Uses the given path to navigate through the Symbol table hierarchy to locate
    /// a specific symbol.  If the path is invalid (contains modules which do not exist)
    /// or the item identified by the path does not exist, then an error is returned.
    ///
    /// This function will work with relative and canonical paths.
    pub fn lookup_symbol_by_path(&'a self, path: &Path) -> Result<(&'a Symbol, Path)> {
        //println!("{} - {}", stdext::function_name!(), path);
        if path.len() > 1 {
            let canon_path = self.to_canonical(path)?;

            // Look in the project being compiled
            let project_symbol = self.get_item(&canon_path);

            // look in any imported symbols
            let imported_symbol = self.get_imported_symbol(&canon_path);

            // Make sure that there is no ambiguity about what is being referenced
            match (project_symbol, imported_symbol) {
                (Some(ps), None) => Ok((ps, canon_path)),
                (None, Some(is)) => Ok((is, canon_path)),
                (Some(_), Some(_)) => Err(format!("Found multiple definitions of {}", path)),
                (None, None) => Err(format!(
                    "Could not find item with the given path: {} ({})",
                    path, canon_path
                )),
            }
        } else if path.len() == 1 {
            // If the path has just the item name, then check the local scope and
            // the parent scopes for the given symbol
            let item = &path[0];
            self.get_symbol(item)
                .ok_or(format!("{} is not defined", item))
        } else {
            Err("empty path passed to lookup_path".into())
        }
        .map(|(s, p)| {
            if s.is_extern {
                (s, vec![s.name.clone()].into())
            } else {
                (s, p)
            }
        })
    }

    fn get_item(&self, canon_path: &Path) -> Option<&Symbol> {
        // If the path contains more than just the item's name then
        // traverse the parent path to find the specified item
        let item = canon_path
            .item()
            .expect("Expected a canonical path with at least one step in it");

        if !canon_path.is_canonical() {
            panic!("Given path is not canonical: {}", canon_path);
        }

        let mut current = self.root;
        // Follow the path, up to, but not including the final element of the path
        // (which is the item being looked for);
        unsafe {
            for idx in 1..canon_path.len() - 1 {
                /*(println!(
                    "[{}] {:?}: {:?}",
                    canon_path,
                    (*current).get_name(),
                    (*current)
                        .get_modules()
                        .iter()
                        .map(|m| (
                            m.get_name(),
                            m.get_functions()
                                .iter()
                                .map(|f| f.name().unwrap())
                                .collect::<Vec<&str>>()
                        ))
                        .collect::<Vec<_>>()
                );*/
                match (*current).get_module(&canon_path[idx]) {
                    Some(m) => current = m,
                    None => return None,
                }
            }

            (*current).annotation().sym.get(item)
        }
    }

    /**
    Given a type reference that appears in a node that is not the current node, will convert
    that type reference to a canonical path from a relative path.  If the type reference is
    already an absolute path then no change is made.  This is used for indirect type reference
    look ups: for example, if the current node is a routine call and the routine definition is
    looked up to validate the parameter types in the definition agains the parameters in the
    call, to canonize the routine definition's parameter types, this function would be used: as
    they are in the RoutineDef node not the RoutineCall node.
     */
    pub fn canonize_nonlocal_type_ref(&self, parent_path: &Path, ty: &Type) -> Result<Type> {
        match ty {
            Type::Custom(path) => Ok(Type::Custom(path.to_canonical(parent_path)?)),
            Type::Coroutine(ty) => Ok(Type::Coroutine(Box::new(
                self.canonize_nonlocal_type_ref(parent_path, &ty)?,
            ))),
            Type::Array(el_ty, len) => {
                if *len <= 0 {
                    Err(format!("Expected length > 0 for array, but found {}", *len))
                } else {
                    Ok(Type::Array(
                        box self.canonize_nonlocal_type_ref(parent_path, el_ty)?,
                        *len,
                    ))
                }
            }
            _ => Ok(ty.clone()),
        }
    }

    /**
    Given a type reference that appears in the current node, will convert that type reference
    to a canonical path from a relative path.  If the type reference is already an absolute
    path then no change is made.

    For example, the path `super::MyStruct` would be converted to `root::my_module::MyStruct`
    if the current node were in a module contained within `my_module`.
     */
    pub fn canonize_local_type_ref(&self, ty: &Type) -> Result<Type> {
        match ty {
            Type::Custom(path) => Ok(Type::Custom(self.to_canonical(path)?)),
            Type::Coroutine(ty) => Ok(Type::Coroutine(Box::new(
                self.canonize_local_type_ref(&ty)?,
            ))),
            Type::CoroutineDef(params, ret_ty) => {
                let cparams = params
                    .iter()
                    .map(|pty| self.canonize_local_type_ref(pty))
                    .collect::<Result<Vec<Type>>>()?;
                let cret_ty = self.canonize_local_type_ref(ret_ty)?;
                Ok(Type::CoroutineDef(cparams, Box::new(cret_ty)))
            }
            Type::FunctionDef(params, ret_ty) => {
                let cparams = params
                    .iter()
                    .map(|pty| self.canonize_local_type_ref(pty))
                    .collect::<Result<Vec<Type>>>()?;
                let cret_ty = self.canonize_local_type_ref(ret_ty)?;
                Ok(Type::FunctionDef(cparams, Box::new(cret_ty)))
            }
            Type::Array(el_ty, len) => {
                if *len <= 0 {
                    Err(format!("Expected length > 0 for array, but found {}", *len))
                } else {
                    Ok(Type::Array(box self.canonize_local_type_ref(el_ty)?, *len))
                }
            }
            _ => Ok(ty.clone()),
        }
    }

    /// Converts a relative path, `path`, into a canonical path by merging it with
    /// the path to the current node, as represented by the stack.
    pub fn to_canonical(&self, path: &Path) -> Result<Path> {
        let current_path = self.to_path().ok_or("A valid path is expected")?;
        path.to_canonical(&current_path)
    }

    /// Starting from the bottom of the stack this builds a path
    /// of all the modules that we are current in, in effect
    /// the current path within the AST.
    pub fn to_path(&self) -> Option<Path> {
        let mut steps: Vec<String> = vec![CANONICAL_ROOT.into()];

        for node in self.stack.iter() {
            match node.scope_type() {
                ScopeType::Module { name } | ScopeType::Routine { name } => {
                    steps.push(name.clone());
                }
                _ => (),
            }
        }

        match self.head.scope_type() {
            ScopeType::Module { name } => steps.push(name.clone()),
            _ => (),
        }

        if steps.len() > 0 {
            Some(steps.into())
        } else {
            None
        }
    }
}
