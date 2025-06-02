use crate::json::group::Group;
use crate::json::misc::JSONModel;
use std::collections::HashMap;

use crate::sunspec_models::{Model, SunSpecModels, Symbol};
use std::fs::File;
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
    /// * `id` - The model id to load from disk.
    /// * `manufacturer` - The name of the manufacturer, for models >=64200
    fn load_model_xml(id: u16, manufacturer: &Option<String>) -> anyhow::Result<SunSpecModels> {
        let filename = match id {
            f if f >= 64200 => {
                if let Some(mn) = manufacturer {
                    format!("models/{}/smdx_{:05}.xml", mn.to_ascii_lowercase(), f)
                } else {
                    format!("models/smdx_{:05}.xml", f)
                }
            }
            _ => {
                format!("models/smdx_{:05}.xml", id)
            }
        };
        let fd = match File::open(filename.clone()) {
            Ok(f) => f,
            Err(e) => {
                anyhow::bail!("Error reading XML file {filename}: {e}");
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
    fn load_model_json(id: u16, manufacturer: &Option<String>) -> anyhow::Result<SunSpecModels> {
        let filename = format!("models/model_{}.json", id);
        let fd = match File::open(filename.clone()) {
            Ok(f) => f,
            Err(e) => {
                anyhow::bail!("Error reading JSON file {filename}: {e}");
            }
        };
        let jsonmodel: JSONModel = match serde_json::from_reader(fd) {
            Ok(m) => m,
            Err(e) => {
                anyhow::bail!("Couldn't deserialize JSON: {e}");
            }
        };

        let ssm = SunSpecModels::from(&jsonmodel);
        Ok(ssm)
    }
    fn load_model(id: u16, manufacturer: Option<String>) -> anyhow::Result<SunSpecModels> {
        return if let Ok(ssm) = SunSpecData::load_model_json(id, &manufacturer).map_err(|e| {
            trace!("{e}");
            e
        }) {
            trace!("load_model_json {id}");
            Ok(ssm)
        } else {
            trace!("load_model_xml {id}");
            SunSpecData::load_model_xml(id, &manufacturer)
        };
    }
    /// retrieve a model definition from the models repository
    ///
    /// # Arguments
    ///
    /// * `model_id` - The model id to load from disk.
    ///
    /// # Returns
    /// Returns a valid SunSpecModels instance, or None if it didn't exist.
    pub fn get_model(mut self, id: u16, mn: Option<String>) -> Option<SunSpecModels> {
        let lookup = self.models.get(&id);

        if lookup.is_none() {
            match SunSpecData::load_model(id, mn) {
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
    pub fn get_symbols_for_point(
        self,
        id: u16,
        point_name: String,
        mn: Option<String>,
    ) -> Option<Vec<Symbol>> {
        if let Some(model) = self.get_model(id, mn) {
            // block[0] is always the fixed block, and block[1] will always be
            // the first repeating block, if exists.
            let points = &model.model.block[0].point;
            for point in points.iter() {
                if point.id == point_name {
                    return point.symbol.clone();
                }
            }
            if &model.model.block.len() > &1_usize {
                let points = &model.model.block[1].point;
                for point in points.iter() {
                    if point.id == point_name {
                        return point.symbol.clone();
                    }
                }
            }
        }
        None
    }
}
