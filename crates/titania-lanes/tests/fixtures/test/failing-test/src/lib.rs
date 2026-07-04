pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn fails() {
        assert_eq!(super::value(), 2);
    }
}
