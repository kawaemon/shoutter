use std::{fmt::Display, marker::PhantomData, str::FromStr};

use derivative::Derivative;
use uuid::Uuid;

#[derive(Derivative)]
#[derivative(
    Clone(bound = ""),
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct Id<T>(Uuid, #[derivative(Debug = "ignore")] PhantomData<fn() -> T>);

impl<T> FromStr for Id<T> {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s).map(|uuid| Id(uuid, PhantomData))
    }
}

impl<T> Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
