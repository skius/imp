use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use lalrpop_util::ParseError;
use crate::ast::{AxStm, Bexp};

pub fn err_acc(errs: &mut ImpErrors, res: Result<(), ImpErrors>) {
    if let Err(e) = res {
        errs.0.extend(e.0);
    }
}

#[derive(Debug, Clone)]
pub struct ImpErrors(pub Vec<ImpErrorInner>);

impl From<ImpErrorInner> for ImpErrors {
    fn from(err: ImpErrorInner) -> Self {
        ImpErrors(vec![err])
    }
}

impl<L, T, E> From<ParseError<L, T, E>> for ImpErrors
    where
        L: Display,
        T: Display,
        E: Display,
{
    fn from(err: ParseError<L, T, E>) -> Self {
        // TODO: improve
        ImpErrorInner::ParseError {
            msg: format!("{}", err),
        }.into()
    }
}


// pub fn err_acc(f: F) -> ()
// where
//     F: FnOnce(Result<(), ImpError>) -> i64,
// {
//
// }

#[derive(Debug, Clone)]
pub enum ImpErrorInner {
    ParseError {
        msg: String,
    },
    EntailmentError {
        entailment_src: Bexp,
        entailment_dst: Bexp,
        //TODO: change these two together into one enum perhaps
        is_unknown: bool,
        untrue_model: Option<HashMap<String, i64>>
    },
    AxStructureError {
        actual: Bexp,
        expected: Bexp,
        stm: AxStm,
    },
    Other(String),
}

pub fn string_of_model(model: &HashMap<String, i64>) -> String {
    let mut res = String::new();
    res.push_str("{ ");
    let mut first = true;
    for key in model.keys() {
        if first {
            res.push_str(&format!("{}={}", key, model[key]));
            first = false;
        } else {
            res.push_str(&format!(", {}={}", key, model[key]));
        }
    }
    res.push_str(" }");
    res
}

impl Display for ImpErrorInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpErrorInner::EntailmentError { entailment_src, entailment_dst, is_unknown: true, untrue_model } => {
                f.write_str(&format!(
                    "Was unable to prove the following entailment:\n{}\n|=\n{}\nThis does not mean your proof is incorrect, but possibly that there is too much logic in this step.",
                    entailment_src.pretty_string(),
                    entailment_dst.pretty_string(),
                ))
            },
            ImpErrorInner::EntailmentError { entailment_src, entailment_dst, is_unknown: false, untrue_model: Some(model) } => {
                f.write_str(&format!(
                    "The following entailment is incorrect:\
                    \n{}\n|=\n{}\n\
                    It does not hold in the following model:\n{}",
                    entailment_src.pretty_string(),
                    entailment_dst.pretty_string(),
                    string_of_model(model),
                ))
            },
            other => f.write_str(&format!("{:?}", other))
        }
    }
}