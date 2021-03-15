#![allow(unused_imports, unused_variables)]

/// The compiler traverses the Braid AST and constructs and constructs
/// an LLVM Module through LLVM IR.

/// This uses the LLVM C API to interface with LLVM and construct the
/// Module. Resulting IR can then be fed into the LLVM Compiler to compile
/// into native assembly or into a JIT.
use std::{collections::HashMap, error::Error};

use ast::Expression;
use inkwell::{
    builder::Builder,
    context::Context,
    execution_engine::{ExecutionEngine, JitFunction},
    module::Module,
    targets::{InitializationConfig, Target},
    types::*,
    values::*,
    AddressSpace, IntPredicate, OptimizationLevel,
};

use crate::{
    ast,
    ast::{Node, RoutineDef, StructDef},
    compiler::memory::stringpool::StringPool,
    semantics::semanticnode::SemanticAnnotations,
};

use super::scopestack::RegisterLookup;

/// A LLVM IR generator which can be used to generate all the code
/// for a single LLVM Module.
pub struct IrGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    externs: &'ctx Vec<(crate::ast::Path, Vec<crate::ast::Type>, crate::ast::Type)>,
    string_pool: StringPool,
    registers: RegisterLookup<'ctx>,
    struct_table: HashMap<String, &'ctx StructDef<SemanticAnnotations>>,
}

impl<'ctx> IrGen<'ctx> {
    pub fn new(
        ctx: &'ctx Context,
        module: &str,
        externs: &'ctx Vec<(crate::ast::Path, Vec<crate::ast::Type>, crate::ast::Type)>,
    ) -> IrGen<'ctx> {
        IrGen {
            context: ctx,
            module: ctx.create_module(module),
            builder: ctx.create_builder(),
            externs,
            string_pool: StringPool::new(),
            registers: RegisterLookup::new(),
            struct_table: HashMap::new(),
        }
    }

    /// Print the LLVM IR to stderr
    pub fn print_err(&self) {
        self.module.print_to_stderr();
    }

    /// Print the LLVM IR to the given file
    pub fn print(&self, path: &std::path::Path) {
        self.module.print_to_file(path).unwrap()
    }

    /// Take the given Braid AST to compile it to LLVM IR and add it to the LLVM module.
    ///
    /// All user input is expected to be fully validated and correct by the time it reaches
    /// the compiler phase (via syntactic and semantic analysis).  Therefore, if anything
    /// goes wrong during compilation, it is assumed to be the result of a critical bug in
    /// the compiler itself and not an issue with the input Braid code. This means that any
    /// error at this stage is unrecoverable; since its a bug in the compiler itself it cannot
    /// be trusted. So, if any unexpected state is encountered or any error happens this module
    /// will panic at that point in code and crash the compiler.
    pub fn ingest(&mut self, m: &'ctx crate::ast::Module<SemanticAnnotations>) {
        self.compile_string_pool(m);
        self.add_externs();
        self.add_mod_items(m);
        self.create_main();
        match m.to_llvm_ir(self) {
            None => (),
            Some(_) => panic!("Expected None when compiling a Module"),
        }
    }

    /// Creates `main` entry point which will be called by the OS to start the Braid
    /// application. This main will initialize platform level values and state, then
    /// call the user defined main `my_main`.
    fn create_main(&self) {
        let main_type = self.context.i64_type().fn_type(&[], false);
        let main = self.module.add_function("main", main_type, None);
        let entry_bb = self.context.append_basic_block(main, "entry");
        self.builder.position_at_end(entry_bb);
        let user_main = self.module.get_function("root_my_main").unwrap();
        let status = self
            .builder
            .build_call(user_main, &[], "user_main")
            .try_as_basic_value()
            .left()
            .unwrap();
        self.builder.build_return(Some(&status));
    }

    /// Add the list of external function declarations to the function table
    /// in the LLVM module
    fn add_externs(&self) {
        for (path, params, ty) in self.externs {
            self.add_extern_decl(&path.to_label(), params, ty)
        }
    }

    /// Take the given AST and add declarations for every function to the
    /// LLVM module. This is required so that the FunctionValue can be looked
    /// up when generating code for function calls.
    fn add_mod_items(&mut self, m: &'ctx crate::ast::Module<SemanticAnnotations>) {
        for s in m.get_structs() {
            if let crate::ast::Item::Struct(sd) = s {
                self.add_struct_def(sd);
            } else {
                panic!("Expected a struct but got {}", s)
            }
        }

        for f in m.get_functions() {
            if let crate::ast::Item::Routine(rd) = f {
                self.add_fn_decl(rd);
            }
        }

        for m in m.get_modules() {
            self.add_mod_items(m);
        }
    }

    /// Takes a RoutineDef and adds its declaration to the
    /// LLVM Module. This function declaration can then be
    /// looked up through `self.module` for function calls
    /// and to add the definition to the function when
    /// compiling the AST to LLVM.
    fn add_fn_decl(&self, rd: &'ctx RoutineDef<SemanticAnnotations>) {
        let ty = rd.ty.to_llvm(self);
        let mut params = vec![];
        for p in rd.get_params() {
            let mut p_ty = p.ty.to_llvm(self);

            // Pass structs around by reference
            match p_ty {
                BasicTypeEnum::StructType(st_ty) => p_ty = st_ty.into(),
                _ => {}
            }

            params.push(p_ty);
        }

        let fn_type = match ty {
            BasicTypeEnum::IntType(ity) => ity.fn_type(&params, false),
            BasicTypeEnum::PointerType(pty) => pty.fn_type(&params, false),
            _ => panic!(),
        };
        let fn_name = rd.annotations.get_canonical_path().to_label();
        self.module.add_function(&fn_name, fn_type, None);
    }

    /// Takes a tuple describing the signature of an extern and adds its declaration to the
    /// LLVM Module. This function declaration can then be
    /// looked up through `self.module` for function calls
    /// and to add the definition to the function when
    /// compiling the AST to LLVM.
    fn add_extern_decl(
        &self,
        name: &str,
        params: &Vec<crate::ast::Type>,
        ret_ty: &crate::ast::Type,
    ) {
        let llvm_ty = ret_ty.to_llvm(self);
        let mut llvm_params = vec![];
        for p in params {
            llvm_params.push(p.to_llvm(self))
        }
        let fn_type = match llvm_ty {
            BasicTypeEnum::IntType(ity) => ity.fn_type(&llvm_params, false),
            _ => panic!(),
        };
        self.module.add_function(name, fn_type, None);
    }

    /// Add a struct definition to the LLVM context
    fn add_struct_def(&mut self, sd: &'ctx StructDef<SemanticAnnotations>) {
        self.struct_table
            .insert(sd.annotation().get_canonical_path().to_label(), sd);
        let name = sd.annotation().get_canonical_path().to_label();
        let fields_llvm: Vec<BasicTypeEnum<'ctx>> =
            sd.get_fields().iter().map(|f| f.ty.to_llvm(self)).collect();
        let struct_ty = self.context.opaque_struct_type(&name);
        struct_ty.set_body(&fields_llvm, false);
    }

    /// Add all string literals to the data section of the assemby output
    fn compile_string_pool(&mut self, m: &crate::ast::Module<SemanticAnnotations>) {
        self.string_pool.extract_from_module(m);

        for (s, id) in self.string_pool.pool.iter() {
            let escaped_s = s; //str_to_llvm(s);
            let len_w_null = escaped_s.len() + 1;
            let g = self.module.add_global(
                self.context.i8_type().array_type(len_w_null as u32),
                None,
                &Self::id_to_str_pool_var(*id),
            );
            g.set_initializer(&self.context.const_string(escaped_s.as_bytes(), true));
        }
    }

    /// Will look for `s` in the string pool, if found, it will return the
    /// name of the global variable that is bound to that string. Otherwise,
    /// it will return `None`
    fn get_str_var(&self, s: &str) -> Option<String> {
        self.string_pool
            .get(s)
            .map(|id| Self::id_to_str_pool_var(*id))
    }

    /// Convert the ID of a string to the name of the global variable that
    /// references that string
    fn id_to_str_pool_var(id: usize) -> String {
        format!("str_{}", id)
    }
}

trait ToLlvmIr<'ctx> {
    type Value: inkwell::values::AnyValue<'ctx>;

    /// Compile a Language unit to LLVM and return the appropriate LLVM Value
    /// if it has one (Modules don't have LLVM Values so those will return None)
    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value>;
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::Module<SemanticAnnotations> {
    type Value = FunctionValue<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        for m in self.get_modules() {
            m.to_llvm_ir(llvm);
        }
        for s in self.get_structs() {}
        for f in self.get_functions() {
            if let crate::ast::Item::Routine(rdef) = f {
                let fn_val = rdef
                    .to_llvm_ir(llvm)
                    .expect("Expected Function Value from RoutineDef");
            }
        }
        for c in self.get_coroutines() {}

        None
    }
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::RoutineDef<SemanticAnnotations> {
    type Value = FunctionValue<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        let fn_value = llvm
            .module
            .get_function(&self.annotations.get_canonical_path().to_label())
            .expect("Could not find function");
        let entry_bb = llvm.context.append_basic_block(fn_value, "entry");
        llvm.builder.position_at_end(entry_bb);

        llvm.registers.open_fn().unwrap();
        let llvm_params = fn_value.get_params();
        let num_params = self.get_params().len();
        for pi in 0..num_params {
            let pname = &(*self.get_params())[pi].name;

            // move parameter into the stack
            let pptr = llvm.builder.build_alloca(llvm_params[pi].get_type(), pname);
            llvm.builder.build_store(pptr, llvm_params[pi]);
            llvm.registers.insert(pname, pptr.into()).unwrap();
        }

        // Compile the body to LLVM
        for stm in &self.body {
            let value = stm.to_llvm_ir(llvm);
        }

        llvm.registers.close_fn().unwrap();

        Some(fn_value)
    }
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::Statement<SemanticAnnotations> {
    type Value = AnyValueEnum<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        match self {
            crate::ast::Statement::Return(ret) => ret.to_llvm_ir(llvm).map(|i| i.into()),
            crate::ast::Statement::Expression(exp) => exp.to_llvm_ir(llvm).map(|v| v.into()),
            crate::ast::Statement::Bind(bind) => bind.to_llvm_ir(llvm).map(|i| i.into()),
            _ => None,
        }
    }
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::Bind<SemanticAnnotations> {
    type Value = PointerValue<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        let ptr = llvm
            .builder
            .build_alloca(self.get_type().to_llvm(llvm), self.get_id());
        let rhs = self.get_rhs().to_llvm_ir(llvm).unwrap();
        llvm.builder.build_store(ptr, rhs);
        llvm.registers.insert(self.get_id(), ptr.into()).unwrap();
        Some(ptr)
    }
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::Return<SemanticAnnotations> {
    type Value = InstructionValue<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        Some(match self.get_value() {
            None => llvm.builder.build_return(None),
            Some(val) => {
                let val = val
                    .to_llvm_ir(llvm)
                    .expect("Return expression did not compile to an LLVM value");
                llvm.builder.build_return(Some(&val))
            }
        })
    }
}

impl<'ctx> ToLlvmIr<'ctx> for crate::ast::Expression<SemanticAnnotations> {
    type Value = BasicValueEnum<'ctx>;

    fn to_llvm_ir(&self, llvm: &mut IrGen<'ctx>) -> Option<Self::Value> {
        match self {
            crate::ast::Expression::Integer32(_, i) => {
                let i32t = llvm.context.i32_type();
                Some(i32t.const_int(*i as u64, true).into())
            }
            crate::ast::Expression::Integer64(_, i) => {
                let i64t = llvm.context.i64_type();
                Some(i64t.const_int(*i as u64, true).into())
            }
            crate::ast::Expression::Boolean(_, b) => {
                let bt = llvm.context.bool_type();
                Some(bt.const_int(*b as u64, false).into())
            }
            crate::ast::Expression::StringLiteral(_, s) => {
                let str_id = llvm.get_str_var(s).unwrap();
                let val = llvm.module.get_global(&str_id).unwrap();
                let val_ptr = val.as_pointer_value();
                let bitcast = llvm.builder.build_bitcast(
                    val_ptr,
                    llvm.context
                        .i8_type()
                        .array_type(0)
                        .ptr_type(AddressSpace::Generic),
                    "sptr",
                );
                Some(bitcast.into())
            }
            crate::ast::Expression::Identifier(_, id) => {
                let ptr = llvm.registers.get(id).unwrap().into_pointer_value();
                let val = llvm.builder.build_load(ptr, id);
                Some(val)
            }
            crate::ast::Expression::UnaryOp(_, op, exp) => {
                let v = exp.to_llvm_ir(llvm).expect("Expected a value");
                Some(op.to_llvm(llvm, v))
            }
            crate::ast::Expression::BinaryOp(_, op, l, r) => {
                let left = l.to_llvm_ir(llvm).expect("Expected a value");
                let right = r.to_llvm_ir(llvm).expect("Expected a value");
                Some(op.to_llvm(llvm, left, right))
            }
            crate::ast::Expression::RoutineCall(_, call, name, params) => {
                let llvm_params: Vec<BasicValueEnum<'ctx>> =
                    params.iter().map(|p| p.to_llvm_ir(llvm).unwrap()).collect();
                let call = llvm.module.get_function(&name.to_label()).unwrap();
                let result = llvm.builder.build_call(call, &llvm_params, "result");
                let result_bv = result.try_as_basic_value().left().unwrap();
                Some(result_bv)
            }
            crate::ast::Expression::ExpressionBlock(_, stmts, exp) => {
                llvm.registers.open_local().unwrap();
                for stmt in stmts {
                    stmt.to_llvm_ir(llvm).unwrap();
                }
                let val = exp.as_ref().map(|e| e.to_llvm_ir(llvm)).flatten();
                llvm.registers.close_local().unwrap();
                val
            }
            crate::ast::Expression::If {
                cond,
                if_arm,
                else_arm,
                ..
            } => {
                let cond_val = cond.to_llvm_ir(llvm).unwrap().into_int_value();
                let current_fn = llvm
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let then_bb = llvm.context.append_basic_block(current_fn, "then");
                let else_bb = llvm.context.insert_basic_block_after(then_bb, "else");
                let merge_bb = llvm.context.insert_basic_block_after(else_bb, "merge");
                llvm.builder
                    .build_conditional_branch(cond_val, then_bb, else_bb);

                llvm.builder.position_at_end(then_bb);
                let if_arm_val = if_arm.to_llvm_ir(llvm);
                llvm.builder.build_unconditional_branch(merge_bb);

                llvm.builder.position_at_end(else_bb);
                let else_arm_val = else_arm.as_ref().map(|ea| ea.to_llvm_ir(llvm).unwrap());
                llvm.builder.build_unconditional_branch(merge_bb);

                llvm.builder.position_at_end(merge_bb);

                match (if_arm_val, else_arm_val) {
                    (Some(if_arm_val), Some(else_arm_val)) => {
                        // create phi to unify the branches
                        let phi = llvm.builder.build_phi(if_arm_val.get_type(), "phi");
                        phi.add_incoming(&[(&if_arm_val, then_bb), (&else_arm_val, else_bb)]);
                        Some(phi.as_basic_value())
                    }
                    (None, None) => None,
                    _ => panic!(
                        "Mismatching arms on if expression: {:?}, {:?}",
                        if_arm_val, else_arm_val
                    ),
                }
            }
            ast::Expression::MemberAccess(_, val, field) => {
                let sdef = llvm
                    .struct_table
                    .get(&val.get_type().get_path().unwrap().to_label())
                    .unwrap();
                let field_idx = sdef.get_field_idx(field).unwrap();

                let zero = llvm.context.i64_type().const_int(0, true);
                let field_idx_llvm = llvm.context.i64_type().const_int(field_idx as u64, true);

                let val_llvm = val.to_llvm_ir(llvm).unwrap().into_pointer_value();
                let field_ptr = llvm
                    .builder
                    .build_struct_gep(val_llvm, field_idx as u32, "")
                    .unwrap();
                let field_val = llvm.builder.build_load(field_ptr, "");
                Some(field_val)
            }
            ast::Expression::StructExpression(_, name, fields) => {
                let sname = self.annotation().ty().get_path().unwrap().to_label();
                let sdef = llvm
                    .struct_table
                    .get(&sname)
                    .expect(&format!("Cannot find {} in {:?}", sname, llvm.struct_table));
                let sd_llvm = llvm.module.get_struct_type(&sname).unwrap();
                let s_ptr = llvm.builder.build_alloca(sd_llvm, "");

                // convert field names to field indexes (order of fields in expression may not
                // be the same as in the defintion)
                let idx_fields: Vec<(usize, &ast::Expression<SemanticAnnotations>)> = fields
                    .iter()
                    .map(|(n, v)| (sdef.get_field_idx(n).unwrap(), v))
                    .collect();

                for (f_idx, e) in idx_fields {
                    let val = e.to_llvm_ir(llvm).unwrap();
                    let f_ptr = llvm
                        .builder
                        .build_struct_gep(s_ptr, f_idx as u32, "")
                        .unwrap();
                    llvm.builder.build_store(f_ptr, val);
                }
                Some(s_ptr.into())
            }
            _ => todo!("{} not implemented yet", self),
            /*
            crate::ast::Expression::CustomType(_, _) => {}
            crate::ast::Expression::Path(_, _) => {}
            crate::ast::Expression::MemberAccess(_, _, _) => {}
            crate::ast::Expression::IdentifierDeclare(_, _, _) => {}
            crate::ast::Expression::StructExpression(_, _, _) => {}
            crate::ast::Expression::Yield(_, _) => {}
            */
        }
    }
}

impl crate::ast::UnaryOperator {
    fn to_llvm<'ctx>(
        &self,
        llvm: &IrGen<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let rv = right.into_int_value();
        match self {
            crate::ast::UnaryOperator::Minus => llvm.builder.build_int_neg(rv, "").into(),
            crate::ast::UnaryOperator::Not => llvm.builder.build_not(rv, "").into(),
        }
    }
}

impl crate::ast::BinaryOperator {
    fn to_llvm<'ctx>(
        &self,
        llvm: &IrGen<'ctx>,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let lv = left.into_int_value();
        let rv = right.into_int_value();

        match self {
            crate::ast::BinaryOperator::Add => llvm.builder.build_int_add(lv, rv, "").into(),
            crate::ast::BinaryOperator::Sub => llvm.builder.build_int_sub(lv, rv, "").into(),
            crate::ast::BinaryOperator::Mul => llvm.builder.build_int_mul(lv, rv, "").into(),
            crate::ast::BinaryOperator::Div => llvm.builder.build_int_signed_div(lv, rv, "").into(),
            crate::ast::BinaryOperator::BAnd => llvm.builder.build_and(lv, rv, "").into(),
            crate::ast::BinaryOperator::BOr => llvm.builder.build_or(lv, rv, "").into(),
            crate::ast::BinaryOperator::Eq => llvm
                .builder
                .build_int_compare(IntPredicate::EQ, lv, rv, "")
                .into(),
            crate::ast::BinaryOperator::NEq => llvm
                .builder
                .build_int_compare(IntPredicate::NE, lv, rv, "")
                .into(),
            crate::ast::BinaryOperator::Ls => llvm
                .builder
                .build_int_compare(IntPredicate::SLT, lv, rv, "")
                .into(),
            crate::ast::BinaryOperator::LsEq => llvm
                .builder
                .build_int_compare(IntPredicate::SLE, lv, rv, "")
                .into(),
            crate::ast::BinaryOperator::Gr => llvm
                .builder
                .build_int_compare(IntPredicate::SGT, lv, rv, "")
                .into(),
            crate::ast::BinaryOperator::GrEq => llvm
                .builder
                .build_int_compare(IntPredicate::SGE, lv, rv, "")
                .into(),
        }
    }
}

impl crate::ast::Type {
    fn to_llvm<'ctx>(&self, llvm: &IrGen<'ctx>) -> BasicTypeEnum<'ctx> {
        match self {
            crate::ast::Type::I32 => llvm.context.i32_type().into(),
            crate::ast::Type::I64 => llvm.context.i64_type().into(),
            crate::ast::Type::Bool => llvm.context.bool_type().into(),
            crate::ast::Type::Unit => llvm.context.custom_width_int_type(1).into(),
            crate::ast::Type::StringLiteral => llvm
                .context
                .i8_type()
                .array_type(0)
                .ptr_type(AddressSpace::Generic)
                .into(),
            crate::ast::Type::Custom(name) => llvm
                .module
                .get_struct_type(&name.to_label())
                .expect(&format!("Could not find struct {}", name))
                .ptr_type(AddressSpace::Generic)
                .into(),
            _ => panic!("Can't convert type to LLVM: {}", self),
        }
    }
}
