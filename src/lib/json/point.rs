use crate::json::defaults;
use crate::json::Symbol;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Point {
    #[serde(default = "defaults::point_access")]
    pub access: PointAccess,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default = "defaults::point_mandatory")]
    pub mandatory: PointMandatory,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sf: Option<PointSf>,
    pub size: i64,
    #[serde(rename = "static", default = "defaults::point_static")]
    pub static_: PointStatic,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<Symbol>,
    #[serde(rename = "type")]
    pub type_: PointType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<PointValue>,
}

impl From<&Point> for Point {
    fn from(value: &Point) -> Self {
        value.clone()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PointAccess {
    R,
    #[serde(rename = "RW")]
    Rw,
}

impl From<&PointAccess> for PointAccess {
    fn from(value: &PointAccess) -> Self {
        value.clone()
    }
}

impl ToString for PointAccess {
    fn to_string(&self) -> String {
        match *self {
            Self::R => "R".to_string(),
            Self::Rw => "RW".to_string(),
        }
    }
}

impl std::str::FromStr for PointAccess {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        match value {
            "R" => Ok(Self::R),
            "RW" => Ok(Self::Rw),
            _ => Err("invalid value".into()),
        }
    }
}

impl std::convert::TryFrom<&str> for PointAccess {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointAccess {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointAccess {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl Default for PointAccess {
    fn default() -> Self {
        PointAccess::R
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PointMandatory {
    M,
    O,
}

impl From<&PointMandatory> for PointMandatory {
    fn from(value: &PointMandatory) -> Self {
        value.clone()
    }
}

impl ToString for PointMandatory {
    fn to_string(&self) -> String {
        match *self {
            Self::M => "M".to_string(),
            Self::O => "O".to_string(),
        }
    }
}

impl std::str::FromStr for PointMandatory {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        match value {
            "M" => Ok(Self::M),
            "O" => Ok(Self::O),
            _ => Err("invalid value".into()),
        }
    }
}

impl std::convert::TryFrom<&str> for PointMandatory {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointMandatory {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointMandatory {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl Default for PointMandatory {
    fn default() -> Self {
        PointMandatory::O
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PointSf {
    String(String),
    Integer(i64),
}

impl From<&PointSf> for PointSf {
    fn from(value: &PointSf) -> Self {
        value.clone()
    }
}

impl std::str::FromStr for PointSf {
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

impl std::convert::TryFrom<&str> for PointSf {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointSf {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointSf {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl ToString for PointSf {
    fn to_string(&self) -> String {
        match self {
            Self::String(x) => x.to_string(),
            Self::Integer(x) => x.to_string(),
        }
    }
}

impl From<i64> for PointSf {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PointStatic {
    D,
    S,
}

impl From<&PointStatic> for PointStatic {
    fn from(value: &PointStatic) -> Self {
        value.clone()
    }
}

impl ToString for PointStatic {
    fn to_string(&self) -> String {
        match *self {
            Self::D => "D".to_string(),
            Self::S => "S".to_string(),
        }
    }
}

impl std::str::FromStr for PointStatic {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        match value {
            "D" => Ok(Self::D),
            "S" => Ok(Self::S),
            _ => Err("invalid value".into()),
        }
    }
}

impl std::convert::TryFrom<&str> for PointStatic {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointStatic {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointStatic {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl Default for PointStatic {
    fn default() -> Self {
        PointStatic::D
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum PointType {
    #[serde(rename = "int16")]
    Int16,
    #[serde(rename = "int32")]
    Int32,
    #[serde(rename = "int64")]
    Int64,
    #[serde(rename = "raw16")]
    Raw16,
    #[serde(rename = "uint16")]
    Uint16,
    #[serde(rename = "uint32")]
    Uint32,
    #[serde(rename = "uint64")]
    Uint64,
    #[serde(rename = "acc16")]
    Acc16,
    #[serde(rename = "acc32")]
    Acc32,
    #[serde(rename = "acc64")]
    Acc64,
    #[serde(rename = "bitfield16")]
    Bitfield16,
    #[serde(rename = "bitfield32")]
    Bitfield32,
    #[serde(rename = "bitfield64")]
    Bitfield64,
    #[serde(rename = "enum16")]
    Enum16,
    #[serde(rename = "enum32")]
    Enum32,
    #[serde(rename = "float32")]
    Float32,
    #[serde(rename = "float64")]
    Float64,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "sf")]
    Sf,
    #[serde(rename = "pad")]
    Pad,
    #[serde(rename = "ipaddr")]
    Ipaddr,
    #[serde(rename = "ipv6addr")]
    Ipv6addr,
    #[serde(rename = "eui48")]
    Eui48,
    #[serde(rename = "sunssf")]
    Sunssf,
    #[serde(rename = "count")]
    Count,
}

impl From<&PointType> for PointType {
    fn from(value: &PointType) -> Self {
        value.clone()
    }
}

impl ToString for PointType {
    fn to_string(&self) -> String {
        match *self {
            Self::Int16 => "int16".to_string(),
            Self::Int32 => "int32".to_string(),
            Self::Int64 => "int64".to_string(),
            Self::Raw16 => "raw16".to_string(),
            Self::Uint16 => "uint16".to_string(),
            Self::Uint32 => "uint32".to_string(),
            Self::Uint64 => "uint64".to_string(),
            Self::Acc16 => "acc16".to_string(),
            Self::Acc32 => "acc32".to_string(),
            Self::Acc64 => "acc64".to_string(),
            Self::Bitfield16 => "bitfield16".to_string(),
            Self::Bitfield32 => "bitfield32".to_string(),
            Self::Bitfield64 => "bitfield64".to_string(),
            Self::Enum16 => "enum16".to_string(),
            Self::Enum32 => "enum32".to_string(),
            Self::Float32 => "float32".to_string(),
            Self::Float64 => "float64".to_string(),
            Self::String => "string".to_string(),
            Self::Sf => "sf".to_string(),
            Self::Pad => "pad".to_string(),
            Self::Ipaddr => "ipaddr".to_string(),
            Self::Ipv6addr => "ipv6addr".to_string(),
            Self::Eui48 => "eui48".to_string(),
            Self::Sunssf => "sunssf".to_string(),
            Self::Count => "count".to_string(),
        }
    }
}

impl std::str::FromStr for PointType {
    type Err = crate::json::error::ConversionError;
    fn from_str(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        match value {
            "int16" => Ok(Self::Int16),
            "int32" => Ok(Self::Int32),
            "int64" => Ok(Self::Int64),
            "raw16" => Ok(Self::Raw16),
            "uint16" => Ok(Self::Uint16),
            "uint32" => Ok(Self::Uint32),
            "uint64" => Ok(Self::Uint64),
            "acc16" => Ok(Self::Acc16),
            "acc32" => Ok(Self::Acc32),
            "acc64" => Ok(Self::Acc64),
            "bitfield16" => Ok(Self::Bitfield16),
            "bitfield32" => Ok(Self::Bitfield32),
            "bitfield64" => Ok(Self::Bitfield64),
            "enum16" => Ok(Self::Enum16),
            "enum32" => Ok(Self::Enum32),
            "float32" => Ok(Self::Float32),
            "float64" => Ok(Self::Float64),
            "string" => Ok(Self::String),
            "sf" => Ok(Self::Sf),
            "pad" => Ok(Self::Pad),
            "ipaddr" => Ok(Self::Ipaddr),
            "ipv6addr" => Ok(Self::Ipv6addr),
            "eui48" => Ok(Self::Eui48),
            "sunssf" => Ok(Self::Sunssf),
            "count" => Ok(Self::Count),
            _ => Err("invalid value".into()),
        }
    }
}

impl std::convert::TryFrom<&str> for PointType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointType {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum PointValue {
    String(String),
    Integer(i64),
}

impl From<&PointValue> for PointValue {
    fn from(value: &PointValue) -> Self {
        value.clone()
    }
}

impl std::str::FromStr for PointValue {
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

impl std::convert::TryFrom<&str> for PointValue {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &str) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<&String> for PointValue {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: &String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl std::convert::TryFrom<String> for PointValue {
    type Error = crate::json::error::ConversionError;
    fn try_from(value: String) -> Result<Self, crate::json::error::ConversionError> {
        value.parse()
    }
}

impl ToString for PointValue {
    fn to_string(&self) -> String {
        match self {
            Self::String(x) => x.to_string(),
            Self::Integer(x) => x.to_string(),
        }
    }
}

impl From<i64> for PointValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}
