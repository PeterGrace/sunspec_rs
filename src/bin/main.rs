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
    let slave = Slave(6);
    let mut ss = match SunSpecConnection::new(socket_addr, Some(slave)).await {
        Ok(mb) => mb,
        Err(e) => {
            error!("Can't create modbus connection: {e}");
            process::exit(1);
        }
    };
    //ss.models = SunSpecConnection::populate_models(ss.ctx).await;
    ss.models.insert(1, ModelData { id: 1, len: 66, address: 40002, model: None });
    let mut ssd = SunSpecData::default();
    ss.models.iter().for_each(|(id, md)| {
        let id = id.clone();
        let ssd = ssd.clone();
        let m = ssd.get_model(id);
    });
    let mut md = ss.models.get(&1).unwrap().clone();
    if md.model.is_none() {
        md.model = ssd.get_model(1);
    }

    let field = "Md";
    info!("Attempting to call get point on model 1, field {field}");
    let point = ss.get_point(md, field).await;




    //  let suns = ss.ctx.get_string(40000, 2).await.unwrap();
    //
    //  let id = ss.ctx.get_u16(40002).await.unwrap();
    //  let length = ss.ctx.get_u16(40003).await.unwrap();
    //  let Mn = match ss.ctx.get_string(40004, 16).await {
    //      Ok(b) => b,
    //      Err(e) => {
    //          error!("{e}");
    //          process::exit(1);
    //      }
    //  };
    // let Md = ss.ctx.get_string(40020, 16).await.unwrap();
    // let Opt = ss.ctx.get_string(40036, 8).await.unwrap();
    // let Vr = ss.ctx.get_string(40044, 8).await.unwrap();
    // let SN = ss.ctx.get_string(40052, 16).await.unwrap();
    // let DA = ss.ctx.get_u16(40068).await.unwrap();
    //  println!("{suns} {id} {length} {Mn} {Md} {Opt} {Vr} {SN} {DA}");
}
