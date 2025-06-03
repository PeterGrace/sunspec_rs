#[macro_use]
extern crate tracing;

use ::sunspec_rs::sunspec_connection::NOT_IMPLEMENTED_U32;
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use lazy_static::lazy_static;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use std::convert::TryFrom;
use sunspec_rs::json::group::{Group, GroupCount};
use sunspec_rs::json::misc::JSONModel;
use sunspec_rs::json::point::{Point, PointType, PointValue};
use sunspec_rs::sunspec_connection::PointNode;
use sunspec_rs::sunspec_connection::{SunSpecConnection, SunSpecReadError, Word};
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::{ModelSource, ValueType};
use tokio::sync::RwLock;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

pub async fn setup(addr: &str, slave_id: u8) -> (SunSpecConnection, SunSpecData) {
    let socket_addr = addr.parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(slave_id), false).await {
        Ok(mb) => mb,
        Err(e) => {
            panic!("Can't create modbus connection: {e}");
        }
    };

    let ssd = SunSpecData::default();
    match ss.clone().populate_models(&ssd).await {
        Ok(m) => ss.models = m,
        Err(e) => {
            panic!("Can't populate models: {e}")
        }
    };

    return (ss, ssd);
}
#[tokio::main]
pub async fn main() {
    //region enable console log output and tokio-console monitoring
    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("INFO"));
    let format_layer = tracing_subscriber::fmt::layer()
        .event_format(
            tracing_subscriber::fmt::format()
                .with_file(true)
                .with_thread_ids(true)
                .with_line_number(true),
        )
        .with_span_events(FmtSpan::NONE);

    let subscriber = Registry::default() // W: unused variable: `subscriber`
        .with(env_filter)
        .with(format_layer);

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    //endregion
    let addr = "127.0.0.1:8502";
    let (mut ss, _) = setup(addr, 1).await;
    let model: u16 = 705;
    if let Some(m) = ss.models.clone().get(&model) {
        info!("Model: {}, length: {}", model, m.len);
        if let ModelSource::Json(json) = &m.model.source {
            // m.len + 2 because len doesn't include ID and L values in its calculation
            if let Ok(mut data) = ss.get_raw(m.address, m.len + 2).await {
                // start with our root level points
                process_json_group(
                    &mut data,
                    &json.group,
                    None,
                    &mut m.address.clone(),
                    &mut ss.catalog,
                )
                .await;
                // ok, now for groups
            }
        }
    }
}
pub fn parse_point_data(p: &Point, d: &Vec<Word>) -> anyhow::Result<ValueType> {
    match p.type_ {
        PointType::Uint16 | PointType::Int16 | PointType::Sunssf | PointType::Enum16 => {
            if let Some(pointval) = d[0].to_i64() {
                Ok(ValueType::Integer((pointval as i16) as i32))
            } else {
                Err(anyhow!("Can't convert to integer"))
            }
        }
        PointType::Uint32 => {
            let val = (d[0] as u32) << 16 | d[1] as u32;
            if val == NOT_IMPLEMENTED_U32 {
                return Err(anyhow!("Not implemented"));
            } else {
                Ok(ValueType::Integer(val as i32))
            }
        }
        _ => Err(anyhow!("Not implemented")),
    }
}
#[async_recursion]
pub async fn process_json_group(
    data: &mut Vec<Word>,
    group: &Group,
    prefix: Option<String>,
    address: &mut u16,
    mut catalog: &mut HashMap<String, PointNode>,
) {
    let newprefix = match prefix.clone() {
        Some(s) => {
            //info!("Group: {:#?}", group);
            format!("{}.{}", s, group.name)
        }
        None => format!(".{}", group.name),
    };
    let mut entries: i64 = 0;
    match &group.count {
        GroupCount::String(countval) => {
            let count_lookup = match prefix {
                Some(s) => format!(".{}.{}", s.split('.').nth(1).unwrap(), countval),
                None => format!(".{}", countval),
            };
            if let Some(num_of_groups_val) = catalog.get(&count_lookup) {
                if let ValueType::Integer(num_groups) = num_of_groups_val.value {
                    info!("Group: {}, count: {}", newprefix, num_groups);
                    entries = num_groups as i64;
                }
            }
        }
        GroupCount::Integer(i) => {
            info!("Group: {}, count: {}", newprefix, i);
            entries = i.to_i64().unwrap();
        }
    }
    for i in 0..entries {
        for p in group.points.iter() {
            let datum: Vec<Word> = data.drain(..p.size as usize).collect();
            match parse_point_data(&p, &datum) {
                Ok(v) => {
                    let pointname = if entries > 1 {
                        format!("{}[{}].{}", newprefix, i, p.name)
                    } else {
                        format!("{}.{}", newprefix, p.name)
                    };
                    info!("{} @0x{} {:#?}", pointname, address, v);
                    // this is too simple, the actual solution will need to account for
                    // which group and group number the point belongs to
                    catalog.insert(
                        pointname,
                        PointNode {
                            value: v,
                            address: address.clone(),
                        },
                    );
                    *address += p.size as u16;
                }
                Err(e) => {
                    error!(
                        "Can't parse point {}.{} {:?}: {e}",
                        newprefix, p.name, p.type_
                    );
                }
            }
        }
        for g in group.groups.iter() {
            process_json_group(data, g, Some(newprefix.clone()), address, &mut catalog).await;
        }
    }
}
