//! Transforms the MIR representation into LLVM

use std::collections::{hash_map::Entry, HashMap};

use inkwell::{builder::Builder, context::Context, module::Module, values::*};

use crate::{
    compiler::{
        mir::{ir::*, Transformer},
        Span,
    },
    StringId, StringTable,
};

struct LlvmTransformer<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    table: &'ctx StringTable,
    function: FunctionValue<'ctx>,

    vars: HashMap<VarId, PointerValue<'ctx>>,
}

impl<'ctx> LlvmTransformer<'ctx> {
    pub fn new(
        func_name: StringId,
        ctx: &'ctx Context,
        module: &str,
        table: &'ctx StringTable,
    ) -> Self {
        let module = ctx.create_module(module);

        // Create a function to build
        let ft = ctx.void_type().fn_type(&[], false);
        let name = table.get(func_name).unwrap();
        let function = module.add_function(&name, ft, None);

        let builder = ctx.create_builder();

        Self {
            context: ctx,
            module,
            builder,
            table,
            function,
            vars: HashMap::default(),
        }
    }

    fn to_label(&self, vd: &VarDecl) -> String {
        let name = self.table.get(vd.name()).unwrap();
        let scope = vd.scope();
        format!("{}_{}", name, scope)
    }
}

impl<'ctx> Transformer<PointerValue<'ctx>, BasicValueEnum<'ctx>> for LlvmTransformer<'ctx> {
    fn start_bb(&mut self, bb: BasicBlockId) {
        let bb = self
            .context
            .append_basic_block(self.function, &bb.to_string());
        self.builder.position_at_end(bb);
    }

    fn add_var(&mut self, id: VarId, decl: &VarDecl) {
        let name = self.to_label(decl);

        // Check if variable name already exists
        match self.vars.entry(id) {
            // If it does -> return an error
            Entry::Occupied(_) => {
                panic!("Attempting to add a variable that is already in the table")
            }

            // If not, then allocate a pointer in the Builder
            Entry::Vacant(ve) => {
                // and add a mapping from VarID to the pointer in the local var table
                let ty = self.context.i64_type();
                let ptr = self.builder.build_alloca(ty, &name);
                ve.insert(ptr);
            }
        }
    }

    fn add_temp(&mut self) {
        todo!()
    }

    fn term_return(&mut self) {
        self.builder.build_return(None);
    }

    fn assign(&mut self, span: Span, l: PointerValue<'ctx>, v: BasicValueEnum<'ctx>) {
        self.builder.build_store(l, v);
    }

    fn var(&self, v: VarId) -> PointerValue<'ctx> {
        *self
            .vars
            .get(&v)
            .expect("Cound not find given VarId in vars table")
    }

    fn const_i64(&self, i: i64) -> BasicValueEnum<'ctx> {
        self.context.i64_type().const_int(i as u64, true).into()
    }

    fn load(&self, lv: PointerValue<'ctx>) -> BasicValueEnum<'ctx> {
        todo!()
    }

    fn add(&self, a: BasicValueEnum<'ctx>, b: BasicValueEnum<'ctx>) -> BasicValueEnum<'ctx> {
        todo!()
    }

    fn sub(&self, a: BasicValueEnum<'ctx>, b: BasicValueEnum<'ctx>) -> BasicValueEnum<'ctx> {
        todo!()
    }
}

#[cfg(test)]
mod mir2llvm_tests {
    use inkwell::context::Context;

    use crate::{
        compiler::{
            ast::{Module, MAIN_MODULE},
            diagnostics::Logger,
            lexer::{tokens::Token, LexerError},
            mir::{transform, MirProject, Traverser},
            parser::Parser,
            semantics::semanticnode::SemanticContext,
            CompilerDisplay, CompilerError, Lexer, SourceMap,
        },
        resolve_types, StringId, StringTable,
    };

    use super::LlvmTransformer;

    #[test]
    fn print_llvm() {
        let text = "
            fn test() {
                let x: i64 := 5;
                return;
            }
        ";

        let (table, module) = compile(text);
        let mut project = MirProject::new();
        transform::transform(&module, &mut project).unwrap();

        let context = Context::create();
        let mut llvm = LlvmTransformer::new(StringId::new(), &context, "test", &table);

        let mut mvr = Traverser::new(&project, &mut llvm);

        // Traverser is given a MirProject
        // call traverser.map(llvm) this will use the llvm xfmr to map MirProject to LlvmProject
        mvr.map();
        // Print LLVM
        llvm.module.print_to_stderr();
    }

    type LResult = std::result::Result<Vec<Token>, CompilerError<LexerError>>;

    fn compile(input: &str) -> (StringTable, Module<SemanticContext>) {
        let table = StringTable::new();
        let mut sm = SourceMap::new();
        sm.add_string(input, "/test".into()).unwrap();
        let src = sm.get(0).unwrap().read().unwrap();

        let main = table.insert("main".into());
        let main_mod = table.insert(MAIN_MODULE.into());
        let main_fn = table.insert("my_main".into());

        let logger = Logger::new();
        let tokens: Vec<Token> = Lexer::new(src, &table, &logger)
            .unwrap()
            .tokenize()
            .into_iter()
            .collect::<LResult>()
            .unwrap();

        let parser = Parser::new(&logger);
        let ast = match parser.parse(main, &tokens) {
            Ok(ast) => ast.unwrap(),
            Err(err) => {
                panic!("{}", err.fmt(&sm, &table).unwrap());
            }
        };
        match resolve_types(&ast, main_mod, main_fn, &logger) {
            Ok(module) => (table, module),
            Err(err) => {
                panic!("{}", err.fmt(&sm, &table).unwrap());
            }
        }
    }
}
