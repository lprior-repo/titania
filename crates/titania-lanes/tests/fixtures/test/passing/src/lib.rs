pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn passes() {
        assert_eq!(super::value(), 1);
    }
}
