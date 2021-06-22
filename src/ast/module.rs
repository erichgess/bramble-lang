use super::{
    extern_decl::Extern,
    node::{
        Annotation, Node, NodeType, {PostOrderIter, PreOrderIter},
    },
    path::Path,
    routinedef::{RoutineDef, RoutineDefType},
    structdef::StructDef,
};
use braid_lang::result::Result;

#[derive(Clone, Debug, PartialEq)]
pub struct Module<M> {
    annotations: M,
    name: String,
    modules: Vec<Module<M>>,
    functions: Vec<Item<M>>,
    coroutines: Vec<Item<M>>,
    structs: Vec<Item<M>>,
    externs: Vec<Item<M>>,
}

impl<M: Annotation> Node<M> for Module<M> {
    fn annotation(&self) -> &M {
        &self.annotations
    }

    fn annotation_mut(&mut self) -> &mut M {
        &mut self.annotations
    }

    fn node_type(&self) -> NodeType {
        NodeType::Module
    }

    fn children(&self) -> Vec<&dyn Node<M>> {
        let mut v: Vec<&dyn Node<M>> = vec![];
        for m in self.modules.iter() {
            v.push(m);
        }
        for f in self.functions.iter() {
            v.push(f);
        }
        for c in self.coroutines.iter() {
            v.push(c);
        }
        for s in self.structs.iter() {
            v.push(s);
        }
        v
    }

    fn name(&self) -> Option<&str> {
        Some(&self.name)
    }

    fn iter_postorder(&self) -> PostOrderIter<M> {
        PostOrderIter::new(self)
    }

    fn iter_preorder(&self) -> PreOrderIter<M> {
        PreOrderIter::new(self)
    }
}

impl<M> std::fmt::Display for Module<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.write_str(self.get_name())
    }
}

impl<M> Module<M> {
    pub fn new(name: &str, annotation: M) -> Module<M> {
        Module {
            annotations: annotation,
            name: name.into(),
            modules: Vec::new(),
            functions: Vec::new(),
            coroutines: Vec::new(),
            structs: Vec::new(),
            externs: Vec::new(),
        }
    }

    pub fn add_module(&mut self, m: Module<M>) {
        self.modules.push(m);
    }

    pub fn add_function(&mut self, f: RoutineDef<M>) -> Result<()> {
        let fname = f.get_name();
        if self.get_item(fname).is_none() {
            self.functions.push(Item::Routine(f));
            Ok(())
        } else {
            Err(format!("{} already exists in module", fname))
        }
    }

    pub fn add_coroutine(&mut self, c: RoutineDef<M>) -> Result<()> {
        let cname = c.get_name();
        if self.get_item(cname).is_none() {
            self.coroutines.push(Item::Routine(c));
            Ok(())
        } else {
            Err(format!("{} already exists in module", cname))
        }
    }

    pub fn add_struct(&mut self, s: StructDef<M>) -> Result<()> {
        let name = s.get_name();
        if self.get_item(name).is_none() {
            self.structs.push(Item::Struct(s));
            Ok(())
        } else {
            Err(format!("{} already exists in module", name))
        }
    }

    pub fn add_extern(&mut self, e: Extern<M>) -> Result<()> {
        let name = e.get_name();
        if self.get_item(name).is_none() {
            self.externs.push(Item::Extern(e));
            Ok(())
        } else {
            Err(format!("{} already exists in module", name))
        }
    }

    pub fn add_item(&mut self, i: Item<M>) -> Result<()> {
        match i {
            Item::Routine(r) => {
                if *r.get_def() == RoutineDefType::Function {
                    self.add_function(r)
                } else {
                    self.add_coroutine(r)
                }
            }
            Item::Struct(s) => self.add_struct(s),
            Item::Extern(e) => self.add_extern(e),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_modules(&self) -> &Vec<Module<M>> {
        &self.modules
    }

    pub fn get_modules_mut(&mut self) -> &mut Vec<Module<M>> {
        &mut self.modules
    }

    pub fn get_functions(&self) -> &Vec<Item<M>> {
        &self.functions
    }

    pub fn get_functions_mut(&mut self) -> &mut Vec<Item<M>> {
        &mut self.functions
    }

    pub fn get_coroutines(&self) -> &Vec<Item<M>> {
        &self.coroutines
    }

    pub fn get_coroutines_mut(&mut self) -> &mut Vec<Item<M>> {
        &mut self.coroutines
    }

    pub fn get_structs(&self) -> &Vec<Item<M>> {
        &self.structs
    }

    pub fn get_structs_mut(&mut self) -> &mut Vec<Item<M>> {
        &mut self.structs
    }

    pub fn get_externs(&self) -> &Vec<Item<M>> {
        &self.externs
    }

    pub fn get_externs_mut(&mut self) -> &mut Vec<Item<M>> {
        &mut self.externs
    }

    pub fn get_module(&self, name: &str) -> Option<&Module<M>> {
        self.modules.iter().find(|m| m.name == name)
    }

    pub fn get_module_mut(&mut self, name: &str) -> Option<&mut Module<M>> {
        self.modules.iter_mut().find(|m| m.name == name)
    }

    pub fn get_item(&self, name: &str) -> Option<&Item<M>> {
        self.functions.iter().find(|f| f.get_name() == name).or(self
            .coroutines
            .iter()
            .find(|c| c.get_name() == name)
            .or(self
                .structs
                .iter()
                .find(|c| c.get_name() == name)
                .or(self.externs.iter().find(|e| e.get_name() == name))))
    }

    // TODO: This function appears to only be used in tests, so I believe it can be
    // deleted
    /*pub fn go_to(&self, path: &Path) -> Option<&Item<M>> {
        // If the path is empty, then return None as it is not possible for
        // anything to match
        if path.len() == 0 {
            None
        }
        // If path has one element, then that is the item name
        // and return the matching item
        else if path.len() == 1 {
            path.item().and_then(|item| self.get_item(item))
        } else {
            // otherwise, get the parent of the path and traverse the
            // module hierarchy by the parent, returning None if at
            // any point no module matches the parent path

            // If the parent path terminates on a module, then get
            // the item from the path (the last element in teh path)
            // and search the terminating module for that item and
            // return the result
            let parent_path = path.parent();
            match self.go_to_module(&parent_path) {
                Some(parent) => {
                    let item = path.item().expect("Path with >1 length has no item");
                    parent.get_item(item)
                }
                None => None,
            }
        }
    }*/

    pub fn go_to_module(&self, path: &Path) -> Option<&Module<M>> {
        if path.len() == 0 {
            None
        } else {
            // check to make sure that the first step in the path
            // is this module, and then use the path to traverse
            // through descendent modules
            if self.name == path[0] {
                let mut current = self;
                for idx in 1..path.len() {
                    match current.get_module(&path[idx]) {
                        Some(m) => current = m,
                        None => return None,
                    }
                }
                Some(current)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::routinedef::{RoutineDef, RoutineDefType};
    use crate::ast::ty::Type;

    use super::*;

    #[test]
    pub fn test_new_module() {
        let module = Module::new("test", 1);
        assert_eq!(module.get_name(), "test");
        assert_eq!(*module.annotation(), 1);
    }

    #[test]
    pub fn test_get_nonexistant_item() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let f = module.get_item("not_found");
        assert_eq!(f, None);
    }

    #[test]
    pub fn test_add_function() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let f = module.get_item("func");
        assert_eq!(f, Some(&Item::Routine(fdef)));
    }

    #[test]
    pub fn test_add_function_that_already_exists() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let result = module.add_function(fdef.clone());
        assert_eq!(result, Err("func already exists in module".into()));
    }

    #[test]
    pub fn test_add_coroutine() {
        let mut module = Module::new("test", 1);
        let cdef = RoutineDef {
            annotations: 1,
            name: "cor".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_coroutine(cdef.clone()).unwrap();
        let c = module.get_item("cor").unwrap();
        assert_eq!(c, &Item::Routine(cdef));
    }

    #[test]
    pub fn test_add_coroutine_that_already_exists() {
        let mut module = Module::new("test", 1);
        let cdef = RoutineDef {
            annotations: 1,
            name: "cor".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_coroutine(cdef.clone()).unwrap();
        let result = module.add_coroutine(cdef.clone());
        assert_eq!(result, Err("cor already exists in module".into()));
    }

    #[test]
    pub fn test_add_coroutine_with_same_name_as_function() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();

        let cdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        let result = module.add_coroutine(cdef.clone());
        assert_eq!(result, Err("dupe already exists in module".into()));
    }

    #[test]
    pub fn test_add_function_with_same_name_as_coroutine() {
        let mut module = Module::new("test", 1);
        let cdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_coroutine(cdef.clone()).unwrap();

        let fdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        let result = module.add_function(fdef.clone());
        assert_eq!(result, Err("dupe already exists in module".into()));
    }

    /*#[test]
    pub fn test_go_to_item_does_not_exist() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let f = module.go_to(&vec!["test", "nothing"].into());
        assert_eq!(f, None);
    }

    #[test]
    pub fn test_go_to_root_does_not_match() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let f = module.go_to(&vec!["bad", "func"].into());
        assert_eq!(f, None);
    }

    #[test]
    pub fn test_go_to_function() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "func".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();
        let f = module.go_to(&vec!["test", "func"].into());
        assert_eq!(f, Some(&Item::Routine(fdef)));
    }

    #[test]
    pub fn test_go_to_coroutine() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "co".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_coroutine(fdef.clone()).unwrap();
        let f = module.go_to(&vec!["test", "co"].into());
        assert_eq!(f, Some(&Item::Routine(fdef)));
    }

    #[test]
    pub fn test_go_to_nested() {
        let mut module = Module::new("inner", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "co".into(),
            def: RoutineDefType::Coroutine,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_coroutine(fdef.clone()).unwrap();
        let mut outer = Module::new("outer", 2);
        outer.add_module(module.clone());
        let f = outer.go_to(&vec!["outer", "inner", "co"].into());
        assert_eq!(f, Some(&Item::Routine(fdef)));
    }*/

    #[test]
    pub fn test_add_extern() {
        let mut module = Module::new("test", 1);
        let edef = Extern::new("puts", 1, vec![], Type::Unit);
        module.add_extern(edef.clone()).unwrap();
        let c = module.get_item("puts").unwrap();
        assert_eq!(c, &Item::Extern(edef));
    }

    #[test]
    pub fn test_add_extern_that_already_exists() {
        let mut module = Module::new("test", 1);
        let edef = Extern::new("puts", 1, vec![], Type::Unit);
        module.add_extern(edef.clone()).unwrap();
        let result = module.add_extern(edef.clone());
        assert_eq!(result, Err("puts already exists in module".into()));
    }

    #[test]
    pub fn test_add_extern_with_same_name_as_function() {
        let mut module = Module::new("test", 1);
        let fdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        module.add_function(fdef.clone()).unwrap();

        let edef = Extern::new("dupe", 1, vec![], Type::Unit);
        let result = module.add_extern(edef.clone());
        assert_eq!(result, Err("dupe already exists in module".into()));
    }

    #[test]
    pub fn test_add_function_with_same_name_as_extern() {
        let mut module = Module::new("test", 1);
        let edef = Extern::new("dupe", 1, vec![], Type::Unit);
        module.add_extern(edef.clone()).unwrap();

        let fdef = RoutineDef {
            annotations: 1,
            name: "dupe".into(),
            def: RoutineDefType::Function,
            params: vec![],
            ty: Type::I64,
            body: vec![],
        };
        let result = module.add_function(fdef.clone());
        assert_eq!(result, Err("dupe already exists in module".into()));
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item<M> {
    Routine(RoutineDef<M>),
    Struct(StructDef<M>),
    Extern(Extern<M>),
}

impl<M: Annotation> Node<M> for Item<M> {
    fn annotation(&self) -> &M {
        match self {
            Item::Routine(r) => r.annotation(),
            Item::Struct(s) => s.annotation(),
            Item::Extern(e) => e.annotation(),
        }
    }

    fn annotation_mut(&mut self) -> &mut M {
        match self {
            Item::Routine(r) => r.annotation_mut(),
            Item::Struct(s) => s.annotation_mut(),
            Item::Extern(e) => e.annotation_mut(),
        }
    }

    fn node_type(&self) -> NodeType {
        match self {
            Item::Routine(r) => r.node_type(),
            Item::Struct(s) => s.node_type(),
            Item::Extern(e) => e.node_type(),
        }
    }

    fn children(&self) -> Vec<&dyn Node<M>> {
        match self {
            Item::Routine(r) => r.children(),
            Item::Struct(s) => s.children(),
            Item::Extern(e) => e.children(),
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Item::Routine(r) => r.name(),
            Item::Struct(s) => s.name(),
            Item::Extern(e) => e.name(),
        }
    }

    fn iter_postorder(&self) -> PostOrderIter<M> {
        PostOrderIter::new(self)
    }

    fn iter_preorder(&self) -> PreOrderIter<M> {
        PreOrderIter::new(self)
    }
}

impl<M> std::fmt::Display for Item<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.write_str(self.get_name())
    }
}

impl<M> Item<M> {
    pub fn get_name(&self) -> &str {
        match self {
            Item::Routine(r) => r.get_name(),
            Item::Struct(s) => s.get_name(),
            Item::Extern(e) => e.get_name(),
        }
    }

    pub fn to_routine(&self) -> Option<&RoutineDef<M>> {
        match self {
            Item::Routine(r) => Some(r),
            Item::Struct(_) => None,
            Item::Extern(_) => None,
        }
    }

    pub fn root_str(&self) -> String {
        match self {
            Item::Routine(r) => format!("{} {}", r.get_def(), r.get_name()),
            Item::Struct(s) => s.root_str(),
            Item::Extern(e) => e.root_str(),
        }
    }
}
