use super::ast::*;
use super::state::*;
use z3::SatResult;

pub fn verify_block_except_cons(AxBlock(AssertionChain(first), rem): AxBlock) {
    let mut pre = first.last().unwrap();
    for (stm, AssertionChain(post_chain)) in &rem {
        let post = post_chain.first().unwrap();
        match stm {
            AxStm::Skip => assert_eq!(pre, post),
            AxStm::Assign(v, aexp) => {
                // println!("Substituting {:?} in {:?} for {:?}", v, post, aexp);
                let pre_must = post.clone().substitute(v, aexp);
                assert_eq!(*pre, pre_must);
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
                assert_eq!(*then_pre, then_pre_must);
                assert_eq!(*else_pre, else_pre_must);
                assert_eq!(post, then_post);
                assert_eq!(post, else_post);
            },
            AxStm::While(cond, AxBlock(AssertionChain(inner_pre_chain), inner_rem)) => {
                let inner_pre = inner_pre_chain.first().unwrap();
                let inner_post = inner_rem.last().unwrap().1.0.last().unwrap();

                let inner_pre_must = Bexp::Bop(Box::new(cond.clone()), Bopcode::And, Box::new(pre.clone()));
                assert_eq!(*inner_pre, inner_pre_must);
                assert_eq!(inner_post, pre);
                let post_must = Bexp::Bop(Box::new(Bexp::Not(Box::new(cond.clone()))), Bopcode::And, Box::new(pre.clone()));
                assert_eq!(*post, post_must)
            },
        }
        pre = post_chain.last().unwrap();
    }
}

pub fn verify_cons(cfg: &z3::Config, AxBlock(first, rem): AxBlock) {

    verify_assertion_chain(&cfg, first);
    for (stm, post_chain) in rem {
        match stm {
            AxStm::While(_, inner_block) => verify_cons(cfg, inner_block),
            AxStm::If(_, then_block, else_block) => {
                verify_cons(cfg, then_block);
                verify_cons(cfg, else_block);
            },
            _ => (),
        }
        verify_assertion_chain(cfg, post_chain);
    }
}

fn verify_assertion_chain(cfg: &z3::Config, AssertionChain(chain): AssertionChain) {
    let mut p = chain.first().unwrap();

    for q in chain.iter().skip(1) {
        let ctx = z3::Context::new(&cfg);

        println!("Verifying ConsAx rule:\n{{ {:?} }} ⊨ {{ {:?} }}", p, q);

        let p_entails_q = entails(&ctx, p.to_z3_bool(&ctx), q.to_z3_bool(&ctx));
        let mut solver = z3::Solver::new(&ctx);
        solver.assert(&p_entails_q);
        let res = solver.check();
        if res == SatResult::Unsat {
            println!("Verified.");
        } else {
            println!("ERROR {:?}! Model where entailment does not hold:", res);
            println!("{:?}", solver.get_model().unwrap());
            panic!("verification failed.");
        }

        p = q;
    }
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

fn verify_cons_single(ctx: &z3::Context, s: Box<StmAx>) {
    match *s {
        StmAx::Seq(stm1, stm2) => {
            verify_cons_single(ctx, stm1.clone());


            let post = stm1.get_post();
            let pre = stm2.get_pre();

            println!("Verifying ConsAx rule:\n{{ {:?} }} |= {{ {:?} }}", post, pre);

            let post_entails_pre = entails(ctx, post.to_z3_bool(ctx), pre.to_z3_bool(ctx));
            let mut solver = z3::Solver::new(ctx);
            solver.assert(&post_entails_pre);
            let res = solver.check();
            if res == SatResult::Unsat {
                println!("Verified.");
            } else {
                println!("ERROR! Model where entailment does not hold:");
                println!("{:?}", solver.get_model().unwrap());
                panic!("verification failed.");
            }

            verify_cons_single(ctx, stm2);
        },
        StmAx::If(_, _, then_stm, else_stm, _) => {
            verify_cons_single(ctx, then_stm);
            verify_cons_single(ctx, else_stm);
        },
        StmAx::While(_, _, stm_inner, _) => {
            verify_cons_single(ctx, stm_inner);
        }
        _ => (),
    }
}

fn entails<'a>(ctx: &'a z3::Context, a: z3::ast::Bool<'a>, b: z3::ast::Bool<'a>) -> z3::ast::Bool<'a> {
    z3::ast::Bool::and(ctx, &[&a, &b.not()])
}