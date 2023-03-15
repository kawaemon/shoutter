pub fn only_ascii(checking_str: &str) -> Result<(), ValidationError> {
    if checking_str.is_ascii() {
        Ok(())
    } else {
        Err(ValidationError::new("non_ascii_char_included"))
    }
}

