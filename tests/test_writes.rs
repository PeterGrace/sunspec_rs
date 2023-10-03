use sunspec_rs::sunspec_connection::SunSpecWriteError;
use sunspec_rs::sunspec_models::ValueType;

mod common;

#[tokio::test]
pub async fn test_write_string_not_exist() {
    let modelid = 1;
    let field: &str = "Glorblurgl";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let value = ValueType::String(String::from("woohoo"));

    let buf: Vec<u16> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let (ss, _ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;
    match ss.clone().set_point(md.clone(), field, value).await {
        Ok(_) => {}
        Err(e) => {
            assert_eq!(e, SunSpecWriteError::PointDoesntExist)
        }
    }
}

#[tokio::test]
pub async fn test_write_string_read_only() {
    let modelid = 1;
    let field: &str = "Mn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let value = ValueType::String(String::from("woohoo"));

    let buf: Vec<u16> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let (ss, _ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;
    match ss.clone().set_point(md.clone(), field, value).await {
        Ok(_) => {}
        Err(e) => {
            assert_eq!(e, SunSpecWriteError::PointIsReadOnly)
        }
    }
}
#[tokio::test]
pub async fn test_write_string_wrong_type() {
    let modelid = 64263;
    let field: &str = "Enable";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let value = ValueType::String(String::from("woohoo"));

    let buf: Vec<u16> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let (ss, _ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;
    match ss.clone().set_point(md.clone(), field, value).await {
        Ok(_) => {}
        Err(e) => {
            assert_eq!(e, SunSpecWriteError::ValueDoesntMatchPoint);
        }
    }
}
#[tokio::test]
pub async fn test_write_enum32_value_too_big() {
    let modelid = 64263;
    let field: &str = "Enable";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let value = ValueType::Integer(77777_i32);

    let buf: Vec<u16> = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let (ss, _ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;
    match ss.clone().set_point(md.clone(), field, value).await {
        Ok(_) => {}
        Err(e) => {
            assert_eq!(e, SunSpecWriteError::ValueWouldOverflow);
        }
    }
}
