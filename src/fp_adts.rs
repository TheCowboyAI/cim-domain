// Copyright (c) 2025 - Cowboy AI, LLC.

//! Simple algebraic data types (ADTs) used in the FP style APIs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A standard Either ADT: Left(L) or Right(R).
///
/// In this codebase we follow the convention that `Left` holds the
/// alternate representation (e.g., a content address) and `Right`
/// holds the domain value. For event payloads specifically:
/// `Either<DomainCid, E>` means `Left(cid)` or `Right(event)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "value")]
pub enum Either<L, R> {
    /// Left branch
    Left(L),
    /// Right branch
    Right(R),
}

impl<L, R> Either<L, R> {
    /// Map over the Right value.
    pub fn map<T, F>(self, f: F) -> Either<L, T>
    where
        F: FnOnce(R) -> T,
    {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(f(r)),
        }
    }

    /// Get a reference to the Left value if present.
    pub fn left(&self) -> Option<&L> {
        match self {
            Either::Left(l) => Some(l),
            _ => None,
        }
    }

    /// Get a reference to the Right value if present.
    pub fn right(&self) -> Option<&R> {
        match self {
            Either::Right(r) => Some(r),
            _ => None,
        }
    }
}
