//! Converts the Bramble AST to the CFG MIR representation used for
//! dataflow analyses; such as, lifetime checking, variable initialization,
//! consistency rules checking, and so on.

// Transformer
// This process takes the AST for a compilation unit and transforms it into the
// CFG MIR used for dataflow analysis and LLVM IR generation by the Bramble
// compiler.

use log::debug;

use crate::{
    compiler::{
        ast::{
            BinaryOperator, Bind, Context, Expression, Module, Node, Return, RoutineDef, Statement,
            Type, UnaryOperator,
        },
        semantics::semanticnode::SemanticContext,
        source::Offset,
        Span,
    },
    StringId,
};

use super::ir::*;

pub fn module_transform(module: &Module<SemanticContext>) -> Vec<Procedure> {
    let funcs = module.get_functions();
    let mut mirs = vec![];

    for f in funcs {
        match f {
            crate::compiler::ast::Item::Routine(r) => {
                let ft = FuncTransformer::new();
                let p = ft.transform(r);
                mirs.push(p);
            }
            crate::compiler::ast::Item::Struct(_) => todo!(),
            crate::compiler::ast::Item::Extern(_) => todo!(),
        }
    }

    mirs
}

/// Provides a Builder interface for constructing the MIR CFG representation of a
/// routine. This will keep track of the current [`BasicBlock`] and make sure that
/// MIR operations are applied to that [`BasicBlock`]. This also provides a simplfied
/// interface for constructing the MIR operands, operations, and statements, to
/// simplify the code that traverses input ASTs and transforms them into MIR.
struct MirBuilder {
    proc: Procedure,
    current_bb: Option<BasicBlockId>,
}

impl MirBuilder {
    /// Creates a new [`MirBuilder`], which is used to construct the MIR representation
    /// of a function.
    pub fn new() -> MirBuilder {
        MirBuilder {
            proc: Procedure::new(&Type::Unit, Span::zero()),
            current_bb: None,
        }
    }

    /// Add a new [`BasicBlock`] to this function.
    fn new_bb(&mut self) -> BasicBlockId {
        self.proc.new_bb()
    }

    /// Change the active [`BasicBlock`]. After this call, all instructions added
    /// to the function will be appended to the [`BasicBlock`] specified by `bb`.
    fn set_bb(&mut self, bb: BasicBlockId) {
        self.current_bb = Some(bb)
    }

    fn find_var(&self, name: StringId) -> Option<VarId> {
        self.proc.find_var(name)
    }

    /// Create an [`i8`] constant
    fn const_i8(&mut self, i: i8) -> Operand {
        Operand::Constant(Constant::I8(i))
    }

    /// Create an [`i16`] constant
    fn const_i16(&mut self, i: i16) -> Operand {
        Operand::Constant(Constant::I16(i))
    }

    /// Create an [`i32`] constant
    fn const_i32(&mut self, i: i32) -> Operand {
        Operand::Constant(Constant::I32(i))
    }

    /// Create an [`i64`] constant
    fn const_i64(&mut self, i: i64) -> Operand {
        Operand::Constant(Constant::I64(i))
    }

    /// Create a [`u8`] constant
    fn const_u8(&mut self, i: u8) -> Operand {
        Operand::Constant(Constant::U8(i))
    }

    /// Create a [`u16`] constant
    fn const_u16(&mut self, i: u16) -> Operand {
        Operand::Constant(Constant::U16(i))
    }

    /// Create a [`u32`] constant
    fn const_u32(&mut self, i: u32) -> Operand {
        Operand::Constant(Constant::U32(i))
    }

    /// Create a [`u64`] constant
    fn const_u64(&mut self, i: u64) -> Operand {
        Operand::Constant(Constant::U64(i))
    }

    /// Create an [`f64`] constant
    fn const_f64(&mut self, f: f64) -> Operand {
        Operand::Constant(Constant::F64(f))
    }

    /// Create a [`bool`] constant
    fn const_bool(&mut self, b: bool) -> Operand {
        Operand::Constant(Constant::Bool(b))
    }

    /// Create a reference to a string literal
    fn const_stringliteral(&mut self, s: StringId) -> Operand {
        Operand::Constant(Constant::StringLiteral(s))
    }

    /// Create a `null` value
    fn const_null(&mut self) -> Operand {
        Operand::Constant(Constant::Null)
    }

    /// Add a new user declared variable to this function's stack
    fn var(&mut self, name: StringId, mutable: bool, ty: &Type, span: Span) -> VarId {
        self.proc.add_var(name, mutable, ty, ScopeId::new(0), span)
    }

    /// Add a new temporary variable to this function's stack
    fn temp(&mut self, ty: &Type) -> TempId {
        self.proc.add_temp(ty)
    }

    /// Create a new temporary variable and store the [`RValue`] in it.
    fn temp_store(&mut self, rv: RValue, ty: &Type, span: Span) -> Operand {
        let tv = LValue::Temp(self.temp(ty));
        debug!("Temp store: {:?} := {:?}", tv, rv);

        self.store(tv.clone(), rv, span);

        Operand::LValue(tv)
    }

    /// Store the given [`RValue`] in the location specified by the given
    /// [`LValue`].
    fn store(&mut self, lv: LValue, rv: RValue, span: Span) {
        let cid = self.current_bb.unwrap();
        let bb = self.proc.get_bb_mut(cid);
        bb.add_stm(super::ir::Statement::new(
            StatementKind::Assign(lv, rv),
            span,
        ));
    }

    fn not(&mut self, right: Operand) -> RValue {
        debug!("Not: {:?}", right);
        RValue::UnOp(UnOp::Not, right)
    }

    fn negate(&mut self, right: Operand) -> RValue {
        debug!("Negate: {:?}", right);
        RValue::UnOp(UnOp::Negate, right)
    }

    /// Add an addition operation to the current [`BasicBlock`].
    fn add(&mut self, left: Operand, right: Operand) -> RValue {
        debug!("Add: {:?}, {:?}", left, right);
        RValue::BinOp(BinOp::Add, left, right)
    }

    /// Add a subtraction operation to the current [`BasicBlock`].
    fn sub(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Sub");
        RValue::BinOp(BinOp::Sub, left, right)
    }

    /// Add a multiply operation to the current [`BasicBlock`].
    fn mul(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Mul");
        RValue::BinOp(BinOp::Mul, left, right)
    }

    /// Add a divide operation to the current [`BasicBlock`].
    fn div(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Div");
        RValue::BinOp(BinOp::Div, left, right)
    }

    /// Add a bitwise and operation to the current [`BasicBlock`].
    fn bitwise_and(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("And");
        RValue::BinOp(BinOp::And, left, right)
    }

    /// Add a bitwise and operation to the current [`BasicBlock`].
    fn bitwise_or(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Or");
        RValue::BinOp(BinOp::Or, left, right)
    }

    /// Add an equality test operation to the current [`BasicBlock`].
    fn eq(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Eq");
        RValue::BinOp(BinOp::Eq, left, right)
    }

    /// Add a not equal test operation to the current [`BasicBlock`].
    fn neq(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Neq");
        RValue::BinOp(BinOp::Ne, left, right)
    }

    /// Add a less than test operation to the current [`BasicBlock`].
    fn lt(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Less Than");
        RValue::BinOp(BinOp::Lt, left, right)
    }

    /// Add a less than or equal to test operation to the current [`BasicBlock`].
    fn le(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Less or Equal");
        RValue::BinOp(BinOp::Le, left, right)
    }

    /// Add a greater than test operation to the current [`BasicBlock`].
    fn gt(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Greater");
        RValue::BinOp(BinOp::Gt, left, right)
    }

    /// Add a greater than or equal to test operation to the current [`BasicBlock`].
    fn ge(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Greater or Equal");
        RValue::BinOp(BinOp::Ge, left, right)
    }

    /// Add a raw pointer offset operation to the current [`BasicBlock`].
    fn offset(&mut self, left: Operand, right: Operand) -> RValue  {
        debug!("Pointer Offset");
        todo!()
    }

    /// Terminates by returning to the caller function
    fn term_return(&mut self, span: Span) {
        debug!("Terminator: Return");
        let cid = self.current_bb.unwrap();
        let bb = self.proc.get_bb_mut(cid);
        bb.set_terminator(Terminator::new(TerminatorKind::Return, span));
    }

    /// Terminates by going to the destination basic block
    fn term_goto(&mut self, target: BasicBlockId, span: Span) {
        debug!("Goto: {:?}", target);
        let cid = self.current_bb.unwrap();
        let bb = self.proc.get_bb_mut(cid);
        bb.set_terminator(Terminator::new(TerminatorKind::GoTo { target }, span))
    }

    /// Terminates with a conditional go to
    fn term_cond_goto(
        &mut self,
        cond: Operand,
        then_bb: BasicBlockId,
        else_bb: BasicBlockId,
        span: Span,
    ) {
        debug!("If {:?} then {:?} else {:?}", cond, then_bb, else_bb);
        let cid = self.current_bb.unwrap();
        let bb = self.proc.get_bb_mut(cid);
        bb.set_terminator(Terminator::new(
            TerminatorKind::CondGoTo {
                cond,
                tru: then_bb,
                fls: else_bb,
            },
            span,
        ));
    }

    /// Terminates by calling the given function
    fn term_call(&mut self) {
        debug!("Call");
        todo!()
    }
}

/// Transform a single function to the MIR form
struct FuncTransformer {
    mir: MirBuilder,
}

impl FuncTransformer {
    pub fn new() -> FuncTransformer {
        FuncTransformer {
            mir: MirBuilder::new(),
        }
    }

    pub fn transform(mut self, func: &RoutineDef<SemanticContext>) -> Procedure {
        self.mir.proc.set_span(func.context.span());

        // Create a new MIR Procedure
        // Create a BasicBlock for the function
        let bb = self.mir.new_bb();
        self.mir.set_bb(bb);

        // Iterate over every statement and add it to the basic block
        func.body.iter().for_each(|stm| self.statement(stm));

        // Add the return from function as the terminator for the final basic block of the function
        self.mir.term_return(span_end(func.context.span()));
        self.mir.proc
    }

    fn statement(&mut self, stm: &Statement<SemanticContext>) {
        debug!("Transform statement");
        match stm {
            Statement::Bind(bind) => self.bind(bind),
            Statement::Expression(expr) => {
                self.expression(expr);
            }
            Statement::Mutate(_) => todo!(),
            Statement::YieldReturn(_) => todo!(),
            Statement::Return(ret) => self.ret(ret),
        }
    }

    fn bind(&mut self, bind: &Bind<SemanticContext>) {
        debug!("Binding statement");
        let var = bind.get_id();
        let mutable = bind.is_mutable();
        let ty = bind.get_type();
        let vid = self.mir.var(var, mutable, ty, bind.context().span());

        let expr = self.expression(bind.get_rhs());

        self.mir
            .store(LValue::Var(vid), RValue::Use(expr), bind.context().span())
    }

    fn ret(&mut self, ret: &Return<SemanticContext>) {
        match ret.get_value() {
            Some(val) => {
                let v = self.expression(val);
                self.mir
                    .store(LValue::ReturnPointer, RValue::Use(v), val.context().span());
            }
            None => (),
        };
        self.mir.term_return(ret.context().span());
    }

    /// This can return either an Operand or an RValue, if this is evaluating a constant or an identifier
    /// then this returns an operand.  If this is evaluating an operation then it returns an RValue.
    fn expression(&mut self, expr: &Expression<SemanticContext>) -> Operand {
        match expr {
            // Literals
            Expression::I8(_, i) => self.mir.const_i8(*i),
            Expression::I16(_, i) => self.mir.const_i16(*i),
            Expression::I32(_, i) => self.mir.const_i32(*i),
            Expression::I64(_, i) => self.mir.const_i64(*i),
            Expression::U8(_, u) => self.mir.const_u8(*u),
            Expression::U16(_, u) => self.mir.const_u16(*u),
            Expression::U32(_, u) => self.mir.const_u32(*u),
            Expression::U64(_, u) => self.mir.const_u64(*u),
            Expression::F64(_, f) => self.mir.const_f64(*f),
            Expression::Null(_) => self.mir.const_null(),
            Expression::Boolean(_, b) => self.mir.const_bool(*b),
            Expression::StringLiteral(_, sid) => self.mir.const_stringliteral(*sid),

            // Operations
            Expression::BinaryOp(ctx, op, left, right) => {
                let rv = self.binary_op(*op, left, right);
                self.mir.temp_store(rv, ctx.ty(), ctx.span())
            }
            Expression::UnaryOp(ctx, op, right) => {
                let rv = self.unary_op(*op, right);
                self.mir.temp_store(rv, ctx.ty(), ctx.span())
            },
            Expression::TypeCast(_, _, _) => todo!(),
            Expression::SizeOf(_, _) => todo!(),
            Expression::MemberAccess(_, _, _) => todo!(),
            Expression::ArrayExpression(_, _, _) => todo!(),
            Expression::ArrayAt {
                context,
                array,
                index,
            } => todo!(),
            Expression::CustomType(_, _) => todo!(),
            Expression::Identifier(_, id) => {
                // Look up Var ID using the Identifier String ID
                let vid = self.mir.find_var(*id).unwrap();

                // Return a LValue::Var(VarId) as the result of this expression
                Operand::LValue(LValue::Var(vid))
            }
            Expression::Path(_, _) => todo!(),
            Expression::IdentifierDeclare(_, _, _) => todo!(),
            Expression::RoutineCall(_, _, _, _) => todo!(),
            Expression::StructExpression(_, _, _) => todo!(),
            Expression::If {
                context,
                cond,
                if_arm,
                else_arm,
            } => self.if_expr(cond, if_arm, else_arm),
            Expression::While {
                context,
                cond,
                body,
            } => todo!(),
            Expression::ExpressionBlock(_, block, expr) => {
                for stm in block {
                    self.statement(stm);
                }
                if let Some(expr) = expr {
                    self.expression(expr)
                } else {
                    Operand::Constant(Constant::Unit)
                }
            }
            Expression::Yield(_, _) => todo!(),
        }
    }

    fn if_expr(
        &mut self,
        cond: &Expression<SemanticContext>,
        then_block: &Expression<SemanticContext>,
        else_block: &Option<Box<Expression<SemanticContext>>>,
    ) -> Operand {
        let then_bb = self.mir.new_bb();
        let else_bb = else_block.as_ref().map(|block| (block, self.mir.new_bb()));
        let merge_bb = self.mir.new_bb();

        // Setup the conditional
        let cond_val = self.expression(cond);

        // If there is an else block then jump to the else block on false
        // otherwise jump to the merge block
        if let Some(else_bb) = &else_bb {
            self.mir
                .term_cond_goto(cond_val, then_bb, else_bb.1, cond.context().span());
        } else {
            self.mir
                .term_cond_goto(cond_val, then_bb, merge_bb, cond.context().span());
        }

        // Only create a temp location if this if expression can resolve to a
        // value
        let result = if else_block.is_some() && then_block.get_type() != Type::Unit {
            Some(self.mir.temp(then_block.get_type()))
        } else {
            None
        };

        self.mir.set_bb(then_bb);
        let val = self.expression(then_block);
        result.map(|t| {
            self.mir.store(
                LValue::Temp(t),
                RValue::Use(val),
                then_block.context().span(),
            )
        });
        self.mir
            .term_goto(merge_bb, span_end(then_block.context().span()));

        // If there is an else block, then construct it
        if let Some((else_block, else_bb)) = else_bb {
            self.mir.set_bb(else_bb);
            let val = self.expression(else_block);
            result.map(|t| {
                self.mir.store(
                    LValue::Temp(t),
                    RValue::Use(val),
                    else_block.context().span(),
                )
            });
            self.mir
                .term_goto(merge_bb, span_end(else_block.context().span()));
        }

        self.mir.set_bb(merge_bb);
        match result {
            Some(r) => Operand::LValue(LValue::Temp(r)),
            None => Operand::Constant(Constant::Unit),
        }
    }

    fn unary_op(&mut self, op: UnaryOperator, right: &Expression<SemanticContext>) -> RValue {
        match op {
            UnaryOperator::Negate => {
                let right = self.expression(right);
                self.mir.negate(right)
            },
            UnaryOperator::Not => {
                let right = self.expression(right);
                self.mir.not(right)
            },
            UnaryOperator::AddressConst => todo!(),
            UnaryOperator::AddressMut => todo!(),
            UnaryOperator::DerefRawPointer => todo!(),
        }
    }

    fn binary_op(
        &mut self,
        op: BinaryOperator,
        left: &Expression<SemanticContext>,
        right: &Expression<SemanticContext>,
    ) -> RValue {
        match op {
            BinaryOperator::Add => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.add(left, right)
            }
            BinaryOperator::Sub => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.sub(left, right)
            }
            BinaryOperator::Mul => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.mul(left, right)
            }
            BinaryOperator::Div => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.div(left, right)
            }
            BinaryOperator::BAnd => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.bitwise_and(left, right)
            }
            BinaryOperator::BOr => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.bitwise_or(left, right)
            }
            BinaryOperator::Eq => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.eq(left, right)
            }
            BinaryOperator::NEq => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.neq(left, right)
            }
            BinaryOperator::Ls => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.lt(left, right)
            }
            BinaryOperator::LsEq => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.le(left, right)
            }
            BinaryOperator::Gr => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.gt(left, right)
            }
            BinaryOperator::GrEq => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.ge(left, right)
            }
            BinaryOperator::RawPointerOffset => {
                let left = self.expression(left);
                let right = self.expression(right);
                self.mir.offset(left, right)
            },
        }
    }
}

/// Returns a new span that covers only the last byte of the
/// given span.
///
/// This is used to represent MIR instructions that are inserted
/// after the an expression block as ended and which don't
/// correspond to any code written by the User.  For example, the
/// GoTo inserted at the end of a While loop to return to the top
/// of the loop.
fn span_end(span: Span) -> Span {
    let high = span.high().as_u32();
    if high == 0 {
        Span::zero()
    } else {
        Span::new(Offset::new(high), Offset::new(high))
    }
}
