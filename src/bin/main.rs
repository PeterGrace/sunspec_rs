#[macro_use]
extern crate tracing;
extern crate tokio;

mod cli_args;

use clap::Parser;
use cli_args::CliArgs;
use std::process;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::ValueType;
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
    let mut ss = match SunSpecConnection::new(socket_addr, Some(3), false).await {
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

    if write {
        // write value
        let _model: u16 = 64206_u16;
        let _field: &str = "St";
        let md = ss.models.get(&_model).unwrap().clone();
        match ss
            .clone()
            .set_point(md.clone(), _field, ValueType::Integer(1))
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
        let _model: u16 = 64206_u16;
        let _fields: Vec<&str> = vec!["XFRV", "XFRTms"];
        let md = ss.models.get(&_model).unwrap().clone();
        let block_count = md.clone().get_block_count().unwrap();
        info!("{:#?}", block_count);
        for f in _fields {
            for b in 1..block_count + 1 {
                match ss.clone().get_point(md.clone(), f, Some(b)).await {
                    Ok(pt) => {
                        debug!("{:#?}", pt.value);
                    }
                    Err(e) => {
                        error!("Error received: {e}");
                    }
                }
            }
        }
    }
}
