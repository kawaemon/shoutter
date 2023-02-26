use crate::id::Id;

pub struct User {
    id: Id<Self>,
    name: String,
    bio: String,
}
