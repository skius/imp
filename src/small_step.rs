use super::state::*;
use super::ast::*;
use super::expression::{arithmetic_eval, boolean_eval};

pub struct SOS{
    config: Configuration,
    done: bool,
}

impl SOS {
    pub fn new(config: Configuration) -> Self {
        SOS {config, done: false}
    }

    pub fn run_execution(&mut self) {
        let mut it = self.peekable();
        print!("   ");
        while it.peek().is_some() && it.peek().unwrap().is_nonterminal()  {
            print!("{:?}\n-> ", it.next().unwrap())
        }
        // The iterator never stops unless its a Terminal state, hence the next iterator
        // Option<Configuration> should be Some(Terminal)
        print!("{:?}", it.next().unwrap())

    }
}

impl Iterator for SOS {
    type Item = Configuration;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let old_config = self.config.clone();

        if let Configuration::Terminal(_) = self.config {
            self.done = true;
        } else {
            self.config = transition(self.config.clone());
        }


        return Some(old_config);
    }
}

pub fn transition(initial: Configuration) -> Configuration {
    let (stm, mut initial_state) = match initial {
        Configuration::Terminal(s) => return Configuration::Terminal(s),
        Configuration::Nonterminal(stm, s) => (stm, s),
    };

    match *stm.clone() {
        Stm::Skip => Configuration::Terminal(initial_state),
        Stm::Assign(x, e) => {
            initial_state.update(&x, arithmetic_eval(&e, &initial_state));
            Configuration::Terminal(initial_state)
        },
        Stm::Seq(stm1, stm2) => {
            let config1 = transition(Configuration::Nonterminal(stm1, initial_state));
            match config1 {
                Configuration::Terminal(state1) => Configuration::Nonterminal(stm2, state1),
                Configuration::Nonterminal(stm11, state1) => {
                    Configuration::Nonterminal(
                        Box::new(Stm::Seq(stm11, stm2)),
                        state1
                    )
                },
            }
        },
        Stm::If(cond, stm_then, stm_else) => {
            if boolean_eval(&cond, &initial_state) {
                Configuration::Nonterminal(stm_then, initial_state)
            } else {
                Configuration::Nonterminal(stm_else, initial_state)
            }
        },
        Stm::While(cond, stm_inner) => {
            Configuration::Nonterminal(
                Box::new(Stm::If(
                    cond.clone(),
                    Box::new(Stm::Seq(
                        stm_inner.clone(),
                        Box::new(Stm::While(cond, stm_inner))
                    )),
                    Box::new(Stm::Skip),
                )),
                initial_state
            )
        },
    }
}