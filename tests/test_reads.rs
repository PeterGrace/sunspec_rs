use sunspec_rs::modbus_test_harness::string_to_vec_word;
use sunspec_rs::model_data::ModelData;
use sunspec_rs::sunspec_models::{PointIdentifier, ValueType};

mod common;

#[tokio::test]
pub async fn test_string() {
    let modelid = 1;
    let field: &str = "Mn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let expected: &str = "Test";

    let buf: Vec<u16> = string_to_vec_word(String::from(expected));
    let (ss, ssd, md) =
        common::setup(modelid, String::from(field), String::from("Test"), buf).await;
    if let Ok(pt) = ss
        .clone()
        .get_point(md.clone(), PointIdentifier::Point(field.to_string()))
        .await
    {
        if let Some(val) = pt.value {
            if let ValueType::String(testval) = val {
                assert_eq!(expected, testval);
            } else {
                panic!("Inappropriate responsetype")
            }
        } else {
            panic!("Err in pt.value");
        }
    } else {
        panic!("No point data returned");
    }
}

#[tokio::test]
pub async fn test_u16() {
    let modelid = 64263;
    let field: &str = "SnapRSDetectedCnt";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let expected: i64 = 6;

    let buf: Vec<u16> = vec![expected as u16];
    let (ss, ssd, md) =
        common::setup(modelid, String::from(field), String::from("Pika"), buf).await;
    if let Ok(pt) = ss
        .clone()
        .get_point(md.clone(), PointIdentifier::Point(field.to_string()))
        .await
    {
        if let Some(val) = pt.value {
            if let ValueType::Integer(testval) = val {
                assert_eq!(expected, testval);
            } else {
                panic!("Inappropriate responsetype")
            }
        } else {
            panic!("None in pt.value");
        }
    } else {
        panic!("No point data returned");
    }
}

#[tokio::test]
pub async fn test_u32() {
    let modelid = 64208;
    let field: &str = "WhIn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 3_u8;
    let expected: i64 = 6;

    let buf: Vec<u16> = vec![0b0, 0b110];
    let (ss, ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;
    if let Ok(pt) = ss
        .clone()
        .get_point(md.clone(), PointIdentifier::Point(field.to_string()))
        .await
    {
        if let Some(val) = pt.value {
            if let ValueType::Integer(testval) = val {
                assert!(expected <= testval);
            } else {
                panic!("Inappropriate responsetype")
            }
        } else {
            panic!("None in pt.value");
        }
    } else {
        panic!("No point data returned");
    }
}

#[tokio::test]
pub async fn test_bitfield16() {
    let modelid = 64264;
    let field: &str = "Status";
    let expected: u16 = 0b1001;
    let expected_strings: Vec<String> = vec![
        String::from("INSTALLED_COUNT_IS_LOCKED"),
        String::from("MANUAL_TEST_ACTIVE"),
    ];

    let buf: Vec<u16> = vec![expected];
    let (ss, ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;

    if let Ok(pt) = ss
        .clone()
        .get_point(md.clone(), PointIdentifier::Point(field.to_string()))
        .await
    {
        if let Some(val) = pt.value {
            if let ValueType::Array(testval) = val {
                assert_eq!(testval, expected_strings);
            } else {
                panic!("Inappropriate responsetype")
            }
        } else {
            panic!("None in pt.value");
        }
    } else {
        panic!("No point data returned");
    }
}

#[tokio::test]
pub async fn test_i32() {
    let modelid = 64252;
    let field: &str = "WhIn";
    let expected: i64 = 166095;
    // 0b101000100011001111

    let buf: Vec<u16> = vec![0b10, 0b1000100011001111];
    let (ss, ssd, md) =
        common::setup(modelid, String::from(field), String::from("Generac"), buf).await;

    if let Ok(pt) = ss
        .clone()
        .get_point(md.clone(), PointIdentifier::Point(field.to_string()))
        .await
    {
        if let Some(val) = pt.value {
            if let ValueType::Integer(testval) = val {
                assert_eq!(expected, testval);
            } else {
                panic!("Inappropriate responsetype")
            }
        } else {
            panic!("None in pt.value");
        }
    } else {
        panic!("No point data returned");
    }
}
