use crate::json::defaults::point_access;
use crate::json::group::{Group, GroupType};
use crate::json::misc::JSONModel;
use crate::json::point::{
    Point as JSONPoint, PointAccess, PointMandatory, PointSf, PointStatic, PointType as jpt,
    PointValue,
};
use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::Deref;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ValueType {
    String(String),
    Integer(i32),
    Float(f32),
    Boolean(bool),
    Array(Vec<String>),
    Pad,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Access {
    #[serde(rename = "r")]
    ReadOnly,
    #[serde(rename = "rw")]
    ReadWrite,
}
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(tag = "untagged")]
pub enum PointType {
    #[serde(rename = "string")]
    String(ValueType),
    #[serde(rename = "int16")]
    Int16(ValueType),
    #[serde(rename = "uint16")]
    UnsignedInt16(ValueType),
    #[serde(rename = "acc16")]
    Accumulator16(ValueType),
    #[serde(rename = "enum16")]
    Enum16(ValueType),
    #[serde(rename = "bitfield16")]
    Bitfield16(ValueType),
    #[serde(rename = "int32")]
    Int32(ValueType),
    #[serde(rename = "uint32")]
    UnsignedInt32(ValueType),
    #[serde(rename = "acc32")]
    Accumulator32(ValueType),
    #[serde(rename = "enum32")]
    Enum32(ValueType),
    #[serde(rename = "bitfield32")]
    Bitfield32(ValueType),
    #[serde(rename = "sunssf")]
    SunSpecScaleFactor(ValueType),
    #[serde(rename = "pad")]
    #[default]
    Pad,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Model {
    pub id: u16,
    pub len: u16,
    pub name: String,
    pub block: Vec<Block>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Block {
    pub(crate) len: u16,
    pub(crate) r#type: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) point: Vec<Point>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GroupIdentifier {
    Integer(u16),
    String(String),
}
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct OptionalGroupIdentifier(pub Option<GroupIdentifier>);

impl Deref for OptionalGroupIdentifier {
    type Target = Option<GroupIdentifier>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<OptionalGroupIdentifier> for Option<GroupIdentifier> {
    fn from(value: OptionalGroupIdentifier) -> Self {
        value.0
    }
}

impl From<Option<GroupIdentifier>> for OptionalGroupIdentifier {
    fn from(value: Option<GroupIdentifier>) -> Self {
        match value.clone() {
            None => OptionalGroupIdentifier(None),
            Some(GroupIdentifier::Integer(i)) => {
                OptionalGroupIdentifier(Some(GroupIdentifier::Integer(i)))
            }
            Some(GroupIdentifier::String(s)) => {
                OptionalGroupIdentifier(Some(GroupIdentifier::String(s)))
            }
        }
    }
}
impl Display for GroupIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupIdentifier::Integer(i) => write!(f, "{}", i),
            GroupIdentifier::String(s) => write!(f, "{}", s),
        }
    }
}
impl Display for OptionalGroupIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(val) = self.0.clone() {
            match val {
                GroupIdentifier::Integer(i) => write!(f, "{}", i),
                GroupIdentifier::String(s) => write!(f, "{}", s),
            }
        } else {
            write!(f, "None")
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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
    pub value: Option<ValueType>,
    pub literal: Option<PointLiteral>,
    pub block_id: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Symbol {
    pub id: String,
    #[serde(rename = "$value")]
    pub symbol: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Strings {
    pub(crate) id: String,
    pub(crate) locale: Option<String>,
    #[serde(rename = "$value")]
    pub(crate) literals: Vec<LiteralType>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelLiteral {
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PointLiteral {
    pub id: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SymbolLiteral {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) notes: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LiteralType {
    #[serde(rename = "model")]
    Model(ModelLiteral),
    #[serde(rename = "point")]
    Point(PointLiteral),
    #[serde(rename = "symbol")]
    Symbol(SymbolLiteral),
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub enum ModelSource {
    #[default]
    XML,
    Json(JSONModel),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SunSpecModels {
    pub model: Model,
    pub strings: Vec<Strings>,
    pub source: ModelSource,
}

impl From<&JSONModel> for SunSpecModels {
    fn from(json: &JSONModel) -> Self {
        let mut model_len: u16 = 0;
        for p in json.group.points.iter() {
            model_len = model_len.saturating_add(p.size as u16);
        }

        info!("JSON Groups count is {:#?}", json.group.count);
        let mut blocks: Vec<Block> = vec![];

        let mut block_len: u16 = 0;
        block_len = model_len;
        let mut points: Vec<Point> = vec![];

        let mut offset: u16 = 0;
        // in the json model, ID and L are always zero and 1 in the array.  In the xml/smdx,
        // ID and L aren't specified because they're implied.  Because of this, we skip processing
        // the first two points so that the offset address lines up.
        for point in json.group.points.iter().skip(2) {
            trace!("{}: offset value for {} is {offset}", point.name, json.id);

            let sf_string: Option<String>;
            if point.sf.is_some() {
                match point.clone().sf.unwrap() {
                    PointSf::String(s) => sf_string = Some(s),
                    PointSf::Integer(i) => sf_string = Some(format!("{}", i)),
                }
            } else {
                sf_string = None;
            }

            let foo = Point {
                id: point.name.clone(),
                offset,
                r#type: match point.type_ {
                    jpt::Int16 => "int16".to_string(),
                    jpt::Int32 => "int32".to_string(),
                    jpt::Int64 => "int64".to_string(),
                    jpt::Raw16 => "raw16".to_string(),
                    jpt::Uint16 => "uint16".to_string(),
                    jpt::Uint32 => "uint32".to_string(),
                    jpt::Uint64 => "uint64".to_string(),
                    jpt::Acc16 => "acc16".to_string(),
                    jpt::Acc32 => "acc32".to_string(),
                    jpt::Acc64 => "acc64".to_string(),
                    jpt::Bitfield16 => "bitfield16".to_string(),
                    jpt::Bitfield32 => "bitfield32".to_string(),
                    jpt::Bitfield64 => "bitfield64".to_string(),
                    jpt::Enum16 => "enum16".to_string(),
                    jpt::Enum32 => "enum32".to_string(),
                    jpt::Float32 => "float32".to_string(),
                    jpt::Float64 => "float64".to_string(),
                    jpt::String => "string".to_string(),
                    jpt::Sf => "sf".to_string(),
                    jpt::Pad => "pad".to_string(),
                    jpt::Ipaddr => "ipaddr".to_string(),
                    jpt::Ipv6addr => "ipv6addr".to_string(),
                    jpt::Eui48 => "eui48".to_string(),
                    jpt::Sunssf => "sunssf".to_string(),
                    jpt::Count => "count".to_string(),
                },
                len: Some(point.size as u16),
                mandatory: if point.mandatory == PointMandatory::M {
                    Some(true)
                } else {
                    Some(false)
                },
                access: if point.access == PointAccess::Rw {
                    Some(Access::ReadWrite)
                } else {
                    Some(Access::ReadOnly)
                },
                symbol: Some(
                    point
                        .symbols
                        .iter()
                        .map(|s| Symbol {
                            symbol: s.clone().value.to_string(),
                            id: s.clone().name,
                        })
                        .collect(),
                ),
                units: point.units.clone(),
                scale_factor: sf_string,
                value: if point.static_ == PointStatic::S {
                    match point.value.clone() {
                        Some(PointValue::String(s)) => Some(ValueType::String(s)),
                        Some(PointValue::Integer(i)) => Some(ValueType::Integer(i as i32)),
                        None => {
                            info!(
                                "{}/{}: Point is specified as static, but no value provided.",
                                json.id, point.name,
                            );
                            None
                        }
                    }
                } else {
                    None
                },
                literal: None,
                block_id: None,
            };
            points.push(foo);
            offset = offset.saturating_add(point.size as u16);
        }

        blocks.push(Block {
            len: block_len,
            r#type: None,
            name: Some(json.group.name.clone()),
            point: points,
        });

        if !json.group.groups.is_empty() {
            // we need to generate a new block per group
            for g in json.group.groups.iter() {
                // reinitialize an empty points vec per grouping
                let mut offset: u16 = 0;
                let mut points: Vec<Point> = vec![];
                let mut block = Block::default();
                block.name = Some(g.clone().name);
                for point in g.points.iter() {
                    let sf_string: Option<String>;
                    if point.sf.is_some() {
                        match point.clone().sf.unwrap() {
                            PointSf::String(s) => sf_string = Some(s),
                            PointSf::Integer(i) => sf_string = Some(format!("{i}")),
                        }
                    } else {
                        sf_string = None;
                    }

                    let foo = Point {
                        id: point.name.clone(),
                        offset,
                        r#type: match point.type_ {
                            jpt::Int16 => "int16".to_string(),
                            jpt::Int32 => "int32".to_string(),
                            jpt::Int64 => "int64".to_string(),
                            jpt::Raw16 => "raw16".to_string(),
                            jpt::Uint16 => "uint16".to_string(),
                            jpt::Uint32 => "uint32".to_string(),
                            jpt::Uint64 => "uint64".to_string(),
                            jpt::Acc16 => "acc16".to_string(),
                            jpt::Acc32 => "acc32".to_string(),
                            jpt::Acc64 => "acc64".to_string(),
                            jpt::Bitfield16 => "bitfield16".to_string(),
                            jpt::Bitfield32 => "bitfield32".to_string(),
                            jpt::Bitfield64 => "bitfield64".to_string(),
                            jpt::Enum16 => "enum16".to_string(),
                            jpt::Enum32 => "enum32".to_string(),
                            jpt::Float32 => "float32".to_string(),
                            jpt::Float64 => "float64".to_string(),
                            jpt::String => "string".to_string(),
                            jpt::Sf => "sf".to_string(),
                            jpt::Pad => "pad".to_string(),
                            jpt::Ipaddr => "ipaddr".to_string(),
                            jpt::Ipv6addr => "ipv6addr".to_string(),
                            jpt::Eui48 => "eui48".to_string(),
                            jpt::Sunssf => "sunssf".to_string(),
                            jpt::Count => "count".to_string(),
                        },
                        len: Some(point.size as u16),
                        mandatory: if point.mandatory == PointMandatory::M {
                            Some(true)
                        } else {
                            Some(false)
                        },
                        access: if point.access == PointAccess::Rw {
                            Some(Access::ReadWrite)
                        } else {
                            Some(Access::ReadOnly)
                        },
                        symbol: Some(
                            point
                                .symbols
                                .iter()
                                .map(|s| Symbol {
                                    symbol: s.clone().value.to_string(),
                                    id: s.clone().name,
                                })
                                .collect(),
                        ),
                        units: point.units.clone(),
                        scale_factor: sf_string,
                        value: if point.static_ == PointStatic::S {
                            match point.value.clone() {
                                Some(PointValue::String(s)) => Some(ValueType::String(s)),
                                Some(PointValue::Integer(i)) => Some(ValueType::Integer(i as i32)),
                                None => {
                                    debug!(
                                "Point is specified as static, but no value provided: {:#?}",
                                serde_json::to_string(point)
                            );
                                    None
                                }
                            }
                        } else {
                            None
                        },
                        literal: None,
                        block_id: None,
                    };
                    debug!(
                        "{}/{}/{}: offset value is {offset}, size = {}",
                        json.id, g.name, point.name, point.size
                    );
                    points.push(foo);
                    offset = offset.saturating_add(point.size as u16);
                }
                block.point = points;
                block.len = offset;
                blocks.push(block);
            }
        }

        assert!(model_len > 0);
        let model = Model {
            id: json.id,
            len: model_len,
            name: json.group.clone().name,
            block: blocks,
        };

        SunSpecModels {
            model,
            strings: vec![],
            source: ModelSource::Json(json.clone()),
        }
    }
}
