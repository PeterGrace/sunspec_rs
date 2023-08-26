use std::collections::HashMap;
use crate::modbus::ModbusConnection;
use tokio_modbus::{Address, Quantity, Slave};
use tokio_modbus::client::Context;
use crate::sunspec_data::{Model, Point, PointType};

const SUNSPEC_END_MODEL_ID: u16 = 65535;




#[derive(Debug)]
pub struct SunSpecConnection {
    pub ctx: ModbusConnection,
    pub models: HashMap<u16, ModelData>
}

#[derive(Default,Debug, Clone)]
pub struct ModelData {
    pub id: u16,
    pub len: u16,
    pub address: Address,
    pub model: Option<Model>
}



impl SunSpecConnection {
    pub async fn new(socket_addr: String,slave_id: Option<Slave>) -> anyhow::Result<Self> {
        let ctx = ModbusConnection::new(socket_addr, slave_id).await.unwrap();

        Ok(
            SunSpecConnection {
                ctx,
                models: HashMap::new()
            }
        )
    }

    pub async fn populate_models(mut ctx: ModbusConnection) -> HashMap<u16,ModelData> {
        let mut address = 40002;
        let mut models: HashMap<u16,ModelData> = HashMap::new();
        loop
        {
            let id = ctx.get_u16(address).await.unwrap();
            let length = ctx.get_u16(address+1).await.unwrap();
            if id == SUNSPEC_END_MODEL_ID {
                break;
            }
            info!("found model with id {id} and length {length}");
            models.insert(id, ModelData{ id, len: length, address, model: None});
            address = address + length+2;
        };
        models
    }
    pub async fn get_point(mut self, md: ModelData, name: &str) -> Option<Point> {
        let mut point = Point::default();
        if let Some(model) = md.model {
            model.block.point.iter().for_each(|p| {
                if p.id == name {
                    point = p.clone();
                }
            });
            match point.r#type {
                PointType::string => {
                    let rs = match self.ctx.get_string(2+md.address+point.offset,point.len.unwrap()).await {
                        Ok(rs) => rs,
                        Err(e) => {
                            error!("{e}");
                            "".to_string()
                        }
                    };
                    info!("{}/{name} is {rs}!",model.name);
                },
                PointType::int16 => {}
                PointType::uint16 => {}
                PointType::acc16 => {}
                PointType::enum16 => {}
                PointType::bitfield16 => {}
                PointType::int32 => {}
                PointType::uint32 => {}
                PointType::acc32 => {}
                PointType::enum32 => {}
                PointType::bitfield32 => {}
                PointType::pad => {}
            }
        }
        None
    }
}