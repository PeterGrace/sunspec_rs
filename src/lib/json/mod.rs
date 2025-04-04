#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

pub mod group;
pub mod misc;
pub mod point;

use crate::json::group::GroupCount;
use serde::{Deserialize, Serialize};
use serde_json::*;

pub mod error {
    #[doc = r" Error from a TryFrom or FromStr implementation."]
    pub struct ConversionError(std::borrow::Cow<'static, str>);

    impl std::error::Error for ConversionError {}

    impl std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            std::fmt::Display::fmt(&self.0, f)
        }
    }

    impl std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            std::fmt::Debug::fmt(&self.0, f)
        }
    }

    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }

    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Symbol {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub value: serde_json::Value,
}

impl From<&Symbol> for Symbol {
    fn from(value: &Symbol) -> Self {
        value.clone()
    }
}

pub mod defaults {
    use crate::json::point::{PointAccess, PointMandatory, PointStatic};

    pub(crate) fn group_count() -> super::GroupCount {
        super::GroupCount::Integer(1_i64)
    }

    pub(crate) fn point_access() -> PointAccess {
        PointAccess::R
    }

    pub(crate) fn point_mandatory() -> PointMandatory {
        PointMandatory::O
    }

    pub(crate) fn point_static() -> PointStatic {
        PointStatic::D
    }
}
