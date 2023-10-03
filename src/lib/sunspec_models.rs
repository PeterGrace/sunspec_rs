use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ValueType {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Array(Vec<String>),
    Pad,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Access {
    #[serde(rename = "r")]
    ReadOnly,
    #[serde(rename = "rw")]
    ReadWrite,
}
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(tag = "untagged")]
pub enum PointType {
    #[serde(rename = "string")]
    String(ValueType),
    #[serde(rename = "int16")]
    Int16(ValueType),
    #[serde(rename = "uint16")]
    UnsignedInt16(ValueType),
    #[serde(rename = "acc16")]
    Accumulator16(ValueType),
    #[serde(rename = "enum16")]
    Enum16(ValueType),
    #[serde(rename = "bitfield16")]
    Bitfield16(ValueType),
    #[serde(rename = "int32")]
    Int32(ValueType),
    #[serde(rename = "uint32")]
    UnsignedInt32(ValueType),
    #[serde(rename = "acc32")]
    Accumulator32(ValueType),
    #[serde(rename = "enum32")]
    Enum32(ValueType),
    #[serde(rename = "bitfield32")]
    Bitfield32(ValueType),
    #[serde(rename = "sunssf")]
    SunSpecScaleFactor(ValueType),
    #[serde(rename = "pad")]
    #[default]
    Pad,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Model {
    pub id: u16,
    pub len: u16,
    pub name: String,
    pub block: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Block {
    pub(crate) len: u16,
    pub(crate) r#type: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) point: Vec<Point>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Point {
    pub id: String,
    pub offset: u16,
    pub r#type: String,
    pub len: Option<u16>,
    pub mandatory: Option<bool>,
    pub access: Option<Access>,
    pub symbol: Option<Vec<Symbol>>,
    pub units: Option<String>,
    #[serde(rename = "sf")]
    pub scale_factor: Option<String>,
    pub value: Option<ValueType>,
    pub literal: Option<PointLiteral>,
    pub block_id: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub id: String,
    #[serde(rename = "$value")]
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Strings {
    pub(crate) id: String,
    pub(crate) locale: Option<String>,
    #[serde(rename = "$value")]
    pub(crate) literals: Vec<LiteralType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelLiteral {
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PointLiteral {
    pub id: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SymbolLiteral {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LiteralType {
    #[serde(rename = "model")]
    Model(ModelLiteral),
    #[serde(rename = "point")]
    Point(PointLiteral),
    #[serde(rename = "symbol")]
    Symbol(SymbolLiteral),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SunSpecModels {
    pub model: Model,
    pub strings: Vec<Strings>,
}
