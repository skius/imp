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
        println!("Usage: ./imp <filename> <true/false: run big-step> <true/false: run small-step> <total/partial/false: run axiomatic>");
        println!("Example: ./imp examples/square.imp false false partial");
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
        let prog_res = imp::StmParser::new().parse(contents.as_str());
        let prog = if prog_res.is_err() {
            imp::AxBlockParser::new().parse(contents.as_str()).unwrap().into_stm()
        } else {
            prog_res.unwrap()
        };
        // let prog = imp::StmParser::new().parse(contents.as_str()).unwrap_or(imp::AxBlockParser::new().parse(contents.as_str()).unwrap().into_stm());
        println!("\nRunning big-step evaluator...");
        println!("Big-step result: {:?}", big_step::run(Configuration::Nonterminal(prog, State::new())));
    }
    if run_small == "true" {
        // Allow both pure IMP syntax and pre/post-condition syntax
        let prog_res = imp::StmParser::new().parse(contents.as_str());
        let prog = if prog_res.is_err() {
            imp::AxBlockParser::new().parse(contents.as_str()).unwrap().into_stm()
        } else {
            prog_res.unwrap()
        };
        // let prog = imp::StmParser::new().parse(contents.as_str()).unwrap_or(imp::AxBlockParser::new().parse(contents.as_str()).unwrap().into_stm());
        println!("\nRunning small-step evaluator...");
        let mut sos = small_step::SOS::new(Configuration::Nonterminal(prog.clone(), State::new()));
        sos.run_execution();
    }

    if run_axiomatic == "partial" || run_axiomatic == "true" {
        // Force syntax with pre/post-conditions
        let prog = imp::AxBlockParser::new().parse(contents.as_str()).unwrap();
        println!("\nVerifying partial correctness for program using axiomatic semantics...");
        println!("{:?}\n", prog);

        let cfg = z3::Config::new();
        axiomatic::verify_block_except_cons_partial(&prog);
        axiomatic::verify_cons_partial(&cfg, &prog);
        println!("Successfully verified partial correctness of program.");
    }

    if run_axiomatic == "total" {
        // Force syntax with pre/post-conditions
        let prog = imp::AxBlockParser::new().parse(contents.as_str()).unwrap();
        println!("\nVerifying total correctness for program using axiomatic semantics...");
        println!("{:?}\n", prog);

        let cfg = z3::Config::new();
        axiomatic::verify_block_except_cons_total(&prog);
        axiomatic::verify_cons_total(&cfg, &prog);
        println!("Successfully verified total correctness of program.");
    }
}
