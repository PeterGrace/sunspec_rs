#[macro_use]
extern crate tracing;
extern crate tokio;

mod cli_args;

use std::process;
use tokio_modbus::prelude::*;
use cli_args::CliArgs;
use clap::Parser;
use tokio_modbus::Address;
use tracing_log::AsTrace;
use tracing_subscriber;
use sunspec_rs::sunspec::{ModelData, SunSpecConnection};
use sunspec_rs::sunspec_data::SunSpecData;

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let cli = CliArgs::parse();

    // setup log level
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();

    let socket_addr = "127.0.0.1:5083".parse().unwrap();
    let slave = Slave(3);
    let mut ss = match SunSpecConnection::new(socket_addr, Some(slave)).await {
        Ok(mb) => mb,
        Err(e) => {
            error!("Can't create modbus connection: {e}");
            process::exit(1);
        }
    };

    ss.models = ss.clone().populate_models().await;

    let mut ssd = SunSpecData::default();
    ss.models.iter().for_each(|(id, md)| {
        let id = id.clone();
        let ssd = ssd.clone();
        let m = ssd.get_model(id);
    });

    // let modelid = match ssd.clone().get_model_id_from_name("freq_watt".to_string()) {
    //     Ok(result) => {
    //         match result {
    //             Some(val) => val,
    //             None => {
    //                 error!("Couldn't find model.");
    //                 process::exit(1)
    //             }
    //         }
    //     }
    //     Err(e) => {
    //         error!("Multiple models detected: {e}");
    //         process::exit(1);
    //     }
    // };

    let modelid = 102;
    let fields:Vec<&str> = vec!["RelayStatus", "PhVphA","PhVphB"];

    let mut md = ss.models.get(&modelid).unwrap().clone();
    if md.model.is_none() {
        md.model = ssd.get_model(modelid);
    }
    let model_name = md.clone().model.unwrap().name;
    info!("Attempting to call get point on model {}, fields {:#?}",model_name, fields);
    for f in fields {
        if let Some(pt) = ss.clone().get_point(md.clone(), f).await {
            info!("We received a PointType of {:#?}", pt);
        }
    };
}