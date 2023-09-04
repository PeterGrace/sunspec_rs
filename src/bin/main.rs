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

#[tokio::main]
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

    let modelid = 102;
    let fields:Vec<&str> = vec!["PhVphA","PhVphB"];


    let md = ss.models.get(&modelid).unwrap().clone();
    let resolved_model = md.clone().get_resolved_model().await;
    let model_name = resolved_model.model.clone().name;
    debug!("Attempting to call get point on model {}, fields {:#?}",model_name, fields);
    for f in fields {
        if let Some(pt) = ss.clone().get_point(md.clone(), f).await {
            let mut message: String = String::default();

            message = message + &*format!("{:#?}", pt);
            info!(message);
        }
    };
}