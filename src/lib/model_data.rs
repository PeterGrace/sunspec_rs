use crate::sunspec_connection::SunSpecConnection;
use crate::sunspec_data::{ResolvedModel, SunSpecData};
use crate::sunspec_models::{LiteralType, SunSpecModels, ValueType};
use std::collections::HashMap;
use std::string::ToString;
use tokio_modbus::Address;

#[derive(Default, Debug, Clone)]
pub struct ModelData {
    pub id: u16,
    pub len: u16,
    pub address: Address,
    pub(crate) model: SunSpecModels,
    pub scale_factors: HashMap<String, i16>,
}

impl ModelData {
    /// create new modeldata object
    ///
    /// # Arguments
    ///
    /// * `data` - an initialized SunSpecData object
    /// * `id` - The model id number for the model in question
    /// * `len` - The length of this model (returned when querying the model)
    /// * `address` - Where this particular model exists in the address range
    pub async fn new(
        data: SunSpecData,
        id: u16,
        len: u16,
        address: Address,
    ) -> anyhow::Result<Self> {
        let model = data.get_model(id);
        if model.is_none() {
            anyhow::bail!("Couldn't get model for id {id}");
        }
        Ok(ModelData {
            id,
            len,
            address,
            model: model.unwrap(),
            scale_factors: HashMap::default(),
        })
    }

    /// For a given model point, retrieve its scale factor and store it for later re-use.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the point inside our model to query
    /// * `conn` - The SunSpecConnection we have open already (so that we can query the proper connection)
    pub async fn get_scale_factor(&mut self, name: &str, conn: SunSpecConnection) -> Option<i16> {
        if let Some(value) = self.scale_factors.get(name) {
            return Some(*value);
        } else {
            if let Ok(point) = conn.clone().get_point(self.clone(), name).await {
                if let Some(ValueType::Integer(val)) = point.value {
                    self.scale_factors.insert(name.to_string(), val as i16);
                    return Some(val as i16);
                };
            } else {
                return None;
            };
        }
        return None;
    }

    /// Return a model object that contains descriptive data from the modelfile to explain what this model is
    pub async fn get_resolved_model(self) -> ResolvedModel {
        let mut resolved_model: ResolvedModel = ResolvedModel::default();

        let strings = &self.model.clone().strings[0];
        for l in strings.literals.iter() {
            if let LiteralType::Model(model_literal) = l {
                resolved_model.notes = model_literal.notes.clone();
                resolved_model.description = model_literal.description.clone();
                resolved_model.label = model_literal.label.clone();
            }
        }

        resolved_model.model = self.model.model;
        debug!("resolved model is {:#?}", resolved_model);
        resolved_model
    }
}
