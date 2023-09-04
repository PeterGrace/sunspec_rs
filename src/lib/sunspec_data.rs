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

/// A struct that holds a hash of all known model definitions, used as a library for lazy-loading
/// of data into a model as needed.
#[derive(Default, Debug, Clone)]
pub struct SunSpecData {
    pub models: HashMap<u16, SunSpecModels>,
}


impl SunSpecData {
    /// Load model data from disk
    ///
    /// # Arguments
    ///
    /// * `model_id` - The model id to load from disk.
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
    /// retrieve a model definition from the models repository
    ///
    /// # Arguments
    ///
    /// * `model_id` - The model id to load from disk.
    ///
    /// # Returns
    /// Returns a valid SunSpecModels instance, or None if it didn't exist.
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
