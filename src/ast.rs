use std::fmt::{Debug, Formatter};
use std::hint::unreachable_unchecked;
use z3::ast::Ast;

pub type Var = String;

#[derive(Clone, Debug)]
pub struct AssertionChain(pub Vec<Bexp>);

impl AssertionChain {
    pub fn new(first: Box<Bexp>, rem: Vec<Box<Bexp>>) -> AssertionChain {
        let mut rem: Vec<Bexp> = rem.into_iter().map(|bexp_box| *bexp_box ).collect();
        rem.insert(0, *first);
        AssertionChain(rem)
    }

    pub fn indent_string(&self, prefix: String) -> String {
        let mut result = format!("{}{{ {:?} }}", prefix.clone(), self.0.first().unwrap());

        for assertion in self.0.iter().skip(1) {
            result += &format!("\n{}⊨\n{}{{ {:?} }}", prefix.clone(), prefix.clone(), assertion)
        }

        result
    }
}

#[derive(Clone)]
pub struct AxBlock(pub AssertionChain, pub Vec<(AxStm, AssertionChain)>);

impl AxBlock {
    pub fn indent_string(&self, prefix: String) -> String {
        let first = &self.0;
        let rem = &self.1;

        let mut res = format!("{}", first.indent_string(prefix.clone()));

        for (stm, chain) in rem.iter() {
            res += &format!("\n{}", stm.indent_string(prefix.clone()));
            res += &format!("\n{}", chain.indent_string(prefix.clone()));
        }

        res

    }

    pub fn into_stm(self) -> Box<Stm> {
        let AxBlock(_, rem) = self;
        let mut rem = rem.into_iter();
        let mut pre_stm = rem.next().unwrap().0.into_stm();

        for (curr_stm, _) in rem {
            pre_stm = Box::new(Stm::Seq(pre_stm, curr_stm.into_stm()))
        }

        pre_stm
    }
}

impl Debug for AxBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.indent_string("".to_owned()))
    }
}


#[derive(Clone, Debug)]
pub enum AxStm {
    Assign(Var, Aexp),
    Skip,
    If(Bexp, AxBlock, AxBlock),
    While(Bexp, AxBlock),
}

impl AxStm {
    pub fn into_stm(self) -> Box<Stm> {
        Box::new(match self {
            AxStm::Skip => Stm::Skip,
            AxStm::Assign(v, e) => Stm::Assign(v, Box::new(e)),
            AxStm::If(cond, then_block, else_block) => {
                let then_stm = then_block.into_stm();
                let else_stm = else_block.into_stm();
                Stm::If(Box::new(cond), then_stm, else_stm)
            },
            AxStm::While(cond, inner_block) => {
                let inner_stm = inner_block.into_stm();
                Stm::While(Box::new(cond), inner_stm)
            }
        })
    }

    pub fn indent_string(&self, prefix: String) -> String {
        match self {
            AxStm::Skip => prefix + "skip",
            AxStm::Assign(v, aexp) => prefix + &format!("{} := {:?}", v, aexp),
            AxStm::If(cond, then_block, else_block) => {
                let then_string = then_block.indent_string(prefix.clone() + "    ");
                let else_string = else_block.indent_string(prefix.clone() + "    ");

                format!("{}if {:?} then\n{}\nelse\n{}\nend", prefix, cond, then_string, else_string)
            },
            AxStm::While(cond, inner_block) => {
                let inner_string = inner_block.indent_string(prefix.clone() + "    ");

                format!("{}while {:?} do\n{}\nend", prefix, cond, inner_string)
            },
        }
    }
}



#[derive(Clone)]
pub enum StmAx {
    Assign(Box<Bexp>, Var, Box<Aexp>, Box<Bexp>),
    Seq(Box<StmAx>, Box<StmAx>), // Only one without pre/post condition
    Skip(Box<Bexp>, Box<Bexp>),
    If(Box<Bexp>, Box<Bexp>, Box<StmAx>, Box<StmAx>, Box<Bexp>),
    While(Box<Bexp>, Box<Bexp>, Box<StmAx>, Box<Bexp>),
}

impl StmAx {
    pub fn get_post(&self) -> Box<Bexp> {
        match self {
            StmAx::Assign(_, _, _, post) => post.clone(),
            StmAx::Skip(_, post) => post.clone(),
            StmAx::If(_, _, _, _, post) => post.clone(),
            StmAx::While(_, _, _, post) => post.clone(),
            StmAx::Seq(_, stm2) => stm2.get_post(),
        }
    }

    pub fn get_pre(&self) -> Box<Bexp> {
        match self {
            StmAx::Assign(pre, _, _, _) => pre.clone(),
            StmAx::Skip(pre, _) => pre.clone(),
            StmAx::If(pre, _, _, _, _) => pre.clone(),
            StmAx::While(pre, _, _, _) => pre.clone(),
            StmAx::Seq(stm1, _) => stm1.get_pre(),
        }
    }

    pub fn into_stm(self) -> Box<Stm> {
        Box::new(match self {
            StmAx::Skip(_, _) => Stm::Skip,
            StmAx::Assign(_, var, aexp, _) => Stm::Assign(var, aexp),
            StmAx::Seq(stm1, stm2) => Stm::Seq(stm1.into_stm(), stm2.into_stm()),
            StmAx::If(_, cond, then_stm, else_stm, _) => {
                Stm::If(cond, then_stm.into_stm(), else_stm.into_stm())
            },
            StmAx::While(_, cond, stm_inner, _) => {
                Stm::While(cond, stm_inner.into_stm())
            },
        })
    }
}

impl Debug for StmAx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            StmAx::Assign(pre, var, aexp, post) => f.write_str(format!("{{ {:?} }} {} := {:?} {{ {:?} }}", pre, var, aexp, post).as_str()),
            StmAx::Seq(stm1, stm2) => f.write_str(format!("({:?}; {:?})", stm1, stm2).as_str()),
            StmAx::Skip(pre, post) => f.write_str(format!("{{ {:?} }} skip {{ {:?} }}", pre, post).as_str()),
            StmAx::If(pre, cond, stm_then, stm_else, post) => f.write_str(format!("{{ {:?} }} if {:?} then {:?} else {:?} end {{ {:?} }}", pre, cond, stm_then, stm_else, post).as_str()),
            StmAx::While(pre, cond, stm, post) => f.write_str(format!("{{ {:?} }} while {:?} do {:?} end {{ {:?} }}", pre, cond, stm, post).as_str()),
        }
    }
}

#[derive(Clone)]
pub enum Stm {
    Assign(Var, Box<Aexp>),
    Seq(Box<Stm>, Box<Stm>),
    Skip,
    If(Box<Bexp>, Box<Stm>, Box<Stm>),
    While(Box<Bexp>, Box<Stm>),
}

impl Debug for Stm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stm::Assign(var, aexp) => f.write_str(format!("{} := {:?}", var, aexp).as_str()),
            Stm::Seq(stm1, stm2) => f.write_str(format!("({:?}; {:?})", stm1, stm2).as_str()),
            Stm::Skip => f.write_str("skip"),
            Stm::If(cond, stm_then, stm_else) => f.write_str(format!("if {:?} then {:?} else {:?} end", cond, stm_then, stm_else).as_str()),
            Stm::While(cond, stm) => f.write_str(format!("while {:?} do {:?} end", cond, stm).as_str()),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Bopcode {
    Or,
    And,
}

impl Debug for Bopcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Bopcode::Or => f.write_str("or"),
            Bopcode::And => f.write_str("and"),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Ropcode {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Debug for Ropcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ropcode::Eq => f.write_str("="),
            Ropcode::Ne => f.write_str("#"),
            Ropcode::Lt => f.write_str("<"),
            Ropcode::Le => f.write_str("<="),
            Ropcode::Gt => f.write_str(">"),
            Ropcode::Ge => f.write_str(">="),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Bexp {
    Rop(Box<Aexp>, Ropcode, Box<Aexp>),
    Bop(Box<Bexp>, Bopcode, Box<Bexp>),
    Not(Box<Bexp>)
}

impl Bexp {
    pub fn to_z3_bool<'a>(&self, ctx: &'a z3::Context) -> z3::ast::Bool<'a> {
        match self {
            Bexp::Not(bexp_inner) => bexp_inner.to_z3_bool(ctx).not(),
            Bexp::Rop(left, rop, right) => {
                let left = left.to_z3_int(ctx);
                let right = right.to_z3_int(ctx);
                match rop {
                    Ropcode::Eq => left._eq(&right),
                    Ropcode::Ne => (left._eq(&right)).not(),
                    Ropcode::Lt => left.lt(&right),
                    Ropcode::Le => left.le(&right),
                    Ropcode::Gt => left.gt(&right),
                    Ropcode::Ge => left.ge(&right),
                }
            },
            Bexp::Bop(left, bop, right) => {
                let left = left.to_z3_bool(ctx);
                let right = right.to_z3_bool(ctx);
                match bop {
                    Bopcode::And => z3::ast::Bool::and(ctx, &[&left, &right]),
                    Bopcode::Or => z3::ast::Bool::or(ctx, &[&left, &right]),
                }
            }
        }
    }

    pub fn substitute(self, var: &Var, new_aexp: &Aexp) -> Self {
        match self {
            Bexp::Not(bexp_inner) => Bexp::Not(Box::new(bexp_inner.substitute(var, new_aexp))),
            Bexp::Rop(left, rop, right) => {
                let left = Box::new(left.substitute(var, new_aexp));
                let right = Box::new(right.substitute(var, new_aexp));
                Bexp::Rop(left, rop, right)
            },
            Bexp::Bop(left, bop, right) => {
                let left = Box::new(left.substitute(var, new_aexp));
                let right = Box::new(right.substitute(var, new_aexp));
                Bexp::Bop(left, bop, right)
            },
        }
    }
}

impl Debug for Bexp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Bexp::Rop(left, rop, right) => f.write_str(format!("({:?} {:?} {:?})", *left, rop, *right).as_str()),
            Bexp::Bop(left, bop, right) => f.write_str(format!("({:?} {:?} {:?})", *left, bop, *right).as_str()),
            Bexp::Not(bexp) => f.write_str(format!("(not {:?})", *bexp).as_str()),
        }
    }
}


#[derive(Clone, Eq, PartialEq)]
pub enum Opcode {
    Add,
    Sub,
    Mul,
}

impl Debug for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Add => f.write_str("+"),
            Opcode::Sub => f.write_str("-"),
            Opcode::Mul => f.write_str("*"),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Aexp {
    Numeral(i64),
    Var(Var),
    Op(Box<Aexp>, Opcode, Box<Aexp>),
}

impl Aexp {
    pub fn to_z3_int<'a>(&self, ctx: &'a z3::Context) -> z3::ast::Int<'a> {
        match self {
            Aexp::Numeral(num) => z3::ast::Int::from_i64(ctx, *num),
            Aexp::Var(var) => z3::ast::Int::new_const(ctx, var.as_str()),
            Aexp::Op(left, op, right) => {
                let left = left.to_z3_int(ctx);
                let right = right.to_z3_int(ctx);
                match op {
                    Opcode::Add => z3::ast::Int::add(ctx, &[&left, &right]),
                    Opcode::Sub => z3::ast::Int::sub(ctx, &[&left, &right]),
                    Opcode::Mul => z3::ast::Int::mul(ctx, &[&left, &right]),
                }
            },
        }
    }

    pub fn substitute(self, var: &Var, new_aexp: &Aexp) -> Self {
        match self {
            Aexp::Var(this_var) if &this_var == var => new_aexp.clone(),
            Aexp::Op(left, op, right) => {
                let left = left.substitute(var, new_aexp);
                let right = right.substitute(var, new_aexp);
                Aexp::Op(Box::new(left), op, Box::new(right))
            },
            _ => self,
        }
    }
}

impl Debug for Aexp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Aexp::Numeral(num) => std::fmt::Debug::fmt(num, f),
            Aexp::Var(var) => std::fmt::Display::fmt(var, f),
            Aexp::Op(left, op, right) => f.write_str(format!("({:?} {:?} {:?})", *left, op, *right).as_str())
        }
    }
}