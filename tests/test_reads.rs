use sunspec_rs::sunspec_models::ValueType;

mod common;

#[tokio::test]
pub async fn test_string() {
    let modelid = 1;
    let field: &str = "Mn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 9_u8;
    let expected: &str = "Pika";

    let (ss, _) = common::setup(connection, slave).await;
    let md = ss.models.get(&modelid).unwrap().clone();
    if let Ok(pt) = ss.clone().get_point(md.clone(), field).await {
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
    let expected: i32 = 6;

    let (ss, _) = common::setup(connection, slave).await;
    let md = ss.models.get(&modelid).unwrap().clone();
    if let Ok(pt) = ss.clone().get_point(md.clone(), field).await {
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
    let expected: i32 = 6;

    let (ss, _) = common::setup(connection, slave).await;
    let md = ss.models.get(&modelid).unwrap().clone();
    if let Ok(pt) = ss.clone().get_point(md.clone(), field).await {
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
pub async fn test_bitfield32() {
    let modelid = 64208;
    let field: &str = "WhIn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 3_u8;
    let expected: i32 = 6;

    let (ss, _) = common::setup(connection, slave).await;
    let md = ss.models.get(&modelid).unwrap().clone();
    if let Ok(pt) = ss.clone().get_point(md.clone(), field).await {
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
pub async fn test_i32() {
    let modelid = 64252;
    let field: &str = "WhIn";
    let connection: &str = "127.0.0.1:5083";
    let slave: u8 = 5_u8;
    let expected: i32 = 166095;

    let (ss, _) = common::setup(connection, slave).await;
    let md = ss.models.get(&modelid).unwrap().clone();
    if let Ok(pt) = ss.clone().get_point(md.clone(), field).await {
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
