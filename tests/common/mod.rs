use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;

pub async fn setup(addr: &str, slave_id: u8) -> (SunSpecConnection, SunSpecData) {
    let socket_addr = addr.parse().unwrap();
    let mut ss = match SunSpecConnection::new(socket_addr, Some(slave_id)).await {
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
