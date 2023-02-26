use std::collections::HashSet;

use validator::{Validate, ValidationErrors};

use crate::{id::Id, user::User};

#[derive(Debug, Validate)]
pub struct Shout {
    id: Id<Self>,
    #[validate(length(min = 4, max = 256))]
    content: String,
    likes: HashSet<Id<User>>,
}

impl Shout {
    pub fn new(
        id: Id<Self>,
        content: String,
        likes: HashSet<Id<User>>,
    ) -> Result<Self, ValidationErrors> {
        let raw_shout = Self { id, content, likes };

        match raw_shout.validate() {
            Ok(()) => Ok(raw_shout),
            Err(e) => Err(e),
        }
    }

    pub fn id(&self) -> Id<Shout> {
        self.id
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn like(&mut self, user_id: Id<User>) -> bool {
        self.likes.insert(user_id)
    }

    pub fn dislike(&mut self, user_id: Id<User>) -> bool {
        self.likes.remove(&user_id)
    }

    pub fn liked_by(&self, user_id: Id<User>) -> bool {
        self.likes.iter().position(|id| &user_id == id).is_some()
    }
}
