#[macro_use] extern crate lalrpop_util;

use std::collections::HashMap;
use crate::ast::{Aexp, AxBlock, AxStm, Bexp, ImpFuncDef, Opcode, Ropcode, Stm};
use error::ImpErrorInner;
use error::ImpErrors;
use error::ImpErrorInner::*;
use crate::state::{Configuration, State};

lalrpop_mod!(pub imp_lang); // synthesized by LALRPOP
pub mod ast;
pub mod state;
pub mod big_step;
pub mod small_step;
pub mod expression;
pub mod axiomatic;
pub mod entailment;
pub mod error;

pub type Result<T> = core::result::Result<T, ImpErrors>;

fn stm_prog_from_src(src: &str) -> Result<Box<Stm>> {
    let prog_res = imp_lang::StmParser::new().parse(src);
    let prog = if prog_res.is_err() {
        imp_lang::AxBlockParser::new().parse(src)?.into_stm()
    } else {
        prog_res?
    };
    Ok(prog)
}

fn ax_from_src(src: &str) -> Result<(HashMap<String, ImpFuncDef>, AxBlock)> {
    let mut funcdefs = HashMap::new();

    funcdefs.insert("factorial".to_owned(), ImpFuncDef {
        name: "factorial".to_owned(),
        args: vec!["n".to_owned()],
        body: Aexp::Ite(
            Box::new(Bexp::Rop(Box::new(Aexp::Var("n".to_owned())), Ropcode::Le, Box::new(Aexp::Numeral(0)))),
            Box::new(Aexp::Numeral(1)),
            Box::new(Aexp::Op(
                Box::new(Aexp::Var("n".to_owned())),
                Opcode::Mul,
                Box::new(Aexp::FuncApp("factorial".to_owned(),
                                       vec![Aexp::Op(Box::new(Aexp::Var("n".to_owned())), Opcode::Sub, Box::new(Aexp::Numeral(1)))]))))
        )
    });

    let (funcdefs_vec, prog): (Vec<ImpFuncDef>, AxBlock) = imp_lang::AxProgramParser::new().parse(src)?;

    for funcdef in funcdefs_vec {
        funcdefs.insert(funcdef.name.clone(), funcdef);
    }

    Ok((funcdefs, prog))
}

fn default_z3_cfg() -> z3::Config {
    let mut cfg = z3::Config::new();
    cfg.set_timeout_msec(5000);
    cfg
}

pub fn run_big(src: &str) -> Result<State> {
    let stm = stm_prog_from_src(src)?;

    let state = big_step::run(Configuration::Nonterminal(stm, State::new()));

    Ok(state)
}

pub fn run_small(src: &str) -> Result<Configuration> {
    let stm = stm_prog_from_src(src)?;

    let mut sos = small_step::SOS::new(Configuration::Nonterminal(stm.clone(), State::new()));

    Ok(sos.run_execution())
}



pub fn run_ax_partial(src: &str) -> Result<()> {
    let (fdefs, prog) = ax_from_src(src)?;

    // Analyze structure
    axiomatic::verify_block_except_cons_partial(&prog)?;

    // Analyze entailments
    axiomatic::verify_cons_partial(&default_z3_cfg(), &prog, &fdefs)?;

    Ok(())
}

pub fn run_ax_total(src: &str) -> Result<()> {
    let (fdefs, prog) = ax_from_src(src)?;

    // Analyze structure
    axiomatic::verify_block_except_cons_total(&prog)?;

    // Analyze entailments
    axiomatic::verify_cons_total(&default_z3_cfg(), &prog, &fdefs)?;

    Ok(())
}