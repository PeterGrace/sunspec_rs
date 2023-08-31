use std::collections::HashMap;
use std::string::ToString;
use std::sync::Arc;
use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use tokio::sync::Mutex;
use tokio_modbus::{Address, Quantity, Slave};
use tokio_modbus::client::{Context, Reader, tcp};
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use crate::sunspec_data::{Model, Point, PointType, ResponseType, Symbol};
use async_recursion::async_recursion;

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
const POINT_TYPE_SUNSSF: &str = "sunssf";
const POINT_TYPE_PAD: &str = "pad";


pub type Word = u16;

#[derive(Debug, Clone)]
pub struct SunSpecConnection {
    pub(crate) ctx: Arc<Mutex<Context>>,
    slave_id: Option<Slave>,
    pub models: HashMap<u16, ModelData>,
}

#[derive(Default, Debug, Clone)]
pub struct ModelData {
    pub id: u16,
    pub len: u16,
    pub address: Address,
    pub model: Option<Model>,
    pub scale_factors: HashMap<String, i16>,
}
impl ModelData {
    pub async fn get_scale_factor(mut self, name: &str, mut conn: SunSpecConnection) -> Option<i16> {
        if let Some(value) = self.scale_factors.get(name) {
            return Some(*value);
        } else {
            if let Some(ResponseType::Integer(val)) = conn.get_point(self.clone(),name).await {
                self.scale_factors.insert(name.to_string(), val as i16);
                return Some(val as i16);
            } else {
                return None
            }
        }
    }
}


impl SunSpecConnection {
    pub async fn new(socket_addr: String, slave_id: Option<Slave>) -> anyhow::Result<Self> {
        let socket_addr = socket_addr.parse().unwrap();
        let mut ctx: Context;

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
        let retry_strategy = ExponentialBackoff::from_millis(1000)
            .map(jitter) // add jitter to delays
            .take(5);    // limit to 3 retries

        let ctx = self.ctx.clone();
        match Retry::spawn(retry_strategy, || { action_read_holding_registers(&ctx, addr, q) }).await {
            Ok(e) => Ok(e),
            Err(e) => {
                anyhow::bail!("Error when trying to retry modbus command: {e}");
            }
        }
    }

    pub async fn populate_models(mut self) -> HashMap<u16, ModelData> {
        let mut address = 40002;
        let mut models: HashMap<u16, ModelData> = HashMap::new();
        loop
        {
            let id = self.get_u16(address).await.unwrap();
            let length = self.get_u16(address + 1).await.unwrap();
            if id == SUNSPEC_END_MODEL_ID {
                break;
            }
            info!("found model with id {id}, and length {length}");
            models.insert(id, ModelData { id, len: length, address, model: None, scale_factors: HashMap::new() });
            address = address + length + 2;
        };
        models
    }
    #[async_recursion]
    pub async fn get_point(mut self, md: ModelData, name: &str) -> Option<ResponseType> {
        let mut point = Point::default();
        let mut symbols: Option<Vec<Symbol>> = None;

        if let Some(model) = md.model.clone() {
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
                    match self.get_string(2 + md.address + point.offset, point.len.unwrap()).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            return Some(ResponseType::String(rs));
                        }
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    };
                }
                POINT_TYPE_INT16 => {
                    match self.get_i16(2 + md.address + point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            if let Some(sf_name) = point.scale_factor {
                                if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                    let mut adj: f32 = 0.0;
                                    if sf >= 0 {
                                        adj = (rs * (10*sf.abs())).into();
                                    } else {
                                        adj = (rs / (10*sf.abs())).into();
                                    }

                                    return Some(ResponseType::Float(adj));
                                }
                            }
                            return Some(ResponseType::Integer(rs as i32));
                        }
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                }
                POINT_TYPE_UINT16 => {
                    match self.get_u16(2 + md.address + point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            if let Some(sf_name) = point.scale_factor {
                                if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                    let mut adj: f32 = 0.0;
                                    if sf >= 0 {
                                        adj = (rs.as_f32() * (10_f32*sf.abs() as f32));
                                    } else {
                                        adj = (rs.as_f32() / (10_f32*sf.abs() as f32));
                                    }
                                    return Some(ResponseType::Float(adj));
                                }
                            }
                            return Some(ResponseType::Integer(rs as i32));
                        }
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                }
                POINT_TYPE_ENUM16 => {
                    match self.get_u16(2 + md.address + point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            if symbols.is_some() {
                                let mut symbol_name: String = "".to_string();
                                symbols.unwrap().iter().for_each(|s| {
                                    if s.symbol.parse::<u16>().unwrap() == rs {
                                        symbol_name = s.id.clone();
                                    }
                                });
                                if symbol_name.len() > 0 {
                                    return Some(ResponseType::String(symbol_name));
                                } else {
                                    return None;
                                }
                            }
                        }
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                }
                POINT_TYPE_BITFIELD16 => {
                    match self.get_u16(2 + md.address + point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            if symbols.is_some() {
                                let mut values: Vec<String> = vec![];
                                let bv = BitVec::<_, Lsb0>::from_element(rs.clone());
                                for (idx, s) in symbols.unwrap().iter().enumerate() {
                                    if bv[s.symbol.parse::<usize>().unwrap()] {
                                        values.push(s.id.clone());
                                    };
                                };
                                return Some(ResponseType::Array(values));
                            } else {
                                warn!("We tried to parse a bitfield but there aren't symbols for this point.");
                                return None;
                            }
                        },
                        Err(e) => {
                            error!("{e}");
                            return None;
                        }
                    }
                },
                POINT_TYPE_SUNSSF => {
                    match self.get_i16(2 + md.address + point.offset).await {
                        Ok(rs) => {
                            info!("{}/{name} is {rs}!",model.name);
                            return Some(ResponseType::Integer(rs as i32));
                        }
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
    match ctx.read_holding_registers(addr, q).await {
        Ok(data) => {
            Ok(data)
        }

        Err(e) => {
            warn!("err occcurred in retry: {e}");
            anyhow::bail!("Error in retry: {e}");
        }
    }
}
