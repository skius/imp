use super::ast::*;
use super::state::*;
use super::expression::{arithmetic_eval, boolean_eval};

pub fn run(initial: Configuration) -> State {
    let (stm, mut initial_state) = match initial {
        Configuration::Terminal(s) => return s,
        Configuration::Nonterminal(stm, s) => (stm, s),
    };

    match *stm.clone() {
        Stm::Skip => initial_state,
        Stm::Assign(x, e) => {
            initial_state.update(&x, arithmetic_eval(&e, &initial_state));
            initial_state
        },
        Stm::Seq(stm1, stm2) => {
            let state1 = run(Configuration::Nonterminal(stm1, initial_state));
            run(Configuration::Nonterminal(stm2, state1))
        },
        Stm::If(cond, stm_then, stm_else) => {
            if boolean_eval(&cond, &initial_state) {
                run(Configuration::Nonterminal(stm_then, initial_state))
            } else {
                run(Configuration::Nonterminal(stm_else, initial_state))
            }
        },
        Stm::While(cond, stm_inner) => {
            if boolean_eval(&cond, &initial_state) {
                let state1 = run(Configuration::Nonterminal(stm_inner, initial_state));

                run(Configuration::Nonterminal(stm, state1))
            } else {
                initial_state
            }
        }

    }
}