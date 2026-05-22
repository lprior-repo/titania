#![forbid(unsafe_code)]

use crate::assure::model::{
    AssureError, AssureResult, DecisionPath, EnumVariantId, ExpectedOutcome, Expr, FactDomain,
    PathId, VarId,
};
use crate::assure::oracle::OracleRecord;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type Valuation = BTreeMap<VarId, EnumVariantId>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathCheckInput {
    pub domains: Vec<FactDomain>,
    pub paths: Vec<DecisionPath>,
    pub required_error_outcomes: Vec<String>,
    pub oracles: Vec<OracleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathCheckReport {
    pub valuations_checked: usize,
    pub path_hits: BTreeMap<PathId, usize>,
    pub failures: Vec<PathCheckFailure>,
}

impl PathCheckReport {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathCheckFailure {
    UncoveredValuation {
        valuation: Valuation,
    },
    OverlappingValuation {
        valuation: Valuation,
        paths: Vec<PathId>,
    },
    UnreachablePath {
        path: PathId,
    },
    ErrorOutcomeWithoutPath {
        error: String,
    },
    OracleMapsToNoPath {
        oracle: String,
        path: PathId,
        outcome: ExpectedOutcome,
    },
}

pub fn check_paths(input: &PathCheckInput) -> AssureResult<PathCheckReport> {
    let valuations = enumerate_valuations(&input.domains)?;
    let totality_failures = valuation_failures(&valuations, &input.paths)?;
    let path_hits = count_path_hits(&valuations, &input.paths)?;
    let structural_failures = structural_failures(input, &path_hits);
    let failures = totality_failures
        .into_iter()
        .chain(structural_failures)
        .collect();
    Ok(PathCheckReport {
        valuations_checked: valuations.len(),
        path_hits,
        failures,
    })
}

pub fn evaluate_expr(expr: &Expr, valuation: &Valuation) -> AssureResult<bool> {
    match expr {
        Expr::True => Ok(true),
        Expr::False => Ok(false),
        Expr::Eq { var, value } => evaluate_eq(var, value, valuation),
        Expr::Neq { var, value } => evaluate_eq(var, value, valuation).map(|result| !result),
        Expr::And(items) => items.iter().try_fold(true, |matched, item| {
            if matched {
                evaluate_expr(item, valuation)
            } else {
                Ok(false)
            }
        }),
        Expr::Or(items) => items.iter().try_fold(false, |matched, item| {
            if matched {
                Ok(true)
            } else {
                evaluate_expr(item, valuation)
            }
        }),
        Expr::Not(item) => evaluate_expr(item, valuation).map(|result| !result),
    }
}

fn evaluate_eq(var: &VarId, value: &EnumVariantId, valuation: &Valuation) -> AssureResult<bool> {
    valuation
        .get(var)
        .map(|actual| actual == value)
        .ok_or_else(|| AssureError::UnknownVar {
            var: var.as_str().to_string(),
        })
}

fn enumerate_valuations(domains: &[FactDomain]) -> AssureResult<Vec<Valuation>> {
    domains
        .iter()
        .try_fold(vec![BTreeMap::new()], expand_domain)
}

fn expand_domain(valuations: Vec<Valuation>, domain: &FactDomain) -> AssureResult<Vec<Valuation>> {
    let variants = domain.enum_variants()?;
    Ok(valuations
        .iter()
        .flat_map(|valuation| {
            variants
                .iter()
                .map(|variant| valuation_with(valuation, &domain.var, variant))
        })
        .collect())
}

fn valuation_with(valuation: &Valuation, var: &VarId, variant: &EnumVariantId) -> Valuation {
    let mut next = valuation.clone();
    next.insert(var.clone(), variant.clone());
    next
}

fn valuation_failures(
    valuations: &[Valuation],
    paths: &[DecisionPath],
) -> AssureResult<Vec<PathCheckFailure>> {
    valuations
        .iter()
        .map(|valuation| {
            matching_paths(valuation, paths).map(|matches| totality_failure(valuation, matches))
        })
        .collect::<AssureResult<Vec<_>>>()
        .map(|failures| failures.into_iter().flatten().collect())
}

fn totality_failure(valuation: &Valuation, matches: Vec<PathId>) -> Option<PathCheckFailure> {
    match matches.as_slice() {
        [] => Some(PathCheckFailure::UncoveredValuation {
            valuation: valuation.clone(),
        }),
        [_one] => None,
        _ => Some(PathCheckFailure::OverlappingValuation {
            valuation: valuation.clone(),
            paths: matches,
        }),
    }
}

fn matching_paths(valuation: &Valuation, paths: &[DecisionPath]) -> AssureResult<Vec<PathId>> {
    paths
        .iter()
        .map(|path| {
            evaluate_expr(&path.guard, valuation).map(|matched| matched.then_some(path.id.clone()))
        })
        .collect::<AssureResult<Vec<_>>>()
        .map(|matches| matches.into_iter().flatten().collect())
}

fn count_path_hits(
    valuations: &[Valuation],
    paths: &[DecisionPath],
) -> AssureResult<BTreeMap<PathId, usize>> {
    paths
        .iter()
        .map(|path| path_hit_count(valuations, path).map(|count| (path.id.clone(), count)))
        .collect()
}

fn path_hit_count(valuations: &[Valuation], path: &DecisionPath) -> AssureResult<usize> {
    valuations.iter().try_fold(0usize, |count, valuation| {
        evaluate_expr(&path.guard, valuation).and_then(|matched| {
            count
                .checked_add(usize::from(matched))
                .ok_or(AssureError::CounterOverflow {
                    context: "path hit counting",
                })
        })
    })
}

fn structural_failures(
    input: &PathCheckInput,
    path_hits: &BTreeMap<PathId, usize>,
) -> Vec<PathCheckFailure> {
    unreachable_path_failures(path_hits)
        .into_iter()
        .chain(error_outcome_failures(input))
        .chain(oracle_mapping_failures(input))
        .collect()
}

fn unreachable_path_failures(path_hits: &BTreeMap<PathId, usize>) -> Vec<PathCheckFailure> {
    path_hits
        .iter()
        .filter(|(_path, hits)| **hits == 0)
        .map(|(path, _hits)| PathCheckFailure::UnreachablePath { path: path.clone() })
        .collect()
}

fn error_outcome_failures(input: &PathCheckInput) -> Vec<PathCheckFailure> {
    let produced_errors = input
        .paths
        .iter()
        .filter_map(|path| path.outcome.error_name().map(str::to_string))
        .collect::<BTreeSet<_>>();
    input
        .required_error_outcomes
        .iter()
        .filter(|error| !produced_errors.contains(error.as_str()))
        .map(|error| PathCheckFailure::ErrorOutcomeWithoutPath {
            error: error.clone(),
        })
        .collect()
}

fn oracle_mapping_failures(input: &PathCheckInput) -> Vec<PathCheckFailure> {
    let path_ids = input
        .paths
        .iter()
        .map(|path| path.id.clone())
        .collect::<BTreeSet<_>>();
    input
        .oracles
        .iter()
        .filter(|oracle| !path_ids.contains(&oracle.path))
        .map(|oracle| PathCheckFailure::OracleMapsToNoPath {
            oracle: oracle.id.as_str().to_string(),
            path: oracle.path.clone(),
            outcome: oracle.expected.clone(),
        })
        .collect()
}
