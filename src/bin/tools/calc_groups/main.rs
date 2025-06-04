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
use sunspec_rs::sunspec_connection::{process_json_group, PointNode};
use sunspec_rs::sunspec_connection::{SunSpecConnection, SunSpecReadError, Word};
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::{ModelSource, ValueType};
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
    let (ss, _) = setup(addr, 1).await;
    let model: u16 = 705;
    if let Some(m) = ss.models.clone().get(&model) {
        info!("Model: {}, length: {}", model, m.len);
    }
}
