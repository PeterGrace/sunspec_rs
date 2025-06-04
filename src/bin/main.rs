#[macro_use]
extern crate tracing;
extern crate tokio;

mod cli_args;

use clap::Parser;
use cli_args::CliArgs;
use std::process;
use std::time::Duration;
use sunspec_rs::sunspec_connection::{SunSpecConnection, Word};
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::{
    GroupIdentifier, OptionalGroupIdentifier, PointIdentifier, ValueType,
};
use tokio_modbus::Address;
use tracing_log::AsTrace;
use tracing_subscriber;

#[tokio::main]
pub async fn main() {
    let cli = CliArgs::parse();

    // setup log level
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .with_file(true)
        .with_line_number(true)
        .init();

    let socket_addr = "127.0.0.1:8502".parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(1), false).await {
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

    let write = false;
    let modules = false;

    info!("sleep sleep sleep start");
    tokio::time::sleep(Duration::from_secs(5)).await;
    info!("sleep sleep sleep done");
    if write {
        // write value
        let _model: u16 = 64206_u16;
        let _field: &str = "XFRTms";
        let md = ss.models.get(&_model).unwrap().clone();
        match ss
            .clone()
            .set_point(
                md.clone(),
                PointIdentifier::Point(_field.to_string()),
                ValueType::Integer(300),
            )
            .await
        {
            Ok(_) => {
                info!("It should have worked!");
            }
            Err(e) => {
                error!("Oh no, it didn't work: {e}")
            }
        }
    } else {
        // read fields
        let _model: u16 = 705_u16;
        let md = ss.models.get(&_model).unwrap().clone();
        let block = OptionalGroupIdentifier(Some(GroupIdentifier::String("Crv".to_string())));
        let _fields: Vec<&str> = vec!["VRefAutoTms"];
        for f in _fields {
            let which_block = 1;
            match ss
                .clone()
                .get_point(md.clone(), PointIdentifier::Point(f.to_string()))
                .await
            {
                Ok(pt) => {
                    println!("{_model}/{block}[{which_block}]/{f} = {:#?}", pt.value);
                }
                Err(e) => {
                    error!("Error received: {e}");
                }
            }
        }
    }
}
