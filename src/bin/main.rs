#[macro_use]
extern crate tracing;
extern crate tokio;

mod cli_args;

use std::process;
use cli_args::CliArgs;
use clap::Parser;
use tracing_log::AsTrace;
use tracing_subscriber;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    let cli = CliArgs::parse();

    // setup log level
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();

    let socket_addr = "127.0.0.1:5083".parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(3)).await {
        Ok(mb) => mb,
        Err(e) => {
            error!("Can't create modbus connection: {e}");
            process::exit(1);
        }
    };

    let ssd = SunSpecData::default();
    ss.models = ss.clone().populate_models(ssd.clone()).await;


    // ss.models.iter().for_each(|(id, md)| {
    //     let id = id.clone();
    //     let ssd = ssd.clone();
    //     let m = ssd.get_model(id);
    // });

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
    let fields:Vec<&str> = vec!["PhVphA","PhVphB"];

    let md = ss.models.get(&modelid).unwrap().clone();
    let model_name = md.model.clone().name;
    debug!("Attempting to call get point on model {}, fields {:#?}",model_name, fields);
    for f in fields {
        if let Some(pt) = ss.clone().get_point(md.clone(), f).await {
            let mut message: String = String::default();

            message = message + &*format!("{:#?}", pt.value.unwrap());
            info!(message);
        }
    };
}