use std::collections::HashMap;

use std::fs::File;
use crate::sunspec_models::{Model, PointType, SunSpecModels};


#[derive(Default)]
pub struct ResolvedPoint {
    pub pt: PointType,
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Default, Debug)]
pub struct ResolvedModel {
    pub model: Model,
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}




#[derive(Default, Debug, Clone)]
pub struct SunSpecData {
    pub models: HashMap<u16, SunSpecModels>,
}


impl SunSpecData {
    fn load_model(id: u16) -> anyhow::Result<SunSpecModels> {
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

        Ok(ssm)
    }
    pub fn get_model(mut self, id: u16) -> Option<SunSpecModels> {
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
