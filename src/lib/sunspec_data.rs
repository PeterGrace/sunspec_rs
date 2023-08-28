use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use std::io;
use std::fs::File;
use serde_xml_rs::from_reader;




#[derive(Deserialize, Default, Debug, Clone)]
#[serde(tag="untagged")]
pub enum PointType {
    string(String),
    int16(i16),
    uint16(u16),
    acc16(u16),
    enum16(String),
    bitfield16(Vec<String>),
    int32(i32),
    uint32(u32),
    acc32(u32),
    enum32(String),
    bitfield32(Vec<String>),
    #[default]
    pad,
}

#[derive(Default)]
pub struct ResolvedPoint {
    pt: PointType,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>
}
#[derive(Default)]
pub struct ResolvedModel {
    model: Model,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>
}


#[derive(Deserialize, Clone, Debug)]
pub enum Access {
    r,
    rw,
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
    pub(crate) id: String,
    pub(crate) offset: u16,
    pub(crate) r#type: String,
    pub(crate) len: Option<u16>,
    pub(crate) mandatory: Option<bool>,
    pub(crate) access: Option<Access>,
    pub(crate) symbol: Option<Vec<Symbol>>
}
#[derive(Debug, Clone, Deserialize)]
pub struct Symbol {
    pub(crate) id: String,
    #[serde(rename="$value")]
    pub(crate) symbol: String
}
#[derive(Deserialize)]
pub struct Strings {
    id: String,
    locale: Option<String>,
    #[serde(rename = "$value")]
    literals: Vec<LiteralType>
}
#[derive(Deserialize)]
struct ModelLiteral {
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>
}
#[derive(Deserialize)]
struct PointLiteral {
    id: String,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>
}

#[derive(Deserialize)]
enum LiteralType {
    model(ModelLiteral),
    point(PointLiteral),
}

#[derive(Deserialize)]
pub struct SunSpecModels {
    model: Model,
    strings: Vec<Strings>,
}

#[derive(Default, Debug, Clone)]
pub struct SunSpecData {
    models: HashMap<u16, Model>,
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
                },
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
