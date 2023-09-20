use clap::Parser;
use clap_verbosity_flag;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;
use tracing_log::AsTrace;
use tracing_subscriber;

#[derive(clap::Parser)]
pub struct CliArgs {
    pub addr: String,
    pub port: u16,
    pub slave: u8,
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
}

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
    let cli = CliArgs::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.verbose.log_level_filter().as_trace())
        .init();
    let addr = format!("{}:{}", cli.addr, cli.port);
    let (ss, _) = setup(&addr, cli.slave).await;
    for (id, _) in ss.models.iter() {
        println!("{}", id);
    }
}
