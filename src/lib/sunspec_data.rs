use std::collections::HashMap;
use std::ffi::CString;
use serde::{Serialize, Deserialize};

use std::io;
use std::fs::File;
use serde_xml_rs::from_reader;




#[derive(Deserialize, Default, Debug, Clone)]
#[serde(tag="untagged")]
pub enum PointType {
    string(ResponseType),
    int16(ResponseType),
    uint16(ResponseType),
    acc16(ResponseType),
    enum16(ResponseType),
    bitfield16(ResponseType),
    int32(ResponseType),
    uint32(ResponseType),
    acc32(ResponseType),
    enum32(ResponseType),
    bitfield32(ResponseType),
    sunssf(ResponseType),
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
    pub(crate) symbol: Option<Vec<Symbol>>,
    pub(crate) units: Option<String>,
    #[serde(rename="sf")]
    pub(crate) scale_factor: Option<String>
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
struct SymbolLiteral {
    id: String,
    label: Option<String>,
    description: Option<String>,
    notes: Option<String>
}


#[derive(Deserialize)]
enum LiteralType {
    model(ModelLiteral),
    point(PointLiteral),
    symbol(SymbolLiteral)
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

#[derive(Deserialize, Debug, Clone)]
pub enum ResponseType {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Array(Vec<String>)
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
    pub fn get_model_id_from_name(self, name: String) -> anyhow::Result<Option<u16>> {
        let mut candidates: Vec<u16> = vec![];
        info!("models is of length {}",self.models.len());
        self.models.iter().for_each(|(i,m)| {
            info!("{}", m.name.clone());
            if m.name == name {
                candidates.push(i.clone());
            }
        });
        match candidates.len() {
            0 => {
                return Ok(None);
            },
            1 => {
                return Ok(Some(candidates[0]));
            },
            _ => {
                anyhow::bail!("More than one model with name {name}: {:#?}", candidates);
            }
        }


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
