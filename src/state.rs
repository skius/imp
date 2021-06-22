use super::ast::{Stm, Var};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Configuration {
    Terminal(State),
    Nonterminal(Box<Stm>, State),
}

impl Configuration {
    pub fn is_terminal(&self) -> bool {
        match self {
            Configuration::Terminal(_) => true,
            Configuration::Nonterminal(_, _) => false,
        }
    }

    pub fn is_nonterminal(&self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Debug, Clone)]
pub struct State(HashMap<Var, i64>);

impl State {
    pub fn new() -> Self {
        State(HashMap::new())
    }

    pub fn update(&mut self, v: &Var, val: i64) {
        self.0.insert(v.to_string(), val);
    }

    pub fn get(&self, v: &Var) -> i64 {
        *self.0.get(v).unwrap_or(&0)
    }
}