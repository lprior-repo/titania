pub const BUILD_GIT_SHA: &str = match option_env!("TITANIA_BUILD_GIT_SHA") {
    Some(value) => value,
    None => "unknown",
};

pub const WORKSPACE_NAME: &str = match option_env!("TITANIA_WORKSPACE_NAME") {
    Some(value) => value,
    None => "titania",
};
