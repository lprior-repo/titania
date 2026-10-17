use thiserror::Error;

const CAP_LINTS_FLAG: &str = "--cap-lints";
const CAP_LINTS_ASSIGN: &str = "--cap-lints=";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvocationError {
    #[error("extra argument {arg:?} would cap mandatory lint severity below the lane denies")]
    CapLints { arg: String },
}

#[must_use]
pub fn is_cap_lints_flag(arg: &str) -> bool {
    arg == CAP_LINTS_FLAG || arg.starts_with(CAP_LINTS_ASSIGN)
}

pub fn validate_extra_args(extra_args: &[String]) -> Result<(), InvocationError> {
    let ceiling = extra_args.iter().find(|arg| is_cap_lints_flag(arg));
    ceiling.map_or(Ok(()), |arg| Err(InvocationError::CapLints { arg: arg.clone() }))
}
