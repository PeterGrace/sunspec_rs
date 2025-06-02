use crate::json::group::Group;
use crate::sunspec_models::Block;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct JSONModel {
    pub id: u16,
    pub group: Group,
}
