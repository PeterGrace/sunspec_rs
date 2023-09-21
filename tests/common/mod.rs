use sunspec_rs::modbus_test_harness::ModbusTestHarness;
use sunspec_rs::model_data::ModelData;
use sunspec_rs::sunspec_connection::SunSpecConnection;
use sunspec_rs::sunspec_data::SunSpecData;

pub async fn setup(
    modelid: u16,
    model_field: String,
    manufacturer: String,
    buf_contents: Vec<u16>,
) -> (SunSpecConnection, SunSpecData, ModelData) {
    let meh = ModbusTestHarness { buf: buf_contents };
    let mut ss = match SunSpecConnection::test_new(meh, false).await {
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
    let model = ssd.clone().get_model(modelid, Some(manufacturer)).unwrap();
    let md = ModelData {
        id: modelid,
        len: model.model.len,
        address: model.model.id,
        model: model.clone(),
        scale_factors: Default::default(),
    };
    (ss, ssd, md)
}
