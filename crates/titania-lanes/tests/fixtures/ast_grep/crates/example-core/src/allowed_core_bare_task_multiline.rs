use tokio::task::{
    self,
};

pub fn total(items: &[i32]) -> i32 {
    items.iter().sum()
}
