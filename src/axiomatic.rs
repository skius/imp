use super::ast::*;
use super::state::*;
use super::entailment::*;
use super::imp_lang::*;
use super::error::ImpErrorInner;
use super::error::ImpErrorInner::*;
use z3::{SatResult, FuncDecl, RecFuncDecl, Model};
use std::collections::{HashMap, HashSet};
use z3::ast::{forall_const, Ast};
use std::convert::TryInto;
use egg::RecExpr;
use crate::error::{err_acc, ImpErrors};

use super::Result;

pub fn build_funcmap<'ctx>(ctx: &'ctx z3::Context, funcdefs: &HashMap<String, ImpFuncDef>) -> Result<HashMap<String, RecFuncDecl<'ctx>>> {
    let funcmap: HashMap<_, _> = funcdefs.iter().map(|(k, v)| (k.clone(), v.to_z3_func_decl(&ctx))).collect();
    for (name, f) in &funcmap {
        funcdefs.get(name).ok_or(ImpErrorInner::Other(format!("Function {} not found", name)))?.define(&ctx, &funcmap);
    }
    Ok(funcmap)
}

pub fn verify_block_except_cons_partial(AxBlock(AssertionChain(first), rem): &AxBlock) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };

    let mut pre = first.last().unwrap();
    for (stm, AssertionChain(post_chain)) in rem {
        let post = post_chain.first().unwrap();
        match stm {
            AxStm::Skip => {
                if pre != post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: post.clone(),
                        expected: pre.clone(),
                    }.into()));
                }
            },
            AxStm::Assign(v, aexp) => {
                // println!("Substituting {:?} in {:?} for {:?}", v, post, aexp);
                let pre_must = post.clone().substitute(v, aexp);
                // assert_eq!(*pre, pre_must);
                if *pre != pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: pre.clone(),
                        expected: pre_must,
                    }.into()));
                }
            },
            AxStm::If(
                cond,
                AxBlock(AssertionChain(then_pre_chain), then_rem),
                AxBlock(AssertionChain(else_pre_chain), else_rem))
            => {
                let then_pre = then_pre_chain.first().unwrap();
                let else_pre = else_pre_chain.first().unwrap();
                let then_post = then_rem.last().unwrap().1.0.last().unwrap();
                let else_post = else_rem.last().unwrap().1.0.last().unwrap();

                let then_pre_must = Bexp::Bop(Box::new(cond.clone()), Bopcode::And, Box::new(pre.clone()));
                let else_pre_must = Bexp::Bop(Box::new(Bexp::Not(Box::new(cond.clone()))), Bopcode::And, Box::new(pre.clone()));
                // assert_eq!(*then_pre, then_pre_must);
                if *then_pre != then_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: then_pre.clone(),
                        expected: then_pre_must.clone(),
                    }.into()));
                }

                // assert_eq!(*else_pre, else_pre_must);
                if *else_pre != else_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: else_pre.clone(),
                        expected: else_pre_must.clone(),
                    }.into()))
                }
                // assert_eq!(post, then_post);
                if post != then_post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: then_post.clone(),
                        expected: post.clone(),
                    }.into()))
                }
                // assert_eq!(post, else_post);
                if post != else_post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: else_post.clone(),
                        expected: post.clone(),
                    }.into()))
                }
            },
            AxStm::While(cond, AxBlock(AssertionChain(inner_pre_chain), inner_rem)) => {
                let inner_pre = inner_pre_chain.first().unwrap();
                let inner_post = inner_rem.last().unwrap().1.0.last().unwrap();

                let inner_pre_must = Bexp::Bop(Box::new(cond.clone()), Bopcode::And, Box::new(pre.clone()));
                // assert_eq!(*inner_pre, inner_pre_must);
                if *inner_pre != inner_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: inner_pre.clone(),
                        expected: inner_pre_must.clone(),
                    }.into()))
                }
                // assert_eq!(inner_post, pre);
                if inner_post != pre {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: inner_post.clone(),
                        expected: pre.clone(),
                    }.into()))
                }
                let post_must = Bexp::Bop(Box::new(Bexp::Not(Box::new(cond.clone()))), Bopcode::And, Box::new(pre.clone()));
                // assert_eq!(*post, post_must)
                if *post != post_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: post.clone(),
                        expected: post_must.clone(),
                    }.into()))
                }
                // TODO: Write helper function err_must_eq for ^^ above structure
            },
        }
        pre = post_chain.last().unwrap();
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

pub fn verify_cons_partial(cfg: &z3::Config, AxBlock(first, rem): &AxBlock, funcdefs: &HashMap<String, ImpFuncDef>) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };

    ea(verify_assertion_chain(&cfg, first, funcdefs));
    for (stm, post_chain) in rem {
        match stm {
            AxStm::While(_, inner_block) => ea(verify_cons_partial(cfg, inner_block, funcdefs)),
            AxStm::If(_, then_block, else_block) => {
                ea(verify_cons_partial(cfg, then_block, funcdefs));
                ea(verify_cons_partial(cfg, else_block, funcdefs));
            },
            _ => (),
        }
        ea(verify_assertion_chain(cfg, post_chain, funcdefs));
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

pub fn verify_block_except_cons_total(AxBlock(AssertionChain(first), rem): &AxBlock) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };

    let mut pre = first.last().unwrap();
    for (stm, AssertionChain(post_chain)) in rem {
        let post = post_chain.first().unwrap();
        match stm {
            AxStm::Skip => {
                if pre != post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: post.clone(),
                        expected: pre.clone(),
                    }.into()));
                }
            },
            AxStm::Assign(v, aexp) => {
                // println!("Substituting {:?} in {:?} for {:?}", v, post, aexp);
                let pre_must = post.clone().substitute(v, aexp);
                // assert_eq!(*pre, pre_must);
                if *pre != pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: pre.clone(),
                        expected: pre_must,
                    }.into()));
                }
            },
            AxStm::If(
                cond,
                AxBlock(AssertionChain(then_pre_chain), then_rem),
                AxBlock(AssertionChain(else_pre_chain), else_rem))
            => {
                let then_pre = then_pre_chain.first().unwrap();
                let else_pre = else_pre_chain.first().unwrap();
                let then_post = then_rem.last().unwrap().1.0.last().unwrap();
                let else_post = else_rem.last().unwrap().1.0.last().unwrap();

                let then_pre_must = Bexp::Bop(Box::new(cond.clone()), Bopcode::And, Box::new(pre.clone()));
                let else_pre_must = Bexp::Bop(Box::new(Bexp::Not(Box::new(cond.clone()))), Bopcode::And, Box::new(pre.clone()));

                // assert_eq!(*then_pre, then_pre_must);
                if *then_pre != then_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: then_pre.clone(),
                        expected: then_pre_must.clone(),
                    }.into()));
                }

                // assert_eq!(*else_pre, else_pre_must);
                if *else_pre != else_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: else_pre.clone(),
                        expected: else_pre_must.clone(),
                    }.into()))
                }
                // assert_eq!(post, then_post);
                if post != then_post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: then_post.clone(),
                        expected: post.clone(),
                    }.into()))
                }
                // assert_eq!(post, else_post);
                if post != else_post {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: else_post.clone(),
                        expected: post.clone(),
                    }.into()))
                }
            },
            AxStm::While(cond, AxBlock(AssertionChain(inner_pre_chain), inner_rem)) => {
                // let inner_pre = inner_pre_chain.first().unwrap();
                let inner_post = inner_rem.last().unwrap().1.0.last().unwrap();

                // if let Bexp::Bop(partial, Bopcode::And, variant) = inner_pre {
                //
                // } else {
                //     panic!("A total correctness proof requires an inner pre-condition of the form { condition and ( P ) and variant = LOGICAL_VAR}");
                // }

                let (inner_pre_partial, variant, logical_var) = stm.get_while_things();

                let inner_pre_must = Bexp::Bop(Box::new(cond.clone()), Bopcode::And, Box::new(pre.clone()));
                // assert_eq!(**inner_pre_partial, inner_pre_must);
                if **inner_pre_partial != inner_pre_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: *inner_pre_partial.clone(),
                        expected: inner_pre_must.clone(),
                    }.into()))
                }

                // assert_eq!(*inner_post, Bexp::Bop(Box::new(pre.clone()), Bopcode::And, Box::new(Bexp::Rop(variant.clone(), Ropcode::Lt, logical_var.clone()))));
                let inner_post_must = Bexp::Bop(Box::new(pre.clone()), Bopcode::And, Box::new(Bexp::Rop(variant.clone(), Ropcode::Lt, logical_var.clone())));
                if *inner_post != inner_post_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: inner_post.clone(),
                        expected: inner_post_must.clone(),
                    }.into()))
                }

                let post_must = Bexp::Bop(Box::new(Bexp::Not(Box::new(cond.clone()))), Bopcode::And, Box::new(pre.clone()));
                // assert_eq!(*post, post_must)
                if *post != post_must {
                    ea(Err(AxStructureError {
                        stm: stm.clone(),
                        actual: post.clone(),
                        expected: post_must.clone(),
                    }.into()))
                }
            },
        }
        pre = post_chain.last().unwrap();
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

pub fn verify_cons_total(cfg: &z3::Config, AxBlock(first, rem): &AxBlock, funcdefs: &HashMap<String, ImpFuncDef>) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };

    ea(verify_assertion_chain(&cfg, first, funcdefs));
    for (stm, post_chain) in rem {
        match &stm {
            AxStm::While(_, inner_block) => {
                ea(verify_cons_total(cfg, inner_block, funcdefs));

                let (partial_pre, variant, _) = stm.get_while_things();

                let must_entail = Bexp::Rop(Box::new(Aexp::Numeral(0)), Ropcode::Le, variant.clone());

                println!("Verifying WhTotAx side-condition (b ∧ P ⊨ 0 ≤ e):\n{:?} ⊨ {:?}", partial_pre, must_entail);

                ea(check_entailment(cfg, funcdefs, &partial_pre, &must_entail));
                // let mut solver = z3::Solver::new(&ctx);
                // solver.assert(&entails);
                // let res = solver.check();
                // if res == SatResult::Unsat {
                //     println!("Verified.");
                // } else {
                //     // println!("ERROR {:?}! Model where entailment does not hold:", res);
                //     // println!("{:?}", solver.get_model().unwrap());
                //     panic!("ERROR Result is {:?}", res);
                // }
            },
            AxStm::If(_, then_block, else_block) => {
                ea(verify_cons_total(cfg, then_block, funcdefs));
                ea(verify_cons_total(cfg, else_block, funcdefs));
            },
            _ => (),
        }
        ea(verify_assertion_chain(cfg, post_chain, funcdefs));
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

fn verify_assertion_chain(cfg: &z3::Config, AssertionChain(chain): &AssertionChain, funcdefs: &HashMap<String, ImpFuncDef>) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };


    let mut p = chain.first().unwrap();

    for q in chain.iter().skip(1) {
        println!("Verifying ConsAx rule:\n{{ {:?} }} ⊨ {{ {:?} }}", p, q);

        ea(check_entailment(cfg, funcdefs, p, q));

        p = q;
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

    // match *s {
    //     StmAx::Skip(pre, post) => assert_eq!(pre, post),
    //     StmAx::Assign(pre, var, aexp, post) => {
    //         let post_substituted = post.substitute(&var, &aexp);
    //         assert_eq!(post_substituted, *pre)
    //     },
    //     StmAx::Seq(stm1,  stm2) => {
    //         verify_rules_except_cons(stm1);
    //         verify_rules_except_cons(stm2);
    //     },
    //     StmAx::If(pre, cond, then_stm, else_stm, post) => {
    //         let then_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
    //         let else_pre_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
    //         assert_eq!(then_pre_must, *then_stm.get_pre());
    //         assert_eq!(else_pre_must, *else_stm.get_pre());
    //         assert_eq!(post, then_stm.get_post());
    //         assert_eq!(post, else_stm.get_post());
    //     },
    //     StmAx::While(pre, cond, stm_inner, post) => {
    //         let inner_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
    //         assert_eq!(inner_pre_must, *stm_inner.get_pre());
    //         assert_eq!(pre, stm_inner.get_post());
    //         let post_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
    //         assert_eq!(post_must, *post)
    //     }
    // }
// }

// pub fn verify_rules_except_cons(s: Box<StmAx>) {
//     match *s {
//         StmAx::Skip(pre, post) => assert_eq!(pre, post),
//         StmAx::Assign(pre, var, aexp, post) => {
//             let post_substituted = post.substitute(&var, &aexp);
//             assert_eq!(post_substituted, *pre)
//         },
//         StmAx::Seq(stm1,  stm2) => {
//             verify_rules_except_cons(stm1);
//             verify_rules_except_cons(stm2);
//         },
//         StmAx::If(pre, cond, then_stm, else_stm, post) => {
//             let then_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
//             let else_pre_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
//             assert_eq!(then_pre_must, *then_stm.get_pre());
//             assert_eq!(else_pre_must, *else_stm.get_pre());
//             assert_eq!(post, then_stm.get_post());
//             assert_eq!(post, else_stm.get_post());
//         },
//         StmAx::While(pre, cond, stm_inner, post) => {
//             let inner_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
//             assert_eq!(inner_pre_must, *stm_inner.get_pre());
//             assert_eq!(pre, stm_inner.get_post());
//             let post_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
//             assert_eq!(post_must, *post)
//         }
//     }
// }

// pub fn verify_cons(s: Box<StmAx>) {
//     let config = z3::Config::new();
//     let context = &z3::Context::new(&config);
//
//     verify_cons_single(context, s);
//     println!("Successfully verified program.");
// }

// fn verify_cons_single(ctx: &z3::Context, s: Box<StmAx>) {
//     match *s {
//         StmAx::Seq(stm1, stm2) => {
//             verify_cons_single(ctx, stm1.clone());
//
//
//             let post = stm1.get_post();
//             let pre = stm2.get_pre();
//
//             println!("Verifying ConsAx rule:\n{{ {:?} }} |= {{ {:?} }}", post, pre);
//
//             let post_entails_pre = entails(ctx, post.to_z3_bool(ctx), pre.to_z3_bool(ctx));
//             let mut solver = z3::Solver::new(ctx);
//             solver.assert(&post_entails_pre);
//             let res = solver.check();
//             if res == SatResult::Unsat {
//                 println!("Verified.");
//             } else {
//                 println!("ERROR! Model where entailment does not hold:");
//                 println!("{:?}", solver.get_model().unwrap());
//                 panic!("verification failed.");
//             }
//
//             verify_cons_single(ctx, stm2);
//         },
//         StmAx::If(_, _, then_stm, else_stm, _) => {
//             verify_cons_single(ctx, then_stm);
//             verify_cons_single(ctx, else_stm);
//         },
//         StmAx::While(_, _, stm_inner, _) => {
//             verify_cons_single(ctx, stm_inner);
//         }
//         _ => (),
//     }
// }

fn check_entailment<'a>(cfg: &z3::Config, funcdefs: &HashMap<String, ImpFuncDef>, p: &Bexp, q: &Bexp) -> Result<()> {
    let mut errs = ImpErrors(vec![]);
    let mut ea = |r: Result<()>| {
        err_acc(&mut errs, r);
    };


    let ctx = z3::Context::new(&cfg);

    // TODO: Configure usage of egg optimizer, disallow on functions

    let (p, q) = if p.can_egg() && q.can_egg() {
        let p_sexp = p.sexp_string();
        let q_sexp = q.sexp_string();
        let p_egg: RecExpr<ImpExpr> = p_sexp.parse().unwrap();
        let q_egg: RecExpr<ImpExpr> = q_sexp.parse().unwrap();

        let bests = get_bests(vec![&p_egg, &q_egg]);
        println!("Found bests: {}", bests.iter().map(|recexpr| recexpr.to_string()).collect::<Vec<_>>().join(", "));

        let p_canon_sexp = bests[0].to_string();
        let q_canon_sexp = bests[1].to_string();

        let p_canon: Bexp = *SBexpParser::new().parse(p_canon_sexp.as_str()).unwrap();
        let q_canon: Bexp = *SBexpParser::new().parse(q_canon_sexp.as_str()).unwrap();
        (p_canon, q_canon)
    } else {
        (p.clone(), q.clone())
    };

    let funcmap = build_funcmap(&ctx, funcdefs)?;

    let p_entails_q = entails(&ctx, p.to_z3_bool(&ctx, &funcmap), q.to_z3_bool(&ctx, &funcmap));
    let mut solver = z3::Solver::new(&ctx);
    solver.assert(&p_entails_q);
    // let x = z3::ast::Int::new_const(&ctx, "x");
    // let x_minus_1 = z3::ast::Int::sub(&ctx, &[&x, &z3::ast::Int::from_i64(&ctx, 1)]);
    // let fac = funcmap.get("factorial").unwrap();
    // let fac_of_x_minus_1 = fac.apply(&[&x_minus_1.into()]);
    // let fac_of_x = fac.apply(&[&x.clone().into()]);
    // let x_times_fac_of_x_minus_1 = z3::ast::Int::mul(&ctx, &[&x, &fac_of_x_minus_1.as_int().unwrap()]);
    //
    // solver.assert(&forall_const(
    //     &ctx, &[&x.into()], &[], &fac_of_x._eq(&x_times_fac_of_x_minus_1.into())
    // ).as_bool().unwrap());
    let res = solver.check();
    if res == SatResult::Unsat {
        println!("Verified.");
    } else if res == SatResult::Unknown {
        println!("ERROR! Couldn't prove or disprove. Unknown.");
        ea(Err(EntailmentError {
            entailment_src: p.clone(),
            entailment_dst: q.clone(),
            is_unknown: true,
            untrue_model: None,
        }.into()));
    } else {
        println!("ERROR {:?}!", res);
        println!("Model where entailment does not hold:\n{:?}", solver.get_model().unwrap());

        let mut fv = p.free_vars();
        fv.extend(q.free_vars());

        // panic!("verification failed.");
        // panic!(format!("ERROR Result is {:?}", res));

        ea(Err(EntailmentError {
            entailment_src: p.clone(),
            entailment_dst: q.clone(),
            is_unknown: false,
            untrue_model: Some(map_of_model(&ctx, solver.get_model().unwrap(), fv)),
        }.into()));
    }

    if !errs.0.is_empty() {
        return Err(errs);
    }

    Ok(())
}

fn map_of_model(ctx: &z3::Context, model: Model, fv: HashSet<Var>) -> HashMap<String, i64> {
    fv.into_iter().map(|v| {
        (v.clone(), model.eval(&z3::ast::Int::new_const(ctx, v.as_str()), true).unwrap().as_i64().unwrap())
    }).collect()
}

fn entails<'a>(ctx: &'a z3::Context, a: z3::ast::Bool<'a>, b: z3::ast::Bool<'a>) -> z3::ast::Bool<'a> {
    z3::ast::Bool::and(ctx, &[&a, &b.not()])
}