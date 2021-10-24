#[macro_use] extern crate lalrpop_util;

lalrpop_mod!(pub imp_lang); // synthesized by LALRPOP
pub mod ast;
pub mod state;
pub mod big_step;
pub mod small_step;
pub mod expression;
pub mod axiomatic;
pub mod entailment;