/**
 * This traverses the AST and determines what size register to
 * assign to each node in the AST: if it makes sense to assign
 * it to a register.
 */
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RegSize {
    R8,
    R16,
    R32,
    R64,
}

impl RegSize {
    pub fn assign(nbytes: usize) -> Option<RegSize> {
        if nbytes == 0 {
            None
        } else if nbytes <= 1 {
            Some(RegSize::R8)
        } else if nbytes <= 2 {
            Some(RegSize::R16)
        } else if nbytes <= 4 {
            Some(RegSize::R32)
        } else if nbytes <= 8 {
            Some(RegSize::R64)
        } else {
            None
        }
    }
}

impl std::fmt::Display for RegSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            RegSize::R8 => f.write_str("R8"),
            RegSize::R16 => f.write_str("R16"),
            RegSize::R32 => f.write_str("R32"),
            RegSize::R64 => f.write_str("R64"),
        }
    }
}

pub mod assign {
    use super::*;
    use crate::compiler::memory::scope::SymbolOffsetTable;
    use crate::compiler::memory::struct_table::ResolvedStructTable;
    use crate::expression::Expression;
    use crate::syntax::module::*;
    use crate::syntax::routinedef::*;
    use crate::syntax::statement::*;
    use crate::syntax::structdef::*;
    use crate::syntax::ty::Type;
    use stdext::function_name;

    macro_rules! trace {
        ($ts:expr, $sz:expr) => {
            println!(
                "{} [{}]{} -> {:?} {:?}",
                function_name!(),
                $ts.get_annotations().id(),
                $ts.get_name(),
                $sz,
                RegSize::assign($sz.unwrap_or(0) as usize)
            )
        };
    }

    fn register_for_type(ty: &Type, struct_table: &ResolvedStructTable) -> Option<RegSize> {
        let sz = struct_table.size_of(ty);
        sz.and_then(|sz| RegSize::assign(sz as usize))
    }

    fn assign_register(a: &mut SymbolOffsetTable, struct_table: &ResolvedStructTable) {
        let reg = register_for_type(a.ty(), struct_table);
        a.set_reg_size(reg);
    }

    pub fn for_module(m: &mut Module<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(m, struct_table.size_of(m.get_annotations().ty()));
        assign_register(m.get_annotations_mut(), struct_table);

        for child_module in m.get_modules_mut().iter_mut() {
            for_module(child_module, struct_table);
        }
        for_items(m.get_functions_mut(), struct_table);

        for_items(m.get_coroutines_mut(), struct_table);

        for_items(m.get_structs_mut(), struct_table);
    }

    fn for_items(items: &mut Vec<Item<SymbolOffsetTable>>, struct_table: &ResolvedStructTable) {
        for i in items.iter_mut() {
            trace!(i, struct_table.size_of(i.get_annotations().ty()));
            assign_register(i.get_annotations_mut(), struct_table);
            match i {
                Item::Struct(sd) => {
                    for_structdef(sd, struct_table);
                }
                Item::Routine(rd) => {
                    for_routine(rd, struct_table);
                }
            };
        }
    }

    fn for_structdef(sd: &mut StructDef<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(sd, struct_table.size_of(sd.get_annotations().ty()));
        assign_register(sd.get_annotations_mut(), struct_table);

        for (fname, fty) in sd.get_fields() {
            let reg = register_for_type(fty, struct_table);
            println!(
                "{}:({},{:?}) -> {:?}",
                fname,
                fty,
                struct_table.size_of(fty),
                reg
            );
        }
    }

    fn for_routine(rd: &mut RoutineDef<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(rd, struct_table.size_of(rd.get_annotations().ty()));
        assign_register(rd.get_annotations_mut(), struct_table);

        // loop through all the params
        let mut param_annotations = vec![];
        for p in rd.get_params() {
            println!(
                "{} {} -> {:?}",
                function_name!(),
                p.0,
                struct_table.size_of(&p.1)
            );
            let mut param_annotation = rd.get_annotations().clone();
            param_annotation.ty = p.1.clone();
            assign_register(&mut param_annotation, struct_table);
            param_annotations.push(param_annotation);
        }

        rd.get_param_annotations_mut()
            .append(&mut param_annotations);

        // loop through every statement and analyze the child nodes of the routine definition
        for e in rd.get_body_mut().iter_mut() {
            for_statement(e, struct_table);
        }
    }

    fn for_statement(
        statement: &mut Statement<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(
            statement,
            struct_table.size_of(statement.get_annotations().ty())
        );
        assign_register(statement.get_annotations_mut(), struct_table);

        match statement {
            Statement::Bind(b) => {
                for_bind(b, struct_table);
            }
            Statement::Mutate(m) => {
                for_mutate(m, struct_table);
            }
            Statement::Return(r) => {
                for_return(r, struct_table);
            }
            Statement::YieldReturn(yr) => {
                for_yieldreturn(yr, struct_table);
            }
            Statement::Expression(e) => {
                for_expression(e, struct_table);
            }
        };
    }

    fn for_bind(bind: &mut Bind<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(bind, struct_table.size_of(bind.get_annotations().ty()));
        assign_register(bind.get_annotations_mut(), struct_table);
        for_expression(bind.get_rhs_mut(), struct_table)
    }

    fn for_mutate(mutate: &mut Mutate<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(mutate, struct_table.size_of(mutate.get_annotations().ty()));
        assign_register(mutate.get_annotations_mut(), struct_table);

        for_expression(mutate.get_rhs_mut(), struct_table);
    }

    fn for_yieldreturn(
        yr: &mut YieldReturn<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(yr, struct_table.size_of(yr.get_annotations().ty()));

        assign_register(yr.get_annotations_mut(), struct_table);
        yr.get_value_mut()
            .as_mut()
            .map(|rv| for_expression(rv, struct_table));
    }

    fn for_return(r: &mut Return<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(r, struct_table.size_of(r.get_annotations().ty()));

        assign_register(r.get_annotations_mut(), struct_table);
        r.get_value_mut()
            .as_mut()
            .map(|rv| for_expression(rv, struct_table));
    }

    fn for_expression(exp: &mut Expression<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        use Expression::*;

        trace!(exp, struct_table.size_of(exp.get_annotations().ty()));

        assign_register(exp.get_annotations_mut(), struct_table);
        match exp {
            ExpressionBlock(..) => for_expression_block(exp, struct_table),
            Expression::Integer32(_m, _i) => {}
            Expression::Integer64(_m, _i) => {}
            Expression::Boolean(_m, _b) => {}
            Expression::StringLiteral(_m, _s) => {}
            Expression::CustomType(_m, _name) => {}
            Expression::Identifier(_m, _id) => {}
            Path(_m, _path) => {}
            Expression::IdentifierDeclare(_m, _id, _p) => {}
            MemberAccess(..) => for_member_access(exp, struct_table),
            UnaryOp(..) => for_unary_op(exp, struct_table),
            BinaryOp(..) => for_binary_op(exp, struct_table),
            If { .. } => for_if(exp, struct_table),
            Yield(..) => for_yield(exp, struct_table),
            RoutineCall(..) => for_routine_call(exp, struct_table),
            StructExpression(..) => for_struct_expression(exp, struct_table),
        }
    }

    fn for_expression_block(
        block: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(block, struct_table.size_of(block.get_annotations().ty()));

        assign_register(block.get_annotations_mut(), struct_table);
        if let Expression::ExpressionBlock(_m, ref mut body, ref mut final_exp) = block {
            for e in body.iter_mut() {
                for_statement(e, struct_table);
            }

            final_exp
                .as_mut()
                .map(|fe| for_expression(fe, struct_table));
        } else {
            panic!("Expected ExpressionBlock, but got {:?}", block)
        }
    }

    fn for_member_access(
        access: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(access, struct_table.size_of(access.get_annotations().ty()));

        assign_register(access.get_annotations_mut(), struct_table);
        if let Expression::MemberAccess(_m, src, _member) = access {
            for_expression(src, struct_table);
        } else {
            panic!("Expected MemberAccess, but got {:?}", access)
        }
    }

    fn for_unary_op(un_op: &mut Expression<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(un_op, struct_table.size_of(un_op.get_annotations().ty()));

        assign_register(un_op.get_annotations_mut(), struct_table);
        if let Expression::UnaryOp(_m, _op, operand) = un_op {
            for_expression(operand, struct_table);
        } else {
            panic!("Expected UnaryOp, but got {:?}", un_op)
        }
    }

    fn for_binary_op(
        bin_op: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(bin_op, struct_table.size_of(bin_op.get_annotations().ty()));

        assign_register(bin_op.get_annotations_mut(), struct_table);
        if let Expression::BinaryOp(_m, _op, l, r) = bin_op {
            for_expression(l, struct_table);
            for_expression(r, struct_table);
        } else {
            panic!("Expected BinaryOp, but got {:?}", bin_op)
        }
    }

    fn for_if(if_exp: &mut Expression<SymbolOffsetTable>, struct_table: &ResolvedStructTable) {
        trace!(if_exp, struct_table.size_of(if_exp.get_annotations().ty()));

        assign_register(if_exp.get_annotations_mut(), struct_table);
        if let Expression::If {
            annotation: _m,
            cond,
            if_arm,
            else_arm,
        } = if_exp
        {
            for_expression(cond, struct_table);
            for_expression(if_arm, struct_table);
            else_arm.as_mut().map(|ea| for_expression(ea, struct_table));
        } else {
            panic!("Expected IfExpression, but got {:?}", if_exp)
        }
    }

    fn for_routine_call(
        rc: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(rc, struct_table.size_of(rc.get_annotations().ty()));

        assign_register(rc.get_annotations_mut(), struct_table);
        if let Expression::RoutineCall(_m, _call, _name, params) = rc {
            for p in params {
                for_expression(p, struct_table);
            }
        } else {
            panic!("Expected RoutineCall, but got {:?}", rc)
        }
    }

    fn for_yield(
        yield_exp: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(
            yield_exp,
            struct_table.size_of(yield_exp.get_annotations().ty())
        );

        assign_register(yield_exp.get_annotations_mut(), struct_table);
        if let Expression::Yield(_m, e) = yield_exp {
            for_expression(e, struct_table);
        } else {
            panic!("Expected Yield, but got {:?}", yield_exp)
        }
    }

    fn for_struct_expression(
        se: &mut Expression<SymbolOffsetTable>,
        struct_table: &ResolvedStructTable,
    ) {
        trace!(se, struct_table.size_of(se.get_annotations().ty()));

        assign_register(se.get_annotations_mut(), struct_table);
        if let Expression::StructExpression(_annotations, _struct_name, fields) = se {
            for (_, fe) in fields {
                for_expression(fe, struct_table);
            }
        } else {
            panic!("Expected StructExpression, but got {:?}", se)
        }
    }
}
