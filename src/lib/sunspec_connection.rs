use crate::model_data::ModelData;
use crate::sunspec_data::SunSpecData;
use crate::sunspec_models::{Access, LiteralType, Point, Symbol, ValueType};
use async_recursion::async_recursion;
use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::string::ToString;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::{sleep, timeout};
use tokio_modbus::client::{tcp, Context, Reader, Writer};
use tokio_modbus::{Address, Quantity, Slave};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::RetryIf;

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

const NOT_ACCUMULATED_32: u32 = 0x00000000;
const NOT_IMPLEMENTED_U32: u32 = 0xffffffff;
const NOT_IMPLEMENTED_I32: u32 = 0x80000000;
const NOT_ACCUMULATED_16: u16 = 0x0000;
const NOT_IMPLEMENTED_U16: u16 = 0xffff;
const NOT_IMPLEMENTED_I16: u16 = 0x8000;

const ERROR_ILLEGAL_DATA_VALUE: &str = "Modbus function 3: Illegal data value";
const ERROR_GATEWAY_DEVICE_FAILED_TO_RESPOND: &str =
    "Modbus function 3: Gateway target device failed to respond";
const ERROR_INVALID_RESPONSE_HEADER: &str = "Invalid response header: expected/request";
const DEFAULT_NETWORK_TIMEOUT_MS: u64 = 10_000_u64;
const DEFAULT_BACKOFF_BASE_MS: u64 = 100_u64;

const RECONNECT_COURTESY_SLEEP_SECS: u64 = 10_u64;
pub type Word = u16;

#[derive(Error, Debug, Default, PartialEq)]
pub enum SunSpecCommError {
    #[error("Unrecoverable error: {0}")]
    FatalError(String),
    #[error("Received transient error, will retry.")]
    #[default]
    TransientError,
}

#[derive(Error, Debug, Default, PartialEq)]
pub enum SunSpecReadError {
    #[error("Error in read: {0}")]
    OtherError(String),
    #[error("Device reports this datapoint as not implemented.")]
    DatapointNotImplemented,
    #[error("default")]
    #[default]
    None,
}

#[derive(Error, Debug, Default, PartialEq)]
pub enum SunSpecWriteError {
    #[error("Supplied point does not exist.")]
    PointDoesntExist,
    #[error("Supplied point is not writeable.")]
    PointIsReadOnly,
    #[error("Supplied value does not match point type.")]
    ValueDoesntMatchPoint,
    #[error(
        "Value supplied exceeds defined point type (e.g, too long string or too large number.)"
    )]
    ValueWouldOverflow,
    #[error("An unspecified error occurred.")]
    #[default]
    Default,
}

/// A SunSpecConnection holds the address and slave id for the modbus connection, as well as the
/// actual connection object itself as well as the modeldata for all of the exposed models on
/// that connection.
#[derive(Debug, Clone)]
pub struct SunSpecConnection {
    /// an ip address:port pair resolved as a SocketAddr
    pub addr: SocketAddr,
    /// an optional number for modbus slave address
    slave_num: Option<u8>,
    /// the tokio-modbus Context object that is used for communication
    pub(crate) ctx: Arc<Mutex<Context>>,
    /// a map of the model definitions related to this connection (populated via populate_models)
    pub models: HashMap<u16, ModelData>,
}

impl SunSpecConnection {
    //region connection restart (todo)
    pub async fn restart_connection(mut self) {
        let mut ctx = self.ctx.lock().await;
        ctx.disconnect().await.unwrap();
        sleep(Duration::from_secs(RECONNECT_COURTESY_SLEEP_SECS)).await;

        let mut newctx: Context;
        if self.slave_num.is_some() {
            let slid = self.slave_num.unwrap();
            newctx = match tcp::connect_slave(self.addr, Slave(slid)).await {
                Ok(ctx) => {
                    info!("Reconnected to modbus client at {}/{}", self.addr, slid);
                    ctx
                }
                Err(e) => {
                    panic!("Can't reconnect to slave: {e}");
                }
            };
        } else {
            newctx = match tcp::connect(self.addr).await {
                Ok(ctx) => {
                    info!("Reconnected to modbus client at {}", self.addr);
                    ctx
                }
                Err(e) => {
                    panic!("Can't reconnect: {e}");
                }
            };
        }

        let arc_ctx = Arc::new(Mutex::new(newctx));
        drop(ctx);
        drop(self.ctx);
        self.ctx = arc_ctx;
    }
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
            None => None,
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
        Ok(SunSpecConnection {
            addr: socket_addr,
            slave_num: slave_num,
            ctx: arc_ctx,
            models: HashMap::new(),
        })
    }
    //endregion

    //region get value primitives
    /// Get a text string from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    /// * `quantity` - The number of 16-bit values to read from the bus
    pub async fn get_string(
        &mut self,
        addr: Address,
        quantity: Quantity,
    ) -> Result<String, SunSpecReadError> {
        let data = match self
            .clone()
            .retry_read_holding_registers(addr, quantity)
            .await
        {
            Ok(data) => data,
            Err(e) => {
                return Err(SunSpecReadError::OtherError(e.to_string()));
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
                return Err(SunSpecReadError::OtherError(e.to_string()));
            }
        }
    }
    /// Get a 16 bit signed integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_i16(&mut self, addr: Address) -> Result<i16, SunSpecReadError> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => {
                if data[0] == NOT_IMPLEMENTED_I16 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                }
                data[0]
            }
            Err(e) => return Err(SunSpecReadError::OtherError(e.to_string())),
        };
        Ok(data as i16)
    }
    /// Get a 16 bit unsigned integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_u16(&mut self, addr: Address) -> Result<u16, SunSpecReadError> {
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => {
                if data[0] == NOT_IMPLEMENTED_U16 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                }
                data[0]
            }

            Err(e) => return Err(SunSpecReadError::OtherError(e.to_string())),
        };
        Ok(data)
    }
    /// Set a 16 bit unsigned integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    /// * `data` - A single 16 bit unsigned integer.
    pub async fn set_u16(&mut self, addr: Address, data: u16) -> anyhow::Result<()> {
        let word: Word = data;
        match self.clone().retry_write_register(addr, word).await {
            Ok(_) => {}
            Err(e) => {
                anyhow::bail!("Could not write point: {e}");
            }
        };
        Ok(())
    }
    /// Get a 32 bit signed integer from the modbus connection.  Note, modbus holding registers
    /// are read in blocks of 16 bit words, so a 32 bit number is generated by reading two sequential
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_i32(&mut self, addr: Address) -> Result<i32, SunSpecReadError> {
        match self.clone().retry_read_holding_registers(addr, 2).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => {
                let val = (data[0] as i32) << 16 | data[1] as i32;
                if val == NOT_IMPLEMENTED_I32 as i32 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                } else {
                    Ok(val)
                }
            }
            Err(e) => {
                return Err(SunSpecReadError::OtherError(e.to_string()));
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
    pub async fn get_u32(&mut self, addr: Address) -> Result<u32, SunSpecReadError> {
        match self.clone().retry_read_holding_registers(addr, 2).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => {
                let val = (data[0] as u32) << 16 | data[1] as u32;
                if val == NOT_IMPLEMENTED_U32 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                } else {
                    Ok(val)
                }
            }
            Err(e) => {
                return Err(SunSpecReadError::OtherError(e.to_string()));
            }
        }
    }
    //endregion
    //region inner writing register retry logic
    pub(crate) async fn retry_write_register(
        self,
        addr: Address,
        data: Word,
    ) -> anyhow::Result<()> {
        let retry_strategy = ExponentialBackoff::from_millis(DEFAULT_BACKOFF_BASE_MS)
            .map(jitter) // add jitter to delays
            .take(3); // limit to 3 retries

        let ctx = self.ctx.clone();
        match RetryIf::spawn(
            retry_strategy,
            || action_write_register(&ctx, addr, data),
            |e: &SunSpecCommError| SunSpecCommError::TransientError == *e,
        )
        .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                self.restart_connection().await;
                anyhow::bail!("Error when trying to retry modbus command: {e}");
            }
        }
    }
    //endregion

    //region inner holding registers retry logic
    pub(crate) async fn retry_read_holding_registers(
        self,
        addr: Address,
        q: Quantity,
    ) -> anyhow::Result<Vec<Word>> {
        let retry_strategy = ExponentialBackoff::from_millis(DEFAULT_BACKOFF_BASE_MS)
            .map(jitter) // add jitter to delays
            .take(3); // limit to 3 retries

        let ctx = self.ctx.clone();
        match RetryIf::spawn(
            retry_strategy,
            || {
                let future = action_read_holding_registers(&ctx, addr, q);
                future
            },
            |e: &SunSpecCommError| SunSpecCommError::TransientError == *e,
        )
        .await
        {
            Ok(e) => Ok(e),
            Err(e) => {
                self.restart_connection().await;
                anyhow::bail!("Error when trying to retry modbus command: {e}");
            }
        }
    }
    //endregion

    //region gather models from the device and store them
    pub async fn populate_models(
        mut self,
        data: &SunSpecData,
    ) -> anyhow::Result<HashMap<u16, ModelData>> {
        let mut address = 40002;
        let mut models: HashMap<u16, ModelData> = HashMap::new();
        loop {
            let id = match self.get_i16(address).await {
                Ok(id) => id,
                Err(e) => {
                    anyhow::bail!("Can't get model id: {e}");
                }
            };
            let length = match self.get_u16(address + 1).await {
                Ok(length) => length,
                Err(e) => {
                    anyhow::bail!("Can't get model length: {e}");
                }
            };
            if id == SUNSPEC_END_MODEL_ID as i16 {
                break;
            }
            debug!("found model with id {id}, and length {length}");
            match ModelData::new(data.clone(), id as u16, length, address).await {
                Ok(md) => {
                    models.insert(id as u16, md);
                }
                Err(e) => {
                    error!("Couldn't create ModelData: {e}");
                }
            };
            address = address + length + 2;
        }
        Ok(models)
    }
    //endregion
    //region set point value
    /// Set a specific sunspec point. Checks if the point is writeable, and checks if the value
    /// can be set, then sends the value.
    ///
    /// # Arguments
    ///
    /// * `model_data` - A ModelData instance that has been initialized for the model you're
    ///                  trying to modify.
    /// * `name` - The name of the point you're querying, e.g. "PhVPhA" -- you can find these
    ///            values specified in the sunspec model files.
    /// * `value` - A ValueType enum set with the value you wish to push.
    ///
    /// # Response
    /// Returns a SunSpecWriteError if setting point fails, otherwise returns nothing.
    #[async_recursion]
    pub async fn set_point(
        mut self,
        md: ModelData,
        name: &str,
        data: ValueType,
    ) -> Result<(), SunSpecWriteError> {
        let mut point = Point::default();
        let model = md.model.model.clone();
        model.block.iter().for_each(|b| {
            b.point.iter().for_each(|p| {
                if p.id == name {
                    point = p.clone();
                }
            })
        });
        if point.id.len() == 0 {
            return Err(SunSpecWriteError::PointDoesntExist);
        }
        match point.access {
            None => {
                warn!("Can't determine if this point {name} is writeable, assuming read-only.");
                return Err(SunSpecWriteError::PointIsReadOnly);
            }
            Some(a) => match a {
                Access::ReadOnly => {
                    return Err(SunSpecWriteError::PointIsReadOnly);
                }
                Access::ReadWrite => {
                    debug!("point {name} deemed to be read-write, proceeding");
                }
            },
        };
        match point.r#type.as_str() {
            POINT_TYPE_UINT16 | POINT_TYPE_ENUM16 | POINT_TYPE_BITFIELD16 => {
                if let ValueType::Integer(val) = data {
                    if val < 0 {
                        return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                    }
                    if val.abs() > 0xffff {
                        return Err(SunSpecWriteError::ValueWouldOverflow);
                    }
                    match self
                        .set_u16(2 + md.address + point.offset, val as u16)
                        .await
                    {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            error!("Failure to write point: {e}");
                            return Err(SunSpecWriteError::Default);
                        }
                    }
                } else {
                    error!("Point type {POINT_TYPE_ENUM16} requires an integer to set.");
                    return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                }
            }
            POINT_TYPE_UINT32 | POINT_TYPE_ENUM32 | POINT_TYPE_BITFIELD32 => {
                if let ValueType::Integer(val) = data {
                    if val < 0 {
                        return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                    }
                    if val.abs() >= 0xfff_ffff {
                        return Err(SunSpecWriteError::ValueWouldOverflow);
                    }
                    match self
                        .set_u16(2 + md.address + point.offset, val as u16)
                        .await
                    {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            error!("Failure to write point: {e}");
                            return Err(SunSpecWriteError::Default);
                        }
                    }
                } else {
                    error!("Point type {POINT_TYPE_ENUM16} requires an integer to set.");
                    return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                }
            }
            &_ => {
                error!("Unimplemented write type.");
                return Err(SunSpecWriteError::Default);
            }
        }
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
    pub async fn get_point(mut self, mut md: ModelData, point_name: &str) -> Option<Point> {
        let mut point = Point::default();
        let mut symbols: Option<Vec<Symbol>> = None;
        let model = md.model.model.clone();
        let model_name = model.name;
        model.block.iter().for_each(|b| {
            b.point.iter().for_each(|p| {
                if p.id == point_name {
                    point = p.clone();
                    // if this point also has associated symbols (enum/bitfield), copy them in too
                    if p.symbol.is_some() {
                        symbols = p.symbol.clone();
                    }
                }
            })
        });
        if point.id.len() == 0 {
            warn!(
                "You asked for point {model_name}/{point_name} but it doesn't exist in the model."
            );
            return None;
        }
        //region if there's literals for this point, populate them
        for string in md.model.strings.iter() {
            for literal in string.literals.iter() {
                if let LiteralType::Point(point_literal) = literal {
                    if point_literal.id == point_name {
                        point.literal = Some(point_literal.clone());
                    }
                }
            }
        }

        //endregion

        match point.r#type.as_str() {
            POINT_TYPE_STRING => {
                match self
                    .get_string(2 + md.address + point.offset, point.len.unwrap())
                    .await
                {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        let mut val = rs.clone();
                        // it is unlikely anyone wants the extra nulls at the end of the string
                        val = val.trim_matches(char::from(0)).parse().ok()?;
                        point.value = Some(ValueType::String(val));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{model_name}/{point_name}: {e}");
                        return None;
                    }
                };
            }
            POINT_TYPE_INT16 => match self.get_i16(2 + md.address + point.offset).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                            let mut _adj: f32 = 0.0;
                            if sf >= 0 {
                                _adj = (rs * (10 * sf.abs())).into();
                            } else {
                                _adj = (rs / (10 * sf.abs())).into();
                            }
                            point.value = Some(ValueType::Float(_adj));
                            return Some(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i32));
                    return Some(point);
                }
                Err(e) => {
                    error!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    return None;
                }
            },
            POINT_TYPE_UINT16 | POINT_TYPE_ACC16 => {
                match self.get_u16(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        if point.r#type.as_str() == POINT_TYPE_ACC16 && rs == NOT_ACCUMULATED_16 {
                            error!(
                                "Accumulator datapoint not supported by device (0 value returned)"
                            );
                            return None;
                        }
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                                } else {
                                    _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                                }
                                point.value = Some(ValueType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ValueType::Integer(rs as i32));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{model_name}/{point_name}: {e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_ENUM16 => match self.get_u16(2 + md.address + point.offset).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if symbols.is_some() {
                        let mut symbol_name: String = "".to_string();
                        symbols.unwrap().iter().for_each(|s| {
                            if s.symbol.parse::<u16>().unwrap() == rs {
                                symbol_name = s.id.clone();
                            }
                        });
                        if symbol_name.len() > 0 {
                            point.value = Some(ValueType::String(symbol_name));
                            return Some(point);
                        } else {
                            return None;
                        }
                    }
                }
                Err(e) => {
                    error!("{model_name}/{point_name}: {e}");
                    return None;
                }
            },
            POINT_TYPE_BITFIELD16 => {
                match self.get_u16(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        if symbols.is_some() {
                            let mut values: Vec<String> = vec![];
                            let bv = BitVec::<_, Lsb0>::from_element(rs.clone());
                            for s in symbols.unwrap().iter() {
                                if bv[s.symbol.parse::<usize>().unwrap()] {
                                    values.push(s.id.clone());
                                };
                            }
                            point.value = Some(ValueType::Array(values));
                            return Some(point);
                        } else {
                            warn!("We tried to parse a bitfield but there aren't symbols for this point.");
                            return None;
                        }
                    }
                    Err(e) => {
                        error!("{model_name}/{point_name}: {e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_SUNSSF => match self.get_i16(2 + md.address + point.offset).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    point.value = Some(ValueType::Integer(rs as i32));
                    return Some(point);
                }
                Err(e) => {
                    error!("{model_name}/{point_name}: {e}");
                    return None;
                }
            },
            POINT_TYPE_UINT32 | POINT_TYPE_ACC32 => {
                match self.get_u32(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        if point.r#type.as_str() == POINT_TYPE_ACC32 && rs == NOT_ACCUMULATED_32 {
                            error!(
                                "Accumulator datapoint not supported by device (0 value returned)"
                            );
                            return None;
                        }
                        if let Some(sf_name) = point.clone().scale_factor {
                            if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                                let mut _adj: f32 = 0.0;
                                if sf >= 0 {
                                    _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                                } else {
                                    _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                                }
                                point.value = Some(ValueType::Float(_adj));
                                return Some(point);
                            }
                        }
                        point.value = Some(ValueType::Integer(rs as i32));
                        return Some(point);
                    }
                    Err(e) => {
                        error!("{model_name}/{point_name}: {e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_INT32 => match self.get_i32(2 + md.address + point.offset).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md.get_scale_factor(&sf_name, self.clone()).await {
                            let mut _adj: f32 = 0.0;
                            if sf >= 0 {
                                _adj = rs.as_f32() * (10_f32 * sf.abs() as f32);
                            } else {
                                _adj = rs.as_f32() / (10_f32 * sf.abs() as f32);
                            }
                            point.value = Some(ValueType::Float(_adj));
                            return Some(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i32));
                    return Some(point);
                }
                Err(e) => {
                    error!("{model_name}/{point_name}: {e}");
                    return None;
                }
            },
            POINT_TYPE_ENUM32 => match self.get_u32(2 + md.address + point.offset).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if symbols.is_some() {
                        let mut symbol_name: String = "".to_string();
                        symbols.unwrap().iter().for_each(|s| {
                            if s.symbol.parse::<u32>().unwrap() == rs {
                                symbol_name = s.id.clone();
                            }
                        });
                        if symbol_name.len() > 0 {
                            point.value = Some(ValueType::String(symbol_name));
                            return Some(point);
                        } else {
                            return None;
                        }
                    }
                }
                Err(e) => {
                    error!("{model_name}/{point_name}: {e}");
                    return None;
                }
            },
            POINT_TYPE_BITFIELD32 => {
                match self.get_u32(2 + md.address + point.offset).await {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        if symbols.is_some() {
                            let mut values: Vec<String> = vec![];
                            let bv = BitVec::<_, Lsb0>::from_element(rs.clone());
                            for s in symbols.unwrap().iter() {
                                if bv[s.symbol.parse::<usize>().unwrap()] {
                                    values.push(s.id.clone());
                                };
                            }
                            point.value = Some(ValueType::Array(values));
                            return Some(point);
                        } else {
                            warn!("We tried to parse a bitfield but there aren't symbols for this point.");
                            return None;
                        }
                    }
                    Err(e) => {
                        error!("{model_name}/{point_name}: {e}");
                        return None;
                    }
                }
            }
            POINT_TYPE_PAD => {}
            _ => {
                error!(
                    "{model_name}/{point_name}: unknown point type: {:#?}",
                    point.r#type.as_str()
                );
                return None;
            }
        }

        None
    }
    //endregion
}

//region actual code that reads holding registers (for retry logic)
pub(crate) async fn action_read_holding_registers(
    actx: &Arc<Mutex<Context>>,
    addr: Address,
    q: Quantity,
) -> Result<Vec<Word>, SunSpecCommError> {
    let mut ctx = actx.lock().await;
    match timeout(
        Duration::from_millis(DEFAULT_NETWORK_TIMEOUT_MS),
        ctx.read_holding_registers(addr, q),
    )
    .await
    {
        Ok(future) => match future {
            Ok(data) => Ok(data),
            Err(e) => match e.raw_os_error() {
                None => match e.to_string().as_str() {
                    ERROR_ILLEGAL_DATA_VALUE => {
                        return Err(SunSpecCommError::FatalError(
                            ERROR_ILLEGAL_DATA_VALUE.to_string(),
                        ));
                    }
                    ERROR_GATEWAY_DEVICE_FAILED_TO_RESPOND => {
                        return Err(SunSpecCommError::TransientError);
                    }
                    _ => {
                        if e.to_string().contains(ERROR_INVALID_RESPONSE_HEADER) {
                            return Err(SunSpecCommError::FatalError(String::from(
                                "out of order response",
                            )));
                        };
                        warn!("Non-os specific error: {e}");
                        return Err(SunSpecCommError::TransientError);
                    }
                },
                Some(code) => match code {
                    32 => {
                        return Err(SunSpecCommError::FatalError(e.to_string()));
                    }
                    _ => {
                        warn!("OS-specific error: {:#?}", e);
                        return Err(SunSpecCommError::TransientError);
                    }
                },
            },
        },
        Err(e) => {
            warn!("Timeout attempting read: {e}");
            return Err(SunSpecCommError::TransientError);
        }
    }
}
//endregion

//region actual code that writes a single register
pub(crate) async fn action_write_register(
    actx: &Arc<Mutex<Context>>,
    addr: Address,
    data: Word,
) -> Result<(), SunSpecCommError> {
    let mut ctx = actx.lock().await;
    match timeout(
        Duration::from_millis(DEFAULT_NETWORK_TIMEOUT_MS),
        ctx.write_single_register(addr, data),
    )
    .await
    {
        Ok(future) => match future {
            Ok(_) => Ok(()),
            Err(e) => match e.raw_os_error() {
                None => match e.to_string().as_str() {
                    ERROR_ILLEGAL_DATA_VALUE => {
                        return Err(SunSpecCommError::FatalError(
                            ERROR_ILLEGAL_DATA_VALUE.to_string(),
                        ));
                    }
                    _ => return Err(SunSpecCommError::TransientError),
                },
                Some(code) => match code {
                    _ => {
                        warn!("OS-specific error occurred in retry: {:#?}", e);
                        return Err(SunSpecCommError::TransientError);
                    }
                },
            },
        },
        Err(e) => {
            return Err(SunSpecCommError::TransientError);
        }
    }
}
//endregion
