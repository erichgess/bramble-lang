use crate::{
    compiler::{ast::*, parser::parser::ParserContext},
    diagnostics::{Diag, DiagData},
};
use crate::{diagnostics::config::TracingConfig, StringId};

use super::{error::SemanticError, symbol_table::SymbolTable};

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticContext {
    id: u32,
    ln: u32,
    ty: Type,
    sym: SymbolTable,
    canonical_path: Path,
}

impl Context for SemanticContext {
    fn id(&self) -> u32 {
        self.id
    }

    fn line(&self) -> u32 {
        self.ln
    }
}

impl Diag for SemanticContext {
    fn diag(&self) -> DiagData {
        let mut dd = DiagData::new(self.ln, self.id);

        if self.sym.size() > 0 {
            dd.add("sym", &format!("{}", self.sym));
        }

        if self.canonical_path.len() > 0 {
            dd.add("canon path", &format!("{}", self.canonical_path));
        }

        dd
    }
}

pub type SemanticNode = Expression<SemanticContext>;

impl SemanticNode {
    pub fn get_type(&self) -> &Type {
        let meta = self.get_context();
        &meta.ty
    }
}

impl Statement<SemanticContext> {
    pub fn get_type(&self) -> &Type {
        let m = self.get_context();
        &m.ty
    }
}

impl SemanticContext {
    pub fn new_local(id: u32, ln: u32, ty: Type) -> SemanticContext {
        SemanticContext {
            id,
            ln,
            ty,
            sym: SymbolTable::new(),
            canonical_path: Path::new(),
        }
    }

    pub fn new_routine(id: u32, ln: u32, name: StringId, ty: Type) -> SemanticContext {
        SemanticContext {
            id,
            ln,
            ty,
            sym: SymbolTable::new_routine(name),
            canonical_path: Path::new(),
        }
    }

    pub fn new_module(id: u32, ln: u32, name: StringId, ty: Type) -> SemanticContext {
        SemanticContext {
            id,
            ln,
            ty,
            sym: SymbolTable::new_module(name),
            canonical_path: Path::new(),
        }
    }

    /// Creates a copy of this instances of [`SemanticContext`] but the [`Type`]
    /// field is assigned the value in `ty`
    pub fn with_type(&self, ty: Type) -> SemanticContext {
        let mut sm = self.clone();
        sm.ty = ty;
        sm
    }

    /// Creates a copy of this instances of [`SemanticContext`] but the [`SymbolTable`]
    /// field is assigned the value in `ty`
    pub fn with_sym(&self, sym: SymbolTable) -> SemanticContext {
        let mut sm = self.clone();
        sm.sym = sym;
        sm
    }

    /// Get the [`SymbolTable`] for a node in the AST.
    pub fn sym(&self) -> &SymbolTable {
        &self.sym
    }

    /// Get the [`Type`] for a node in the AST
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn canonical_path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn set_canonical_path(&mut self, path: Path) {
        self.canonical_path = path;
    }

    pub fn add_symbol(
        &mut self,
        name: StringId,
        ty: Type,
        mutable: bool,
        is_extern: bool,
    ) -> Result<(), SemanticError> {
        self.sym.add(name, ty, mutable, is_extern)
    }
}

pub struct SemanticAst {
    next_id: u32,
    tracing: TracingConfig,
}

impl SemanticAst {
    pub fn new() -> SemanticAst {
        SemanticAst {
            next_id: 0,
            tracing: TracingConfig::Off,
        }
    }

    pub fn from_module(
        &mut self,
        m: &Module<ParserContext>,
        tracing: TracingConfig,
    ) -> Module<SemanticContext> {
        let f = |n: &dyn Node<u32>| match n.node_type() {
            NodeType::Module => {
                let name = n.name().expect("Modules must have a name");
                self.module_semantic_context_from(*n.get_context(), name)
            }
            NodeType::RoutineDef(_) => {
                let name = n.name().expect("RoutineDefs must have a name");
                self.routine_semantic_context_from(*n.get_context(), name)
            }
            _ => self.semantic_context_from(*n.get_context()),
        };

        let mut mapper = MapPreOrder::new("parser-to-semantic", f, tracing);
        mapper.apply(m)
    }

    fn semantic_context_from(&mut self, ln: u32) -> SemanticContext {
        let sm_data = SemanticContext::new_local(self.next_id, ln, Type::Unknown);
        self.next_id += 1;
        sm_data
    }

    fn routine_semantic_context_from(&mut self, ln: u32, name: StringId) -> SemanticContext {
        let sm_data = SemanticContext::new_routine(self.next_id, ln, name, Type::Unknown);
        self.next_id += 1;
        sm_data
    }

    fn module_semantic_context_from(&mut self, ln: u32, name: StringId) -> SemanticContext {
        let sm_data = SemanticContext::new_module(self.next_id, ln, name, Type::Unknown);
        self.next_id += 1;
        sm_data
    }
}
