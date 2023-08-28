use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_modbus::{Address, Quantity, Slave};
use tokio_modbus::client::{Context, Reader, tcp};
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use crate::sunspec_data::{Model, Point, PointType, Symbol};

const SUNSPEC_END_MODEL_ID: u16 = 65535;
const POINT_TYPE_STRING: &str = "string";
const POINT_TYPE_INT16: &str = "int16";
const POINT_TYPE_UINT16: &str = "uint16";
const POINT_TYPE_ACC16: &str = "acc16";
const POINT_TYPE_ENUM16: &str = "enum16";
const POINT_TYPE_BITFIELD16: &str = "bitfield16";
const POINT_TYPE_INT32: &str = "int32";
const POINT_TYPE_UINT32: &str = "uint32";
const POINT_TYPE_ACC32: &str = "acc32";
const POINT_TYPE_ENUM32: &str = "enum32";
const POINT_TYPE_BITFIELD32: &str = "bitfield32";
const POINT_TYPE_PAD: &str ="pad";


pub type Word = u16;

#[derive(Debug, Clone)]
pub struct SunSpecConnection {
    pub(crate) ctx: Arc<Mutex<Context>>,
    slave_id: Option<Slave>,
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
        let socket_addr = socket_addr.parse().unwrap();

        let mut ctx :Context;
        if slave_id.is_some() {
            ctx = match tcp::connect_slave(socket_addr, slave_id.unwrap()).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    anyhow::bail!("Can't connect to slave: {e}");
                }
            };
        } else {
            ctx = match tcp::connect(socket_addr).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    anyhow::bail!("Can't connect: {e}");
                }
            };
        }
        let arc_ctx = Arc::new(Mutex::new(ctx));
        Ok(
            SunSpecConnection {
                ctx: arc_ctx,
                models: HashMap::new(),
                slave_id,
            }
        )
    }
    pub async fn get_string(&mut self, addr: Address, quantity: Quantity) -> anyhow::Result<String> {
        let data = match self.clone().retry_read_holding_registers(addr, quantity).await {
            Ok(data) => data,
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        let bytes: Vec<u8> = data.iter().fold(vec![], |mut x, elem| {
            let f = elem.to_be_bytes();
            x.append(&mut f.to_vec());
            x
        });
        match String::from_utf8(bytes) {
            Ok(s) => Ok(s),
            Err(e) => {
                anyhow::bail!("Couldn't format as string: {e}");
            }
        }
    }
    pub async fn get_i16(&mut self, addr: Address) -> anyhow::Result<i16> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => data[0],
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        Ok(data as i16)
    }
    pub async fn get_u16(&mut self, addr: Address) -> anyhow::Result<u16> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => data[0],
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        Ok(data)
    }
    pub async fn retry_read_holding_registers(mut self, addr: Address, q: Quantity) -> anyhow::Result<Vec<Word>> {
        let retry_strategy = ExponentialBackoff::from_millis(10)
            .map(jitter) // add jitter to delays
            .take(3);    // limit to 3 retries

        let ctx = self.ctx.clone();
        match Retry::spawn(retry_strategy,||{action_read_holding_registers(&ctx,addr,q)}).await {
            Ok(e) => Ok(e),
            Err(e) => {
                anyhow::bail!("Error when trying to retry modbus command: {e}");
            }
        }
    }

    pub async fn populate_models(mut self) -> HashMap<u16,ModelData> {
        let mut address = 40002;
        let mut models: HashMap<u16,ModelData> = HashMap::new();
        loop
        {
            let id = self.get_u16(address).await.unwrap();
            let length = self.get_u16(address+1).await.unwrap();
            if id == SUNSPEC_END_MODEL_ID {
                break;
            }
            info!("found model with id {id} and length {length}");
            models.insert(id, ModelData{ id, len: length, address, model: None});
            address = address + length+2;
        };
        models
    }
    pub async fn get_point(mut self, md: ModelData, name: &str) -> Option<PointType> {
        let mut point = Point::default();
        let mut symbols: Option<Vec<Symbol>> = None;
        if let Some(model) = md.model {
            model.block.iter().for_each(|b| {
                b.point.iter().for_each(|p| {
                    if p.id == name {
                        point = p.clone();
                        if p.symbol.is_some() {
                            symbols = p.symbol.clone();
                        }
                    }
                })
            }
            );
            if point.id.len() == 0 {
                warn!("You asked for point {name} but it doesn't exist in the model.");
                return None;
            }



            match point.r#type.as_str() {
                POINT_TYPE_STRING => {
                    match self.get_string(2+md.address+point.offset,point.len.unwrap()).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            return Some(PointType::string(rs));
                        },
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    };
                },
                POINT_TYPE_INT16 => {
                    match self.get_i16(2+md.address+point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            return Some(PointType::int16(rs));
                        },
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                }
                POINT_TYPE_UINT16 => {
                    match self.get_u16(2+md.address+point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            return Some(PointType::uint16(rs));
                        },
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                }
                POINT_TYPE_ENUM16 => {
                    match self.get_u16(2+md.address+point.offset).await {
                        Ok(rs) => {

                            info!("{}/{name} is {rs}!",model.name);
                            if symbols.is_some() {
                                let mut symbol_name: String = "".to_string();
                                symbols.unwrap().iter().for_each(|s| {
                                    if s.symbol == rs.to_string() {
                                        symbol_name = s.id.clone();
                                    }
                                });
                                if symbol_name.len() > 0 {
                                    return Some(PointType::enum16(symbol_name));
                                } else {
                                    return None;
                                }
                            }
                        },
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                },
                _ => {
                    info!("{:#?}",point.r#type.as_str());
                    todo!("unimplemented");
                }
            }
        }
        None
    }
}
pub async fn action_read_holding_registers(mut actx: &Arc<Mutex<Context>>, addr: Address, q: Quantity) -> anyhow::Result<Vec<Word>> {
    let mut ctx = actx.lock().await;
    match ctx.read_holding_registers(addr, q).await{
        Ok(data) => {
            Ok(data)
        },

        Err(e) => {
            anyhow::bail!("Error in retry: {e}");
        }
    }
}
