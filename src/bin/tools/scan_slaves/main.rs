#[macro_use]
extern crate tracing;
use clap::Parser;
use clap_verbosity_flag;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::ValueType;
use tracing_log::AsTrace;
use tracing_subscriber;

#[derive(clap::Parser)]
pub struct CliArgs {
    pub addr: String,
    pub port: u16,
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

#[tokio::main]
pub async fn main() {
    let cli = CliArgs::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();
    let addr = format!("{}:{}", cli.addr, cli.port);
    let mut devices: Vec<String> = vec![];

    let ssd = SunSpecData::default();
    for i in 1..99 {
        let mut ss = match SunSpecConnection::new(addr.clone(), Some(i)).await {
            Ok(mb) => mb,
            Err(e) => {
                panic!("Can't create modbus connection: {e}");
            }
        };
        ss.models = match ss.clone().populate_models(&ssd).await {
            Ok(m) => m,
            Err(_) => {
                continue;
            }
        };

        let md = ss.models.get(&1).unwrap().clone();
        match ss.clone().get_point(md.clone(), "SN").await {
            Ok(p) => {
                if let Some(st) = p.value {
                    if let ValueType::String(s) = st {
                        if devices.contains(&s) {
                            warn!("Slave id {i} is a clone signal.");
                            continue;
                        }
                        info!("Slave {i} has {} models.", ss.models.len());
                        devices.push(s)
                    }
                }
            }
            Err(e) => {
                warn!("No serial number in common? (slave {i})  Should not be possible: {e}");
            }
        }
    }
}
