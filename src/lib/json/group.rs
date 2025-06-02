use crate::json::defaults;
use crate::json::point::Point;
use serde::{Deserialize, Serialize};

impl From<&Group> for Group {
    fn from(value: &Group) -> Self {
        value.clone()
    }
}

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub enum GroupType {
    #[default]
    #[serde(rename = "group")]
    Group,
    #[serde(rename = "sync")]
    Sync,
}

impl From<&GroupType> for GroupType {
    fn from(value: &GroupType) -> Self {
        value.clone()
    }
}

impl ToString for GroupType {
    fn to_string(&self) -> String {
        match *self {
            Self::Group => "group".to_string(),
            Self::Sync => "sync".to_string(),
        }
    }
}

impl std::str::FromStr for GroupType {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        match value {
            "group" => Ok(Self::Group),
            "sync" => Ok(Self::Sync),
            _ => Err("invalid value".into()),
        }
    }
}

impl std::convert::TryFrom<&str> for GroupType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for GroupType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for GroupType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default, PartialEq)]
pub struct Group {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    #[serde(default = "defaults::group_count")]
    pub count: GroupCount,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub points: Vec<Point>,
    #[serde(rename = "type")]
    pub type_: GroupType,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum GroupCount {
    String(String),
    Integer(i64),
}

impl From<&GroupCount> for GroupCount {
    fn from(value: &GroupCount) -> Self {
        value.clone()
    }
}

impl Default for GroupCount {
    fn default() -> Self {
        GroupCount::Integer(1_i64)
    }
}

impl std::str::FromStr for GroupCount {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        if let Ok(v) = value.parse() {
            Ok(Self::String(v))
        } else if let Ok(v) = value.parse() {
            Ok(Self::Integer(v))
        } else {
            Err("string conversion failed for all variants".into())
        }
    }
}

impl std::convert::TryFrom<&str> for GroupCount {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for GroupCount {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for GroupCount {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl ToString for GroupCount {
    fn to_string(&self) -> String {
        match self {
            Self::String(x) => x.to_string(),
            Self::Integer(x) => x.to_string(),
        }
    }
}

impl From<i64> for GroupCount {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}
