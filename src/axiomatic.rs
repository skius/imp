use super::ast::*;
use super::state::*;
use z3::SatResult;


pub fn verify_rules_except_cons(s: Box<StmAx>) {
    match *s {
        StmAx::Skip(pre, post) => assert_eq!(pre, post),
        StmAx::Assign(pre, var, aexp, post) => {
            let post_substituted = post.substitute(&var, &aexp);
            assert_eq!(post_substituted, *pre)
        },
        StmAx::Seq(stm1,  stm2) => {
            verify_rules_except_cons(stm1);
            verify_rules_except_cons(stm2);
        },
        StmAx::If(pre, cond, then_stm, else_stm, post) => {
            let then_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
            let else_pre_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
            assert_eq!(then_pre_must, *then_stm.get_pre());
            assert_eq!(else_pre_must, *else_stm.get_pre());
            assert_eq!(post, then_stm.get_post());
            assert_eq!(post, else_stm.get_post());
        },
        StmAx::While(pre, cond, stm_inner, post) => {
            let inner_pre_must = Bexp::Bop(cond.clone(), Bopcode::And, pre.clone());
            assert_eq!(inner_pre_must, *stm_inner.get_pre());
            assert_eq!(pre, stm_inner.get_post());
            let post_must = Bexp::Bop(Box::new(Bexp::Not(cond)), Bopcode::And, pre);
            assert_eq!(post_must, *post)
        }
    }
}

pub fn verify_cons(s: Box<StmAx>) {
    let config = z3::Config::new();
    let context = &z3::Context::new(&config);

    verify_cons_single(context, s);
    println!("Successfully verified program.");
}

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