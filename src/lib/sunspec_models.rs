use serde::{Serialize,Deserialize};


#[derive(Serialize,Deserialize, Debug, Clone)]
pub enum ResponseType {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Array(Vec<String>),
}



#[derive(Serialize,Deserialize, Clone, Debug)]
pub enum Access {
    #[serde(rename = "r")]
    ReadOnly,
    #[serde(rename = "rw")]
    ReadWrite,
}
#[derive(Serialize,Deserialize, Default, Debug, Clone)]
#[serde(tag = "untagged")]
pub enum PointType {
    #[serde(rename = "string")]
    String(ResponseType),
    #[serde(rename = "int16")]
    Int16(ResponseType),
    #[serde(rename = "uint16")]
    UnsignedInt16(ResponseType),
    #[serde(rename = "acc16")]
    Accumulator16(ResponseType),
    #[serde(rename = "enum16")]
    Enum16(ResponseType),
    #[serde(rename = "bitfield16")]
    Bitfield16(ResponseType),
    #[serde(rename = "int32")]
    Int32(ResponseType),
    #[serde(rename = "uint32")]
    UnsignedInt32(ResponseType),
    #[serde(rename = "acc32")]
    Accumulator32(ResponseType),
    #[serde(rename = "enum32")]
    Enum32(ResponseType),
    #[serde(rename = "bitfield32")]
    Bitfield32(ResponseType),
    #[serde(rename = "sunssf")]
    SunSpecScaleFactor(ResponseType),
    #[serde(rename = "pad")]
    #[default]
    Pad,
}

#[derive(Serialize,Deserialize, Debug, Default, Clone)]
pub struct Model {
    pub id: u16,
    pub len: u16,
    pub name: String,
    pub block: Vec<Block>,
}

#[derive(Serialize,Deserialize, Debug, Default, Clone)]
pub struct Block {
    len: u16,
    r#type: Option<String>,
    pub(crate) point: Vec<Point>,
}

#[derive(Serialize,Deserialize, Debug, Default, Clone)]
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
    pub value: Option<ResponseType>,
    pub literal: Option<PointLiteral>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub(crate) id: String,
    #[serde(rename = "$value")]
    pub(crate) symbol: String,
}

#[derive(Serialize,Deserialize, Debug, Default, Clone)]
pub struct Strings {
    pub(crate) id: String,
    pub(crate) locale: Option<String>,
    #[serde(rename = "$value")]
    pub(crate) literals: Vec<LiteralType>,
}

#[derive(Serialize,Deserialize, Debug, Clone)]
pub struct ModelLiteral {
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) notes: Option<String>,
}

#[derive(Serialize,Deserialize, Debug, Clone)]
pub struct PointLiteral {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) notes: Option<String>,
}

#[derive(Serialize,Deserialize, Debug, Clone)]
pub struct SymbolLiteral {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) notes: Option<String>,
}


#[derive(Serialize,Deserialize, Debug, Clone)]
pub enum LiteralType {
    #[serde(rename = "model")]
    Model(ModelLiteral),
    #[serde(rename = "point")]
    Point(PointLiteral),
    #[serde(rename = "symbol")]
    Symbol(SymbolLiteral),
}

#[derive(Serialize,Deserialize, Clone, Debug, Default)]
pub struct SunSpecModels {
    pub model: Model,
    pub strings: Vec<Strings>,
}