use std::fmt::{Debug, Formatter};
use std::hint::unreachable_unchecked;
use z3::ast::{Ast, Dynamic};
use std::collections::HashMap;
use z3::{RecFuncDecl, Sort};
use std::convert::TryInto;

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

    pub fn get_while_things(&self) -> (&Box<Bexp>, &Box<Aexp>, &Box<Aexp>) {
        match &self {
            AxStm::While(cond, AxBlock(AssertionChain(inner_pre_chain), inner_rem)) => {
                let inner_pre = inner_pre_chain.first().unwrap();
                let inner_post = inner_rem.last().unwrap().1.0.last().unwrap();

                match inner_pre {
                    Bexp::Bop(partial, Bopcode::And, variant_exp) => {
                        match variant_exp.as_ref() {
                            Bexp::Rop(variant, Ropcode::Eq, logical_var) => (partial, variant, logical_var),
                            _ => panic!("A total correctness proof requires an inner pre-condition of the form { condition and ( P ) and variant = LOGICAL_VAR}"),
                        }
                    },
                    _ => panic!("A total correctness proof requires an inner pre-condition of the form { condition and ( P ) and variant = LOGICAL_VAR}"),
                }
            }
            _ => unreachable!()
        }
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
    pub fn to_z3_bool<'a>(&self, ctx: &'a z3::Context, funcmap: &FuncMap<'a>) -> z3::ast::Bool<'a> {
        match self {
            Bexp::Not(bexp_inner) => bexp_inner.to_z3_bool(ctx, funcmap).not(),
            Bexp::Rop(left, rop, right) => {
                let left = left.to_z3_int(ctx, funcmap);
                let right = right.to_z3_int(ctx, funcmap);
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
                let left = left.to_z3_bool(ctx, funcmap);
                let right = right.to_z3_bool(ctx, funcmap);
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

    pub fn pretty_string(&self) -> String {
        match &self {
            Bexp::Rop(left, rop, right) => format!("{} {:?} {}", left.pretty_string(), rop, right.pretty_string()),
            Bexp::Not(not) => {
                if not.precedence() < self.precedence() {
                    format!("not ({})", not.pretty_string())
                } else {
                    format!("not {}", not.pretty_string())
                }
            },
            Bexp::Bop(left, bop, right) => {
                let use_left_paren = self.precedence() > left.precedence() ||
                    self.precedence() == left.precedence() && self.is_right_associative();

                let use_right_paren = self.precedence() > right.precedence() ||
                    self.precedence() == right.precedence() && self.is_left_associative();

                let left_string = if use_left_paren {
                    format!("({})", left.pretty_string())
                } else {
                    format!("{}", left.pretty_string())
                };

                let right_string = if use_right_paren {
                    format!("({})", right.pretty_string())
                } else {
                    format!("{}", right.pretty_string())
                };

                format!("{} {:?} {}", left_string, bop, right_string)
            }
        }
    }

    pub fn is_right_associative(&self) -> bool {
        false // we don't have right-associative arithmetic operators
    }

    pub fn is_left_associative(&self) -> bool {
        true // we don't have right-associative arithmetic operators
    }

    fn precedence(&self) -> u32 {
        match &self {
            Bexp::Rop(_, _, _) => 4,
            Bexp::Not(_) => 3,
            Bexp::Bop(_, Bopcode::And, _) => 2,
            Bexp::Bop(_, Bopcode::Or, _) => 1,
        }
    }
}

impl Debug for Bexp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.pretty_string())
    }
    // fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //     match self {
    //         Bexp::Rop(left, rop, right) => f.write_str(format!("({:?} {:?} {:?})", *left, rop, *right).as_str()),
    //         Bexp::Bop(left, bop, right) => f.write_str(format!("({:?} {:?} {:?})", *left, bop, *right).as_str()),
    //         Bexp::Not(bexp) => f.write_str(format!("(not {:?})", *bexp).as_str()),
    //     }
    // }
}


#[derive(Clone, Eq, PartialEq)]
pub enum Opcode {
    Add,
    Sub,
    Mul,
    Mod,
    Pow,
}

impl Debug for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Add => f.write_str("+"),
            Opcode::Sub => f.write_str("-"),
            Opcode::Mul => f.write_str("*"),
            Opcode::Mod => f.write_str("%"),
            Opcode::Pow => f.write_str("^"),
        }
    }
}

// #[derive(Clone, Eq, PartialEq)]
// pub enum Unopcode {
//     Fact,
// }
//
// impl Debug for Unopcode {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Unopcode::Fact => f.write_str("!"),
//         }
//     }
// }

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ImpFuncDef {
    pub name: String,
    pub args: Vec<Var>,
    pub body: Aexp,
}

impl ImpFuncDef {
    pub fn to_z3_func_decl<'ctx>(&self, ctx: &'ctx z3::Context) -> z3::RecFuncDecl<'ctx> {
        // let domain: Vec<_> = self.args.iter().map(|_| &Sort::int(&ctx)).collect();
        let domain = vec![Sort::int(&ctx); self.args.len()];
        let domain: Vec<_> = domain.iter().collect();
        let f = RecFuncDecl::new(ctx, self.name.as_str(), domain.as_slice(), &Sort::int(&ctx));

        f
    }

    pub fn define<'a>(&self, ctx: &'a z3::Context, funcmap: &FuncMap<'a>) {
        let mut f = funcmap.get(&self.name).unwrap();

        let args: Vec<_> = self.args.iter().map(|arg| z3::ast::Dynamic::from(z3::ast::Int::new_const(&ctx, arg.as_str()))).collect();
        let args: Vec<_> = args.iter().collect();
        f.add_def(args.as_slice(), &self.body.to_z3_int(&ctx, funcmap));
    }
}

type FuncMap<'ctx> = HashMap<String, z3::RecFuncDecl<'ctx>>;

#[derive(Clone, Eq, PartialEq)]
pub enum Aexp {
    Numeral(i64),
    Var(Var),
    Op(Box<Aexp>, Opcode, Box<Aexp>),
    FuncApp(String, Vec<Aexp>),
    Ite(Box<Bexp>, Box<Aexp>, Box<Aexp>),
    // Unop(Box<Aexp>, Unopcode),
}

impl Aexp {
    pub fn to_z3_int<'a>(&self, ctx: &'a z3::Context, funcmap: &FuncMap<'a>) -> z3::ast::Int<'a> {
        match self {
            Aexp::Numeral(num) => z3::ast::Int::from_i64(ctx, *num),
            Aexp::Var(var) => z3::ast::Int::new_const(ctx, var.as_str()),
            Aexp::Op(left, op, right) => {
                let left = left.to_z3_int(ctx, funcmap);
                let right = right.to_z3_int(ctx, funcmap);
                match op {
                    Opcode::Add => z3::ast::Int::add(ctx, &[&left, &right]),
                    Opcode::Sub => z3::ast::Int::sub(ctx, &[&left, &right]),
                    Opcode::Mul => z3::ast::Int::mul(ctx, &[&left, &right]),
                    Opcode::Mod => left.modulo(&right),
                    Opcode::Pow => left.power(&right),
                }
            },
            Aexp::FuncApp(fname, args) => {
                let args: Vec<z3::ast::Dynamic<'a>> = args.into_iter().map(|arg| {
                    arg.to_z3_int(&ctx, &funcmap).into()
                }).collect();

                let args: Vec<_> = args.as_slice().into_iter().map(|v| v).collect();
                let args = args.as_slice();
                let func = funcmap.get(fname).unwrap();
                let res = func.apply(args);
                let res = res.as_int().unwrap();

                res
            },
            Aexp::Ite(cond, t, e) => {
                let cond = cond.to_z3_bool(ctx, funcmap);
                let t = t.to_z3_int(ctx, funcmap);
                let e = e.to_z3_int(ctx, funcmap);

                cond.ite(&t, &e)
            }
        }
    }

    pub fn pretty_string(&self) -> String {
        match &self {
            Aexp::Numeral(num) => format!("{}", num),
            Aexp::Var(v) => format!("{}", v),
            Aexp::Op(left, op, right) => {
                let use_left_paren = self.precedence() > left.precedence() ||
                    self.precedence() == left.precedence() && self.is_right_associative();

                let use_right_paren = self.precedence() > right.precedence() ||
                    self.precedence() == right.precedence() && self.is_left_associative();

                let left_string = if use_left_paren {
                    format!("({})", left.pretty_string())
                } else {
                    format!("{}", left.pretty_string())
                };

                let right_string = if use_right_paren {
                    format!("({})", right.pretty_string())
                } else {
                    format!("{}", right.pretty_string())
                };

                format!("{} {:?} {}", left_string, op, right_string)
            },
            Aexp::FuncApp(fname, args) => {
                let args: Vec<String> = args.into_iter().map(|arg| arg.pretty_string()).collect();
                let arg_string = args.join(", ");
                format!("{}({})", fname, arg_string)
            },
            Aexp::Ite(cond, t, e) => {
                format!("{} ? {} : {}", cond.pretty_string(), t.pretty_string(), e.pretty_string())
            },
        }
    }

    pub fn is_right_associative(&self) -> bool {
        false // we don't have right-associative arithmetic operators
    }

    pub fn is_left_associative(&self) -> bool {
        true // we don't have right-associative arithmetic operators
    }

    fn precedence(&self) -> u32 {
        match &self {
            Aexp::Numeral(_) => 4,
            Aexp::Var(_) => 4,
            Aexp::Op(_, Opcode::Add, _) => 1,
            Aexp::Op(_, Opcode::Sub, _) => 1,
            Aexp::Op(_, Opcode::Mul, _) => 2,
            Aexp::Op(_, Opcode::Mod, _) => 2,
            Aexp::Op(_, Opcode::Pow, _) => 3,
            Aexp::FuncApp(_, _) => 4,
            Aexp::Ite(_, _, _) => 0,
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
            Aexp::Ite(cond, t, e) => {
                let cond = Box::new(cond.substitute(var, new_aexp));
                let t = Box::new(t.substitute(var, new_aexp));
                let e = Box::new(e.substitute(var, new_aexp));
                Aexp::Ite(cond, t, e)
            }
            Aexp::FuncApp(fname, args) => {
                Aexp::FuncApp(fname, args.into_iter().map(|arg| arg.substitute(var, new_aexp)).collect())
            }
            _ => self,
        }
    }
}

impl Debug for Aexp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.pretty_string())
    }
    // fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //     match self {
    //         Aexp::Numeral(num) => std::fmt::Debug::fmt(num, f),
    //         Aexp::Var(var) => std::fmt::Display::fmt(var, f),
    //         Aexp::Op(left, op, right) => f.write_str(format!("({:?} {:?} {:?})", *left, op, *right).as_str())
    //     }
    // }
}