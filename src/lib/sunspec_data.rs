use std::collections::HashMap;
use serde::Deserialize;
use std::fs::File;


#[derive(Deserialize, Default, Debug, Clone)]
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
    Pad
}

#[derive(Default)]
pub struct ResolvedPoint {
    pt: PointType,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>,
}

#[derive(Default)]
pub struct ResolvedModel {
    model: Model,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>,
}


#[derive(Deserialize, Clone, Debug)]
pub enum Access {
    #[serde(rename = "r")]
    ReadOnly,
    #[serde(rename = "rw")]
    ReadWrite,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Model {
    pub id: u16,
    pub len: u16,
    pub name: String,
    pub block: Vec<Block>,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct Block {
    len: u16,
    r#type: Option<String>,
    pub(crate) point: Vec<Point>,
}

#[derive(Deserialize, Debug, Default, Clone)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct Symbol {
    pub(crate) id: String,
    #[serde(rename = "$value")]
    pub(crate) symbol: String,
}

#[derive(Deserialize, Debug)]
pub struct Strings {
    id: String,
    locale: Option<String>,
    #[serde(rename = "$value")]
    literals: Vec<LiteralType>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModelLiteral {
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PointLiteral {
    id: String,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SymbolLiteral {
    id: String,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>,
}


#[derive(Deserialize, Debug, Clone)]
pub enum LiteralType {
    #[serde(rename = "model")]
    Model(ModelLiteral),
    #[serde(rename = "point")]
    Point(PointLiteral),
    #[serde(rename = "symbol")]
    Symbol(SymbolLiteral),
}

#[derive(Deserialize)]
pub struct SunSpecModels {
    model: Model,
    strings: Vec<Strings>,
}

#[derive(Default, Debug, Clone)]
pub struct SunSpecData {
    pub models: HashMap<u16, Model>,
}

#[derive(Deserialize, Debug, Clone)]
pub enum ResponseType {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Array(Vec<String>),
}

impl SunSpecData {
    fn load_model(id: u16) -> anyhow::Result<Model> {
        let filename = format!("models/smdx_{:05}.xml", id);
        let fd = match File::open(filename) {
            Ok(f) => f,
            Err(e) => {
                anyhow::bail!("Error reading XML file: {e}");
            }
        };
        let ssm: SunSpecModels = match serde_xml_rs::from_reader(fd) {
            Ok(m) => m,
            Err(e) => {
                anyhow::bail!("Couldn't deserialize xml: {e}");
            }
        };

        Ok(ssm.model)
    }
    pub fn get_model(mut self, id: u16) -> Option<Model> {
        let lookup = self.models.get(&id);

        if lookup.is_none() {
            match SunSpecData::load_model(id) {
                Ok(m) => {
                    self.models.insert(id, m.clone());
                    Some(m)
                }
                Err(e) => {
                    warn!("Can't load model for {id}: {e}");
                    None
                }
            }
        } else {
            Some(lookup.unwrap().clone())
        }
    }
}
