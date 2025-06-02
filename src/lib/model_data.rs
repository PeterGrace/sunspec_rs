use crate::sunspec_connection::SunSpecConnection;
use crate::sunspec_data::{ResolvedModel, SunSpecData};
use crate::sunspec_models::{
    GroupIdentifier, LiteralType, OptionalGroupIdentifier, SunSpecModels, ValueType,
};
use std::collections::HashMap;
use std::string::ToString;
use thiserror::Error;
use tokio_modbus::Address;

#[derive(Default, Debug, Clone)]
pub struct ModelData {
    pub id: u16,
    pub len: u16,
    pub address: Address,
    pub model: SunSpecModels,
    pub scale_factors: HashMap<String, i16>,
}

#[derive(Error, Debug, Default)]
pub enum SunSpecModelDataError {
    #[error("While calculating repeating block size, division had a remainder.")]
    Remainder,
    #[error("Default error")]
    #[default]
    Error,
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
        manufacturer: Option<String>,
    ) -> anyhow::Result<Self> {
        let model = data.get_model(id, manufacturer);
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
    /// Returns the number of blocks for this model
    pub fn get_block_count(self) -> Result<u16, SunSpecModelDataError> {
        let model = self.model.model.clone();
        let first_block_type = model.block[0]
            .r#type
            .clone()
            .unwrap_or(String::from("none"));
        let model_len = self.len; // the length of the actual model

        if first_block_type == "repeating" {
            // if the first block is a 'repeating' block, per the standard, there are no fixed blocks.
            let fixed_block_len = 0_u16;
            let repeat_block_len = model.block[0].len; // the length of the repeating block
            if (model_len - fixed_block_len) % repeat_block_len != 0 {
                return Err(SunSpecModelDataError::Remainder);
            }
            let num_blocks = (model_len - fixed_block_len) / repeat_block_len;
            Ok(num_blocks)
        } else if self.model.model.block.len() > 1 {
            // the length of the array of read-in blocks
            let fixed_block_len = model.block[0].len; // the length of the fixed block
            let repeat_block_len = model.block[1].len; // the length of the repeating block
            if (model_len - fixed_block_len) % repeat_block_len != 0 {
                return Err(SunSpecModelDataError::Remainder);
            }
            let num_blocks = (model_len - fixed_block_len) / repeat_block_len;
            Ok(num_blocks)
        } else {
            // this should always be 1
            assert_eq!(model.block.len(), 1);
            Ok(model.block.len() as u16)
        }
    }

    /// For a given model point, retrieve its scale factor and store it for later re-use.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the point inside our model to query
    /// * `conn` - The SunSpecConnection we have open already (so that we can query the proper connection)
    pub async fn get_scale_factor(
        &mut self,
        name: &str,
        conn: SunSpecConnection,
        block: Option<GroupIdentifier>,
        addr: Option<u16>,
    ) -> Option<i16> {
        if let Some(value) = self.scale_factors.get(name) {
            return Some(*value);
        } else {
            if let Ok(point) = conn
                .clone()
                .get_point(self.clone(), name, OptionalGroupIdentifier(block), addr)
                .await
            {
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
