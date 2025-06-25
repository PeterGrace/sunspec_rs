#[macro_use]
extern crate tracing;
extern crate tokio;

use std::process;
use std::time::Duration;
use sunspec_rs::sunspec_connection::{SunSpecConnection, TlsConfig, Word};
use sunspec_rs::sunspec_data::SunSpecData;
use sunspec_rs::sunspec_models::{
    GroupIdentifier, OptionalGroupIdentifier, PointIdentifier, ValueType,
};
use tokio_modbus::Address;
use tracing_log::AsTrace;
use tracing_subscriber;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

#[tokio::main]
pub async fn main() {
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

    let tls = TlsConfig::builder()
        .domain("dersim".to_string())
        .ca_file("/home/pgrace/ca.crt".to_string())
        .client_cert_file("/home/pgrace/client.crt".to_string())
        .client_key_file("/home/pgrace/client.key".to_string())
        .build();

    let socket_addr = "127.0.0.1:8502".parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(1), false, Some(tls)).await {
        Ok(mb) => mb,
        Err(e) => {
            error!("Can't create modbus connection: {e}");
            process::exit(1);
        }
    };

    let ssd = SunSpecData::default();
    match ss.populate_models(&ssd).await {
        Ok(m) => ss.models = m,
        Err(e) => {
            panic!("Can't populate models: {e}")
        }
    };

    // read fields
    let _model: u16 = 701_u16;
    let md = ss.models.get(&_model).unwrap().clone();
    let _fields: Vec<PointIdentifier> =
        vec![PointIdentifier::Catalog(".DERMeasureAC.ACType".to_string())];
    for f in _fields {
        match ss.clone().get_point(md.clone(), f.clone()).await {
            Ok(pt) => {
                println!("{_model}/{f} = {:#?}", pt.value);
            }
            Err(e) => {
                error!("Error received: {e}");
            }
        }
    }
}
