use super::ast::*;
use crate::state::State;

fn unsupported_op(op: &Opcode) -> ! {
    panic!("Operation {:?} is not valid IMP! You may only use {:?} in pre-/post-conditions.", op, op)
}

fn unsupported_exp(exp: &Aexp) -> ! {
    panic!("{:?} is not valid IMP. You may only use this expression in pre-/post-conditions.", exp)
}

pub fn arithmetic_eval(aexp: &Box<Aexp>, state: &State) -> i64 {
    match aexp.as_ref() {
        Aexp::Numeral(num) => *num,
        Aexp::Var(var) => state.get(var),
        Aexp::Op(left, Opcode::Add, right) => arithmetic_eval(left, state) + arithmetic_eval(right, state),
        Aexp::Op(left, Opcode::Sub, right) => arithmetic_eval(left, state) - arithmetic_eval(right, state),
        Aexp::Op(left, Opcode::Mul, right) => arithmetic_eval(left, state) * arithmetic_eval(right, state),
        Aexp::Op(_, op, _) => unsupported_op(op),
        exp => unsupported_exp(exp),
    }
}

pub fn boolean_eval(bexp: &Box<Bexp>, state: &State) -> bool {
    match bexp.as_ref() {
        Bexp::Not(bexp_inner) => !boolean_eval(bexp_inner, state),
        Bexp::Bop(left, Bopcode::And, right) => boolean_eval(left, state) && boolean_eval(right, state),
        Bexp::Bop(left, Bopcode::Or, right) => boolean_eval(left, state) || boolean_eval(right, state),
        Bexp::Rop(left, Ropcode::Eq, right) => arithmetic_eval(left, state) == arithmetic_eval(right, state),
        Bexp::Rop(left, Ropcode::Ne, right) => arithmetic_eval(left, state) != arithmetic_eval(right, state),
        Bexp::Rop(left, Ropcode::Lt, right) => arithmetic_eval(left, state) < arithmetic_eval(right, state),
        Bexp::Rop(left, Ropcode::Le, right) => arithmetic_eval(left, state) <= arithmetic_eval(right, state),
        Bexp::Rop(left, Ropcode::Gt, right) => arithmetic_eval(left, state) > arithmetic_eval(right, state),
        Bexp::Rop(left, Ropcode::Ge, right) => arithmetic_eval(left, state) >= arithmetic_eval(right, state),
    }
}

