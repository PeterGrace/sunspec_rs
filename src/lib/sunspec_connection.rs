use std::collections::HashMap;
use std::net::SocketAddr;
use std::string::ToString;
use std::sync::Arc;
use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use tokio::sync::Mutex;
use tokio_modbus::{Address, Quantity, Slave};
use tokio_modbus::client::{Context, Reader, tcp};
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use crate::sunspec_models::{LiteralType, Point, ResponseType, Symbol};
use crate::sunspec_data::SunSpecData;
use async_recursion::async_recursion;
use crate::model_data::ModelData;

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

/// A SunSpecConnection holds the address and slave id for the modbus connection, as well as the
/// actual connection object itself as well as the modeldata for all of the exposed models on
/// that connection.
#[derive(Debug, Clone)]
pub struct SunSpecConnection {
    /// an ip address:port pair resolved as a SocketAddr
    pub addr: SocketAddr,
    /// an optional number for modbus slave address
    _slave_num: Option<u8>,
    /// the tokio-modbus Context object that is used for communication
    pub(crate) ctx: Arc<Mutex<Context>>,
    /// a map of the model definitions related to this connection (populated via populate_models)
    pub models: HashMap<u16, ModelData>,
}


impl SunSpecConnection {
    //region connection restart (todo)
    // pub async fn restart_connection(mut self) -> anyhow::Result<()> {
    //     let mut ctx = self.ctx.lock().await;
    //     ctx.disconnect().await.unwrap();
    //
    //     let mut newctx: Context;
    //     if self.slave_num.is_some() {
    //         newctx = match tcp::connect_slave(self.addr, Slave(self.slave_num.unwrap())).await {
    //             Ok(ctx) => ctx,
    //             Err(e) => {
    //                 anyhow::bail!("Can't connect to slave: {e}");
    //             }
    //         };
    //     } else {
    //         newctx = match tcp::connect(self.addr).await {
    //             Ok(ctx) => ctx,
    //             Err(e) => {
    //                 anyhow::bail!("Can't connect: {e}");
    //             }
    //         };
    //     }
    //
    //     let arc_ctx = Arc::new(Mutex::new(newctx));
    //     self.ctx = arc_ctx;
    //     Ok(())
    //
    // }
    //endregion

    /// Return a new sunspec connection which is ready to communicate with the modbus host.
    ///
    /// # Arguments
    ///
    /// * `socket_addr` - A String of format 'ip:port', e.g. '127.0.0.1:5021'
    /// * `slave_num` - An Option<u8> that indicates the targeted modbus slave device, if any
    //region new sunspec connection
    pub async fn new(socket_addr: String, slave_num: Option<u8>) -> anyhow::Result<Self> {
        let socket_addr = socket_addr.parse().unwrap();
        let ctx: Context;
        let slave_id = match slave_num {
            Some(num) => Some(Slave(num)),
            None => None
        };

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
                addr: socket_addr,
                _slave_num: slave_num,
                ctx: arc_ctx,
                models: HashMap::new(),
            }
        )
    }
    //endregion

    //region get value primitives
    /// Get a text string from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    /// * `quantity` - The number of 16-bit values to read from the bus
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
    /// Get a 16 bit signed integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_i16(&mut self, addr: Address) -> anyhow::Result<i16> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => data[0],
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        Ok(data as i16)
    }
    /// Get a 16 bit unsigned integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_u16(&mut self, addr: Address) -> anyhow::Result<u16> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => data[0],
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        Ok(data)
    }
    /// Get a 32 bit signed integer from the modbus connection.  Note, modbus holding registers
    /// are read in blocks of 16 bit words, so a 32 bit number is generated by reading two sequential
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_i32(&mut self, addr: Address) -> anyhow::Result<i32> {
        match self.clone().retry_read_holding_registers(addr, 2).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => Ok((data[0] as i32) << 16 | data[1] as i32),
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        }
    }
    /// Get a 32-bit unsigned integer from the modbus connection.  Note, modbus holding registers
    /// are read in blocks of 16 bit words, so a 32 bit number is generated by reading two sequential
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_u32(&mut self, addr: Address) -> anyhow::Result<u32> {
        match self.clone().retry_read_holding_registers(addr, 2).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => Ok((data[0] as u32) << 16 | data[1] as u32),
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        }
    }
    //endregion

    //region inner holding registers retry logic
    pub async fn retry_read_holding_registers(self, addr: Address, q: Quantity) -> anyhow::Result<Vec<Word>> {
        let retry_strategy = ExponentialBackoff::from_millis(500)
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
    //endregion

    //region gather models from the device and store them
    pub async fn populate_models(mut self, data: SunSpecData) -> HashMap<u16, ModelData> {
        let mut address = 40002;
        let mut models: HashMap<u16, ModelData> = HashMap::new();
        loop
        {
            let id = self.get_u16(address).await.unwrap();
            let length = self.get_u16(address + 1).await.unwrap();
            if id == SUNSPEC_END_MODEL_ID {
                break;
            }
            debug!("found model with id {id}, and length {length}");
            match ModelData::new(data.clone(), id, length, address).await {
                Ok(md) => {
                    models.insert(id, md);
                }
                Err(e) => {
                    error!("Couldn't create ModelData: {e}");
                }
            };
            address = address + length + 2;
        };
        models
    }
    //endregion

    //region get well-formed point for return to caller
    /// Get a specific sunspec point from the modbus. Returns a Point object, which will have
    /// relevant data about the point, as well as the retrieved value, populated for use.
    ///
    /// # Arguments
    ///
    /// * `model_data` - A ModelData instance that has been initialized for the model you're
    ///                  trying to query.
    /// * `name` - The name of the point you're querying, e.g. "PhVPhA" -- you can find these
    ///            values specified in the sunspec model files.
    #[async_recursion]
    pub async fn get_point(mut self, md: ModelData, name: &str) -> Option<Point> {
        let mut point = Point::default();
        let mut symbols: Option<Vec<Symbol>> = None;
        let model = md.model.model.clone();
        model.block.iter().for_each(|b| {
            b.point.iter().for_each(|p| {
                if p.id == name {
                    point = p.clone();
                    // if this point also has associated symbols (enum/bitfield), copy them in too
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
        //region if there's literals for this point, populate them
        for string in md.model.strings.iter() {
            for literal in string.literals.iter() {
                if let LiteralType::Point(point_literal) = literal {
                    if point_literal.id == name {
                        point.literal = Some(point_literal.clone());
                    }
                }
            }
        }

        //endregion

        match point.r#type.as_str() {
            POINT_TYPE_STRING => {
                match self.get_string(2 + md.address + point.offset, point.len.unwrap()).await {
                    Ok(rs) => {
                        debug!("{}/{name} is {rs}!",model.name);
                        let mut val = rs.clone();
                        // it is unlikely anyone wants the extra nulls at the end of the string
                        val = val.trim_matches(char::from(0)).parse().ok()?;
                        point.value = Some(ResponseType::String(val));
                        return Some(point);
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
                        debug!("{}/{name} is {rs}!",model.name);
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = (rs * (10 * sf.abs())).into();
                                } else {
                                    _adj = (rs / (10 * sf.abs())).into();
                                }
                                point.value = Some(ResponseType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ResponseType::Integer(rs as i32));
                        return Some(point);
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
                        debug!("{}/{name} is {rs}!",model.name);
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                                } else {
                                    _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                                }
                                point.value = Some(ResponseType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ResponseType::Integer(rs as i32));
                        return Some(point);
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
                        debug!("{}/{name} is {rs}!",model.name);
                        if symbols.is_some() {
                            let mut symbol_name: String = "".to_string();
                            symbols.unwrap().iter().for_each(|s| {
                                if s.symbol.parse::<u16>().unwrap() == rs {
                                    symbol_name = s.id.clone();
                                }
                            });
                            if symbol_name.len() > 0 {
                                point.value = Some(ResponseType::String(symbol_name));
                                return Some(point);
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
                        debug!("{}/{name} is {rs}!",model.name);
                        if symbols.is_some() {
                            let mut values: Vec<String> = vec![];
                            let bv = BitVec::<_, Lsb0>::from_element(rs.clone());
                            for s in symbols.unwrap().iter() {
                                if bv[s.symbol.parse::<usize>().unwrap()] {
                                    values.push(s.id.clone());
                                };
                            };
                            point.value = Some(ResponseType::Array(values));
                            return Some(point);
                        } else {
                            warn!("We tried to parse a bitfield but there aren't symbols for this point.");
                            return None;
                        }
                    }
                    Err(e) => {
                        error!("{e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_SUNSSF => {
                match self.get_i16(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{}/{name} is {rs}!",model.name);
                        point.value = Some(ResponseType::Integer(rs as i32));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_UINT32 => {
                match self.get_u32(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{}/{name} is {rs}!",model.name);
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                                } else {
                                    _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                                }
                                point.value = Some(ResponseType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ResponseType::Integer(rs as i32));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_ACC16 => {
                warn!("16-bit accumulator isn't implemented yet");
                return None;
            }
            POINT_TYPE_INT32 => {
                match self.get_i32(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{}/{name} is {rs}!",model.name);
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                                } else {
                                    _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                                }
                                point.value = Some(ResponseType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ResponseType::Integer(rs as i32));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_ACC32 => {}
            POINT_TYPE_ENUM32 => {
                match self.get_u32(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{}/{name} is {rs}!",model.name);
                        if symbols.is_some() {
                            let mut symbol_name: String = "".to_string();
                            symbols.unwrap().iter().for_each(|s| {
                                if s.symbol.parse::<u32>().unwrap() == rs {
                                    symbol_name = s.id.clone();
                                }
                            });
                            if symbol_name.len() > 0 {
                                point.value = Some(ResponseType::String(symbol_name));
                                return Some(point);
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
            POINT_TYPE_BITFIELD32 => {}
            POINT_TYPE_PAD => {}
            _ => {
                error!("unknown point type: {:#?}",point.r#type.as_str());
                return None;
            }
        }

        None
    }
    //endregion
}

//region acftual code that reads holding registers (for retry logic)
pub async fn action_read_holding_registers(actx: &Arc<Mutex<Context>>, addr: Address, q: Quantity) -> anyhow::Result<Vec<Word>> {
    let mut ctx = actx.lock().await;
    match ctx.read_holding_registers(addr, q).await {
        Ok(data) => {
            Ok(data)
        }

        Err(e) => {
            match e.raw_os_error() {
                None => {
                    warn!("err occurred in retry: {e}");
                    anyhow::bail!("Error in retry: {e}");
                }
                Some(code) => {
                    match code {
                        // 32 => {
                        //     //
                        // }
                        _ => {
                            warn!("err occurred in retry: {e}");
                            anyhow::bail!("Error in retry: {e}");
                        }
                    }
                }
            }
        }
    }
}
//endregion