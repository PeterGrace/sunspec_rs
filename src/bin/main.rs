#[macro_use]
extern crate tracing;
extern crate tokio;

mod cli_args;

use clap::Parser;
use cli_args::CliArgs;
use std::process;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;
use tracing_log::AsTrace;
use tracing_subscriber;

#[tokio::main]
pub async fn main() {
    let cli = CliArgs::parse();

    // setup log level
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();

    let socket_addr = "127.0.0.1:5083".parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(6)).await {
        Ok(mb) => mb,
        Err(e) => {
            error!("Can't create modbus connection: {e}");
            process::exit(1);
        }
    };

    let ssd = SunSpecData::default();
    match ss.clone().populate_models(&ssd).await {
        Ok(m) => ss.models = m,
        Err(e) => {
            panic!("Can't populate models: {e}")
        }
    };

    let _field: &str = "AlmRst";
    let fields: Vec<&str> = vec!["State", "Evt1"];

    let md_802 = ss.models.get(&802).unwrap().clone();
    // match ss
    //     .clone()
    //     .set_point(md_802.clone(), field, ValueType::Integer(1))
    //     .await
    // {
    //     Ok(_) => {
    //         info!("It should have worked!");
    //     }
    //     Err(e) => {
    //         error!("Oh no, it didn't work: {e}")
    //     }
    // }

    // write alarm reset value
    // let md_802 = ss.models.get(&802).unwrap().clone();
    // match ss
    //     .clone()
    //     .set_point(md_802.clone(), field, ValueType::Integer(1))
    //     .await
    // {
    //     Ok(_) => {
    //         info!("It should have worked!");
    //     }
    //     Err(e) => {
    //         error!("Oh no, it didn't work: {e}")
    //     }
    // }

    for f in fields {
        if let Ok(pt) = ss.clone().get_point(md_802.clone(), f).await {
            debug!("{:#?}", pt.value);
        }
    }
}
