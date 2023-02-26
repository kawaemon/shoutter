use validator::{Validate, ValidationError};

use crate::id::Id;

fn only_ascii(checking_str: &str) -> Result<(), ValidationError> {
    if checking_str.is_ascii() {
        Ok(())
    } else {
        Err(ValidationError::new("non_ascii_char_included"))
    }
}

#[derive(Debug, Validate)]
pub struct User {
    id: Id<Self>,
    #[validate(
        custom = "only_ascii",
        length(min = 4, max = 32),
    )]
    screen_name: String,
    #[validate(length(min = 4, max = 32))]
    name: String,
    #[validate(length(min = 1, max = 512))]
    bio: String,
}

impl User {
    pub fn new(id: Id<Self>, screen_name: String, name: String, bio: String) -> Self {
        Self { id, screen_name, name, bio }
    }

}
