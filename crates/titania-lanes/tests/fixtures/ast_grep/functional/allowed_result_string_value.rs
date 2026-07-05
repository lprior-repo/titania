use std::collections::HashMap;

pub type Names = HashMap<UserId, String>;

pub struct UserId;
pub struct LoadError;

pub fn load_names() -> Result<Names, LoadError> {
    Ok(HashMap::new())
}
