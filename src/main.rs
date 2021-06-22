#[macro_use] extern crate lalrpop_util;

lalrpop_mod!(pub imp); // synthesized by LALRPOP
pub mod ast;
pub mod state;
pub mod big_step;
pub mod small_step;
pub mod expression;
pub mod axiomatic;

use state::{Configuration, State};
use ast::*;
use std::env;
use std::fs;
use z3;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() < 5 {
        println!("Usage: ./imp <filename> <true/false: run Big-Step> <true/false: run Small-Step> <true/false: run Axiomatic>");
        println!("Example: ./imp examples/square.imp false false true");
        return;
    }

    let filename = &args[1];

    let run_big = &args[2];
    let run_small = &args[3];
    let run_axiomatic = &args[4];
    println!("Reading file {}...", filename);

    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    if run_big == "true" {
        // Allow both pure IMP syntax and pre/post-condition syntax
        let prog = imp::StmParser::new().parse(contents.as_str()).unwrap_or(imp::StmAxParser::new().parse(contents.as_str()).unwrap().into_stm());
        println!("\nRunning Big-Step evaluator...");
        println!("Big-Step result: {:?}", big_step::run(Configuration::Nonterminal(prog, State::new())));
    }
    if run_small == "true" {
        // Allow both pure IMP syntax and pre/post-condition syntax
        let prog = imp::StmParser::new().parse(contents.as_str()).unwrap_or(imp::StmAxParser::new().parse(contents.as_str()).unwrap().into_stm());
        println!("\nRunning Small-Step evaluator...");
        let mut sos = small_step::SOS::new(Configuration::Nonterminal(prog.clone(), State::new()));
        sos.run_execution();
    }
    if run_axiomatic == "true" {
        // Force syntax with pre/post-conditions
        let prog = imp::StmAxParser::new().parse(contents.as_str()).unwrap();
        println!("\nVerifying Axiomatic Semantics for program...");
        axiomatic::verify_rules_except_cons(prog.clone());
        axiomatic::verify_cons(prog);
    }
}
