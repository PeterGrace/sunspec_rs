use crate::json::group::{Group, GroupCount};
use crate::json::point::PointType;
use crate::metrics::{MODBUS_GET, MODBUS_SET};
use crate::modbus_test_harness::ModbusTestHarness;
use crate::model_data::ModelData;
use crate::sunspec_data::SunSpecData;
use crate::sunspec_models::{
    Access, GroupIdentifier, LiteralType, Model, ModelSource, OptionalGroupIdentifier, Point,
    PointIdentifier, Symbol, ValueType,
};
use anyhow::{anyhow, Error};
use async_recursion::async_recursion;
use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use bon::Builder;
use num_traits::pow::Pow;
use num_traits::ToPrimitive;
use pkcs8::der::Decode;
use pkcs8::EncryptedPrivateKeyInfo;
use rustls_pemfile::{certs, pkcs8_private_keys, private_key};
use serde::Deserialize;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;
use std::string::ToString;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_modbus::client::{tcp, Context, Reader, Writer};
use tokio_modbus::{Address, Quantity, Slave};
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::RetryIf;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio_rustls::TlsConnector;

pub const SUNSPEC_END_MODEL_ID: u16 = 65535;
pub const POINT_TYPE_STRING: &str = "string";
pub const POINT_TYPE_INT16: &str = "int16";
pub const POINT_TYPE_UINT16: &str = "uint16";
pub const POINT_TYPE_ACC16: &str = "acc16";
pub const POINT_TYPE_ENUM16: &str = "enum16";
pub const POINT_TYPE_BITFIELD16: &str = "bitfield16";
pub const POINT_TYPE_INT32: &str = "int32";
pub const POINT_TYPE_UINT32: &str = "uint32";
pub const POINT_TYPE_INT64: &str = "int64";
pub const POINT_TYPE_UINT64: &str = "uint64";
pub const POINT_TYPE_ACC32: &str = "acc32";
pub const POINT_TYPE_ACC64: &str = "acc64";
pub const POINT_TYPE_ENUM32: &str = "enum32";
pub const POINT_TYPE_BITFIELD32: &str = "bitfield32";
pub const POINT_TYPE_SUNSSF: &str = "sunssf";
pub const POINT_TYPE_PAD: &str = "pad";

pub const NOT_ACCUMULATED_64: u64 = 0x0000_0000_0000_0000;
pub const NOT_ACCUMULATED_32: u32 = 0x0000_0000;
pub const NOT_IMPLEMENTED_U32: u32 = 0xffffffff;
pub const NOT_IMPLEMENTED_U64: u64 = 0xffff_ffff_ffff_ffff;
pub const NOT_IMPLEMENTED_I32: u32 = 0x80000000;
pub const NOT_ACCUMULATED_16: u16 = 0x0000;
pub const NOT_IMPLEMENTED_U16: u16 = 0xffff;
pub const NOT_IMPLEMENTED_I16: u16 = 0x8000;

pub const ERROR_ILLEGAL_DATA_VALUE: &str = "Modbus function 3: Illegal data value";
pub const ERROR_GATEWAY_DEVICE_FAILED_TO_RESPOND: &str =
    "Modbus function 3: Gateway target device failed to respond";
pub const ERROR_INVALID_RESPONSE_HEADER: &str = "Invalid response header: expected/request";
pub const DEFAULT_NETWORK_TIMEOUT_MS: u64 = 10_000_u64;
pub const DEFAULT_BACKOFF_BASE_MS: u64 = 100_u64;

// Addresses are offset by 2. why?  I'd expect them to be offset in the negative per below
// ====
// excerpt from SunSpec Information Models v 12401
// Device Modbus maps begin at one of three well-known Modbus base addresses.
// Preferred Base Register: 40001
// Alternate Base Register: 50001
// Alternate Base Register: 00001
// Base registers are actual register offsets that start at 1 – not a function code and not
// to be confused with the Modicon convention, which would represent these as
// 4x40001 and 4x50001.
// To read register 40001, use the hexadecimal offset of 0x9C40 (40000) on the wire
// ====
// 2023-10-03: I think I know why and it's pretty obvious when I think about it.  Model Id and
// Model Length are each u16 values.  Maybe?
// 2025-04-16: I think that my above thought for 2023-10-03 is on the right track, as here I am working
// on the json model implementation and finding that my memory addresses are offset by 2.  json declares
// the model id and length in the data struct, where smdx does not.  If I apply ADDR_OFFSET when I read a json
// model, that'd push everything forward by two.
// 2025-05-31: Maybe the address is offset by two because of the `SunS` header at the start of the sunspec data block?
// 2025-06-02:
// Ah-hah: SunSpec DIM Spec v 1.1 says the Length point indicates the **REMAINING** number of registers left.  Since the
// first two registers are ID and L, that means everything's shifted by two (and any references to total length of model
// data would also need to have 2 added to them
pub(crate) const ADDR_OFFSET: u16 = 2_u16;

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
pub enum SunSpecPointError {
    #[error("Unrecoverable error: {0}")]
    CommError(String),
    #[error("General Error: {0}")]
    GeneralError(String),
    #[error("point does not exist: {0}")]
    DoesNotExist(String),
    #[error("Undefined error")]
    #[default]
    UndefinedError,
}

#[derive(Error, Debug, Default, PartialEq)]
pub enum SunSpecReadError {
    #[error("Comm Error in read: {0}")]
    CommError(String),
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
    #[error("Communication error: {0}")]
    CommError(String),
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
    #[error("General error when writing point: {0}")]
    General(String),
    #[error("An unspecified error occurred.")]
    #[default]
    Default,
}

pub trait SunSpecConn: Reader + Writer {}
impl SunSpecConn for Context {}
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
    pub(crate) ctx: Arc<Mutex<Box<dyn SunSpecConn>>>,
    /// a map of the model definitions related to this connection (populated via populate_models)
    pub models: HashMap<u16, ModelData>,
    /// a map of both the address and a retrieved value for each point, in JMES path format.
    pub catalog: HashMap<String, PointNode>,
    /// boolean value that causes get_point to force an error if a symbol doesn't exist.  A false
    /// value indicates that get_point can return a synthesized value instead (e.g., enum, bitfields)
    pub strict_symbol: bool,
}

/// PointNode is a single entry from the point catalog.  It contains a value and the address of the
/// point.
#[derive(Debug, Clone)]
pub struct PointNode {
    pub value: ValueType,
    pub address: u16,
    pub point_data: Point,
}

impl SunSpecConnection {
    /// Return a new sunspec connection which is ready to communicate with the modbus host.
    ///
    /// # Arguments
    ///
    /// * `socket_addr` - A String of format 'ip:port', e.g. '127.0.0.1:5021'
    /// * `slave_num` - An Option<u8> that indicates the targeted modbus slave device, if any
    /// * `strict_symbol` - whether to use strict symbol lookup or allow synthesized
    ///                     names based on point
    //region new sunspec connection

    pub async fn new(
        socket_addr: String,
        slave_num: Option<u8>,
        strict_symbol: bool,
        tls_config: Option<TlsConfig>,
    ) -> anyhow::Result<Self> {
        let socket_addr = socket_addr.parse().unwrap();
        let ctx: Context;
        let slave_id = match slave_num {
            Some(num) => Some(Slave(num)),
            None => None,
        };

        match tls_config {
            Some(tls) => {
                let mut root_cert_store = tokio_rustls::rustls::RootCertStore::empty();
                let ca_path = Path::new(&tls.ca_file);
                let mut pem = BufReader::new(File::open(ca_path)?);
                let certs = rustls_pemfile::certs(&mut pem).collect::<Result<Vec<_>, _>>()?;
                root_cert_store.add_parsable_certificates(certs);

                let cert_path = Path::new(&tls.client_cert_file);
                let key_path = Path::new(&tls.client_key_file);
                let certs = load_certs(cert_path)?;
                let key = load_keys(key_path, None)?;

                let config = tokio_rustls::rustls::ClientConfig::builder()
                    .with_root_certificates(root_cert_store)
                    .with_client_auth_cert(certs, key)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
                let connector = TlsConnector::from(Arc::new(config));

                let stream = TcpStream::connect(&socket_addr).await?;
                stream.set_nodelay(true)?;

                let domain = ServerName::try_from(tls.domain)
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

                let transport = match tokio::time::timeout(
                    Duration::from_secs(5),
                    connector.connect(domain, stream),
                )
                .await
                {
                    Ok(Ok(transport)) => transport,
                    Ok(Err(err)) => {
                        anyhow::bail!("TLS connection error: {err}");
                    }
                    Err(_) => {
                        anyhow::bail!("TLS connection timeout");
                    }
                };
                if slave_id.is_some() {
                    ctx = tcp::attach_slave(transport, slave_id.unwrap());
                } else {
                    ctx = tcp::attach(transport);
                }
            }
            None => {
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
            }
        }

        //let arc_ctx = Arc::new(Mutex::new(ctx));
        Ok(SunSpecConnection {
            addr: socket_addr,
            slave_num,
            ctx: Arc::new(Mutex::new(Box::new(ctx))),
            models: HashMap::new(),
            catalog: HashMap::new(),
            strict_symbol,
        })
    }

    pub async fn test_new(testbuf: ModbusTestHarness, strict_symbol: bool) -> anyhow::Result<Self> {
        Ok(SunSpecConnection {
            addr: "127.0.0.1:5083".parse()?,
            slave_num: Some(0_u8),
            ctx: Arc::new(Mutex::new(Box::new(testbuf))),
            models: HashMap::new(),
            catalog: HashMap::new(),
            strict_symbol,
        })
    }
    //endregion
    pub async fn get_raw(
        &mut self,
        addr: Address,
        amount: u16,
    ) -> Result<Vec<Word>, SunSpecReadError> {
        if amount > 100 {
            // we need to split this read into two distinct requests and then combine them
            let amount_left = 100;
            let address_left = addr;
            let amount_right = amount - amount_left;
            let address_right = addr + amount_left;

            let data_left = match self
                .retry_read_holding_registers(address_left, amount_left)
                .await
            {
                Ok(d) => d,
                Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
            };
            let data_right = match self
                .retry_read_holding_registers(address_right, amount_right)
                .await
            {
                Ok(d) => d,
                Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
            };
            let mut combined = data_left;
            combined.extend(data_right);
            Ok(combined)
        } else {
            let data = match self
                .clone()
                .retry_read_holding_registers(addr, amount)
                .await
            {
                Ok(data) => data,
                Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
            };
            Ok(data)
        }
    }
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
        let _ = MODBUS_GET.with_label_values(&["string"]).start_timer();
        let data = match self
            .clone()
            .retry_read_holding_registers(addr, quantity)
            .await
        {
            Ok(data) => data,
            Err(e) => {
                return Err(SunSpecReadError::CommError(e.to_string()));
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
        let _ = MODBUS_GET.with_label_values(&["i16"]).start_timer();
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => {
                if data[0] == NOT_IMPLEMENTED_I16 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                }
                data[0]
            }
            Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
        };
        Ok(data as i16)
    }
    /// Get a 16 bit unsigned integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    pub async fn get_u16(&mut self, addr: Address) -> Result<u16, SunSpecReadError> {
        let _ = MODBUS_GET.with_label_values(&["u16"]).start_timer();
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => {
                if data[0] == NOT_IMPLEMENTED_U16 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                }
                data[0]
            }

            Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
        };
        Ok(data)
    }
    /// Get a 16 bit unsigned integer from the modbus connection.  Do not check the value.
    /// This function is used in populate_models.
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002

    async fn get_u16_no_check(&mut self, addr: Address) -> Result<u16, SunSpecReadError> {
        let _ = MODBUS_GET.with_label_values(&["u16_nocheck"]).start_timer();
        let data = match self.clone().retry_read_holding_registers(addr, 1).await {
            Ok(data) => data[0],

            Err(e) => return Err(SunSpecReadError::CommError(e.to_string())),
        };
        Ok(data)
    }
    /// Set a 16 bit unsigned integer from the modbus connection
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002
    /// * `data` - A single 16 bit unsigned integer.
    pub async fn set_u16(&mut self, addr: Address, data: u16) -> Result<(), SunSpecWriteError> {
        let _ = MODBUS_SET.with_label_values(&["u16"]).start_timer();
        let word: Word = data;
        match self.clone().retry_write_register(addr, word).await {
            Ok(_) => {}
            Err(e) => {
                return Err(SunSpecWriteError::CommError(e.to_string()));
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
        let _ = MODBUS_GET.with_label_values(&["i32"]).start_timer();
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
                return Err(SunSpecReadError::CommError(e.to_string()));
            }
        }
    }
    /// Get a 64-bit unsigned integer from the modbus connection.  Note, modbus holding registers
    /// are read in blocks of 16 bit words, so a 64 bit number is generated by reading four sequential
    /// addresses.
    ///
    /// # Arguments
    ///
    /// * `addr` - A memory offset address to read, e.g. 40002

    pub async fn get_u64(&mut self, addr: Address) -> Result<u64, SunSpecReadError> {
        let _ = MODBUS_GET.with_label_values(&["u64"]).start_timer();
        match self.clone().retry_read_holding_registers(addr, 4).await {
            // because holding_registers works in 16 bit "words", we need to combine two words into
            // one word here to get a 32 bit number.
            Ok(data) => {
                let val = (data[0] as u64) << 48
                    | (data[1] as u64) << 32
                    | (data[2] as u64) << 16
                    | (data[3] as u64);

                if val == NOT_IMPLEMENTED_U64 {
                    return Err(SunSpecReadError::DatapointNotImplemented);
                } else {
                    Ok(val)
                }
            }
            Err(e) => {
                return Err(SunSpecReadError::CommError(e.to_string()));
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
        let _ = MODBUS_GET.with_label_values(&["u32"]).start_timer();
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
                return Err(SunSpecReadError::CommError(e.to_string()));
            }
        }
    }
    //endregion
    //region inner writing register retry logic

    pub(crate) async fn retry_write_register(
        self,
        addr: Address,
        data: Word,
    ) -> Result<(), SunSpecCommError> {
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
                return Err(e);
            }
        }
    }
    //endregion

    //region inner holding registers retry logic

    pub(crate) async fn retry_read_holding_registers(
        &mut self,
        addr: Address,
        q: Quantity,
    ) -> Result<Vec<Word>, SunSpecCommError> {
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
                return Err(e);
            }
        }
    }
    //endregion

    //region gather models from the device and store them

    pub async fn populate_models(
        &mut self,
        data: &SunSpecData,
    ) -> anyhow::Result<HashMap<u16, ModelData>> {
        let mut address = 40002;
        let mut models: HashMap<u16, ModelData> = HashMap::new();
        let manufacturer = match self.get_string(address + 2, 16).await {
            Ok(s) => match s.trim_matches(char::from(0)).parse() {
                Ok(s) => Some(s),
                Err(e) => {
                    warn!("Can't trim nulls on manufacturer name: {e}");
                    None
                }
            },
            Err(e) => {
                info!(
                    "[{}:{}] Can't get manufacturer of this unit: {e}",
                    self.addr,
                    self.slave_num.unwrap()
                );
                None
            }
        };
        loop {
            let id = match self.get_u16_no_check(address).await {
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
            if id == SUNSPEC_END_MODEL_ID {
                break;
            }
            assert!(id >= 1);
            info!("found model with id {id}, and length {length}");
            match ModelData::new(
                data.clone(),
                id as u16,
                length,
                address,
                manufacturer.clone(),
            )
            .await
            {
                Ok(md) => {
                    // if this is a json model, populate group catalog
                    if let ModelSource::Json(json) = md.clone().model.source {
                        if let Ok(mut data) = self.get_raw(md.address + 2, md.len).await {
                            process_json_group(
                                &mut data,
                                &json.group,
                                None,
                                &mut md.address.clone(),
                                &mut self.catalog,
                            )
                            .await;
                        }
                    }
                    models.insert(id as u16, md);
                }
                Err(e) => {
                    warn!("Couldn't create ModelData: {e}");
                }
            };
            address = address + length + ADDR_OFFSET;
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
        point_identifier: PointIdentifier,
        data: ValueType,
    ) -> Result<(), SunSpecWriteError> {
        let mut point = Point::default();
        let model = md.model.model.clone();
        let mut catalog_entry: Option<PointNode> = None;
        let mut name = String::new();

        match point_identifier {
            PointIdentifier::Catalog(catalog_name) => {
                info!("Catalog name: {catalog_name} specified.  Will use json-supplied point data");
                catalog_entry = self.catalog.get(&catalog_name).cloned();
                point = catalog_entry.clone().unwrap().point_data;
            }
            PointIdentifier::Point(point_str) => {
                name = point_str.clone();
                model.block.iter().for_each(|b| {
                    b.point.iter().for_each(|p| {
                        if p.id == name {
                            point = p.clone();
                        }
                    })
                });
            }
        };

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
        let write_addr = match catalog_entry {
            Some(pn) => pn.address,
            None => ADDR_OFFSET + md.address + point.offset,
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
                    match self.set_u16(write_addr, val as u16).await {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            debug!("write error: {e}");
                            return Err(e);
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
                    return Err(SunSpecWriteError::General(format!(
                        "writing 32bit not implemented yet"
                    )));
                    match self.set_u16(write_addr, val as u16).await {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            return Err(e);
                        }
                    }
                } else {
                    error!("Point type {POINT_TYPE_ENUM16} requires an integer to set.");
                    return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                }
            }
            POINT_TYPE_UINT64 => {
                if let ValueType::Integer(val) = data {
                    if val < 0 {
                        return Err(SunSpecWriteError::ValueDoesntMatchPoint);
                    }
                    if val.abs() >= 0xfff_ffff_fff_ffff {
                        return Err(SunSpecWriteError::ValueWouldOverflow);
                    }
                    return Err(SunSpecWriteError::General(format!(
                        "writing 64bit not implemented yet"
                    )));
                    match self.set_u16(write_addr, val as u16).await {
                        Ok(_) => return Ok(()),
                        Err(e) => {
                            return Err(e);
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
    pub async fn get_point(
        mut self,
        mut md: ModelData,
        point_identifier: PointIdentifier,
    ) -> Result<Point, SunSpecPointError> {
        let mut point = Point::default();
        let mut symbols: Option<Vec<Symbol>> = None;
        let model = md.model.model.clone();
        let model_name = model.clone().name;

        let mut catalog_entry: Option<PointNode> = None;
        let mut point_name: String = String::new();
        match point_identifier.clone() {
            PointIdentifier::Catalog(catalog_name) => {
                info!("Catalog name: {catalog_name} specified.  Will use json-supplied point data");
                catalog_entry = self.catalog.get(&catalog_name).cloned();
                if catalog_entry.is_some() {
                    point = catalog_entry.clone().unwrap().point_data;
                    symbols = point.symbol.clone();
                } else {
                    trace!("Requested {catalog_name}, not found. Iterating through available catalog points:");
                    for (c, _) in self.catalog.iter() {
                        trace!("{c}");
                    }
                    info!("Catalog entry for {catalog_name} not found.  Using default point data.");
                }
            }
            PointIdentifier::Point(point_str) => {
                point_name = point_str.clone();
                model.block.iter().enumerate().for_each(|(idx, b)| {
                    b.point.iter().for_each(|p| {
                        if p.id == point_str {
                            point = p.clone();
                            // if this point also has associated symbols (enum/bitfield), copy them in too
                            if p.symbol.is_some() {
                                symbols = p.symbol.clone();
                            }
                            return;
                        }
                    })
                });
            }
        };

        if point.id.len() == 0 {
            let err = format!(
                "You asked for point {model_name}/{point_identifier} but it doesn't exist in the specified block."
            );
            return Err(SunSpecPointError::DoesNotExist(err));
        }
        //region if there's literals for this point, populate them
        if catalog_entry.is_none() {
            for string in md.model.strings.iter() {
                for literal in string.literals.iter() {
                    if let LiteralType::Point(point_literal) = literal {
                        if point_literal.id == point_name {
                            point.literal = Some(point_literal.clone());
                        }
                    }
                }
            }
        }
        //endregion
        // TODO: Here's where I'd read catalog data for a static point
        let read_addr = match catalog_entry {
            Some(pn) => pn.address,
            None => ADDR_OFFSET + md.address + point.offset,
        };

        match point.r#type.as_str() {
            POINT_TYPE_STRING => {
                match self.get_string(read_addr, point.len.unwrap()).await {
                    Ok(rs) => {
                        debug!("{model_name}/{point_name} is {rs}!");
                        let mut val = rs.clone();
                        // it is unlikely anyone wants the extra nulls at the end of the string
                        val = match val.trim_matches(char::from(0)).parse() {
                            Ok(v) => v,
                            Err(e) => {
                                return Err(SunSpecPointError::GeneralError(format!(
                                    "Failure trimming string nulls: {e}"
                                )));
                            }
                        };
                        point.value = Some(ValueType::String(val));
                        return Ok(point);
                    }
                    Err(e) => {
                        let err = format!(
                            "{}:{} -- {model_name}/{point_name}: {e}",
                            self.addr,
                            self.slave_num.unwrap_or(0)
                        );
                        debug!(err);
                        if let SunSpecReadError::CommError(_) = e {
                            return Err(SunSpecPointError::CommError(err));
                        } else {
                            return Err(SunSpecPointError::GeneralError(err));
                        }
                    }
                };
            }
            POINT_TYPE_INT16 => match self.get_i16(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md
                            .get_scale_factor(&sf_name, self.clone(), None, None)
                            .await
                        {
                            point.value = Some(ValueType::Float(apply_scale_factor(rs, sf)));
                            return Ok(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_UINT16 | POINT_TYPE_ACC16 => match self.get_u16(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if point.r#type.as_str() == POINT_TYPE_ACC16 && rs == NOT_ACCUMULATED_16 {
                        let err = format!(
                            "Accumulator datapoint not supported by device (0 value returned)"
                        );
                        debug!(err);
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md
                            .get_scale_factor(&sf_name, self.clone(), None, None)
                            .await
                        {
                            point.value = Some(ValueType::Float(apply_scale_factor(rs, sf)));
                            return Ok(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_ENUM16 => match self.get_u16(read_addr).await {
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
                            return Ok(point);
                        } else {
                            if self.strict_symbol {
                                return Err(SunSpecPointError::GeneralError(
                                    format!("Enum failure: text symbol doesn't exist for point numeric value (point is {rs})"))
                                );
                            } else {
                                point.value = Some(ValueType::String(format!("ENUM16_{rs}")));
                                return Ok(point);
                            }
                        }
                    } else {
                        return Err(SunSpecPointError::GeneralError(
                        format!("An enum was queried but no symbols are present for the point, so can't render.")));
                    }
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_BITFIELD16 => match self.get_u16(read_addr).await {
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
                        return Ok(point);
                    } else {
                        return if self.strict_symbol {
                            let err = format!("We tried to parse a bitfield but there aren't symbols for this point.");
                            Err(SunSpecPointError::GeneralError(err))
                        } else {
                            point.value = Some(ValueType::String(format!("BITFIELD16_{rs}")));
                            Ok(point)
                        };
                    }
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_SUNSSF => match self.get_i16(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_UINT32 | POINT_TYPE_ACC32 => match self.get_u32(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if point.r#type.as_str() == POINT_TYPE_ACC32 && rs == NOT_ACCUMULATED_32 {
                        let err = format!(
                            "Accumulator datapoint not supported by device (0 value returned)"
                        );
                        debug!(err);
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md
                            .get_scale_factor(&sf_name, self.clone(), None, None)
                            .await
                        {
                            point.value = Some(ValueType::Float(apply_scale_factor(rs, sf)));
                            return Ok(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_UINT64 | POINT_TYPE_ACC64 => match self.get_u64(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if point.r#type.as_str() == POINT_TYPE_ACC64 && rs == NOT_ACCUMULATED_64 {
                        let err = format!(
                            "Accumulator datapoint not supported by device (0 value returned)"
                        );
                        debug!(err);
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md
                            .get_scale_factor(&sf_name, self.clone(), None, None)
                            .await
                        {
                            point.value = Some(ValueType::Float(apply_scale_factor(rs as f64, sf)));
                            return Ok(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_INT32 => match self.get_i32(read_addr).await {
                Ok(rs) => {
                    debug!("{model_name}/{point_name} is {rs}!");
                    if let Some(sf_name) = point.clone().scale_factor {
                        if let Some(sf) = md
                            .get_scale_factor(&sf_name, self.clone(), None, None)
                            .await
                        {
                            point.value = Some(ValueType::Float(apply_scale_factor(rs, sf)));
                            return Ok(point);
                        }
                    }
                    point.value = Some(ValueType::Integer(rs as i64));
                    return Ok(point);
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_ENUM32 => match self.get_u32(read_addr).await {
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
                            return Ok(point);
                        } else {
                            if self.strict_symbol {
                                return Err(SunSpecPointError::GeneralError(
                                        format!("Enum failure: text symbol doesn't exist for point numeric value (point is {rs})"))
                                    );
                            } else {
                                point.value = Some(ValueType::String(format!("ENUM32_{rs}")));
                                return Ok(point);
                            }
                        }
                    } else {
                        return Err(SunSpecPointError::GeneralError(
                format!("An enum was queried but no symbols are present for the point, so can't render.")));
                    }
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_BITFIELD32 => match self.get_u32(read_addr).await {
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
                        return Ok(point);
                    } else {
                        return if self.strict_symbol {
                            let err = format!("We tried to parse a bitfield but there aren't symbols for this point.");
                            Err(SunSpecPointError::GeneralError(err))
                        } else {
                            point.value = Some(ValueType::String(format!("BITFIELD32_{rs}")));
                            Ok(point)
                        };
                    }
                }
                Err(e) => {
                    let err = format!(
                        "{}:{} -- {model_name}/{point_name}: {e}",
                        self.addr,
                        self.slave_num.unwrap_or(0)
                    );
                    debug!(err);
                    if let SunSpecReadError::CommError(_) = e {
                        return Err(SunSpecPointError::CommError(err));
                    } else {
                        return Err(SunSpecPointError::GeneralError(err));
                    }
                }
            },
            POINT_TYPE_PAD => {
                point.value = Some(ValueType::Pad);
                return Ok(point);
            }
            _ => {
                let err = format!(
                    "{model_name}/{point_name}: unknown point type: {:#?}",
                    point.r#type.as_str()
                );
                debug!(err);
                return Err(SunSpecPointError::DoesNotExist(err));
            }
        }

        //Err(SunSpecPointError::UndefinedError)
    }
    //endregion
}

//region actual code that reads holding registers (for retry logic)

pub(crate) async fn action_read_holding_registers(
    actx: &Arc<Mutex<Box<dyn SunSpecConn>>>,
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
            Ok(data) => {
                trace!("Reading {q} words out of address {addr}: {data:#x?}");
                Ok(data)
            }
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
    actx: &Arc<Mutex<Box<dyn SunSpecConn>>>,
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
            warn!("Request timed out, retrying: {e}");
            return Err(SunSpecCommError::TransientError);
        }
    }
}
//endregion
pub fn apply_scale_factor<T, S>(value: T, sf: S) -> f64
where
    T: Copy + Into<f64>,
    S: Into<i32>,
{
    (value.into() as f64) * 10.0_f64.powi(sf.into())
}
pub fn get_block_id_from_name(model: &Model, s: &String) -> Option<usize> {
    for (idx, b) in model.block.iter().enumerate() {
        if let Some(name) = b.clone().name {
            if name == *s {
                return Some(idx);
            }
        }
    }
    None
}
#[async_recursion]
pub async fn process_json_group(
    data: &mut Vec<Word>,
    group: &Group,
    prefix: Option<String>,
    address: &mut u16,
    mut catalog: &mut HashMap<String, PointNode>,
) {
    let mut entries: i64 = 0;
    match &group.count {
        GroupCount::String(countval) => {
            let count_lookup = match prefix.clone() {
                Some(s) => format!(".{}.{}", s.split('.').nth(1).unwrap(), countval),
                None => format!(".{}", countval),
            };
            if let Some(num_of_groups_val) = catalog.get(&count_lookup) {
                if let ValueType::Integer(num_groups) = num_of_groups_val.value {
                    entries = num_groups as i64;
                }
            }
        }
        GroupCount::Integer(i) => {
            entries = i.to_i64().unwrap();
        }
    }
    for i in 0..entries {
        let newprefix = match prefix.clone() {
            Some(s) => {
                if entries > 1 {
                    format!("{s}.{}[{}]", group.name, i + 1)
                } else {
                    format!("{s}.{}", group.name)
                }
            }
            None => {
                if entries > 1 {
                    format!(".{}[{}]", group.name, i + 1)
                } else {
                    format!(".{}", group.name)
                }
            }
        };
        for p in group.points.iter() {
            if (p.name == "ID" || p.name == "L") && prefix.clone().is_none() {
                // we skip ID and L processing but still increment address
                *address += p.size as u16;
                continue;
            }
            let datum: Vec<Word> = data.drain(..p.size as usize).collect();
            match parse_point_data(&p, &datum) {
                Ok(v) => {
                    let pointname = format!("{}.{}", newprefix, p.name);
                    debug!("{}: {} @0x{} {:#?}", group.name, pointname, address, v);
                    // this is too simple, the actual solution will need to account for
                    // which group and group number the point belongs to
                    catalog.insert(
                        pointname,
                        PointNode {
                            value: v,
                            address: address.clone(),
                            point_data: p.clone().into(),
                        },
                    );
                }
                Err(e) => {
                    info!(
                        "Can't parse point {}.{} {:?}: {e}",
                        newprefix, p.name, p.type_
                    );
                }
            }
            *address += p.size as u16;
        }
        for g in group.groups.iter() {
            process_json_group(data, g, Some(newprefix.clone()), address, &mut catalog).await;
        }
    }
}
pub fn parse_point_data(p: &crate::json::point::Point, d: &Vec<Word>) -> anyhow::Result<ValueType> {
    match p.type_ {
        PointType::Uint16 | PointType::Int16 | PointType::Bitfield16 | PointType::Sunssf => {
            if let Some(pointval) = d[0].to_i64() {
                Ok(ValueType::Integer((pointval as i16) as i64))
            } else {
                Err(anyhow!("Can't convert to integer"))
            }
        }
        PointType::Enum16 => {
            if let Some(pointval) = d[0].to_i64() {
                for s in p.symbols.iter() {
                    if let Some(symbol_value) = s.value.as_i64() {
                        if symbol_value == pointval {
                            return Ok(ValueType::String(s.name.clone()));
                        }
                    }
                }
                Ok(ValueType::Integer((pointval as i16) as i64))
            } else {
                Err(anyhow!("Can't convert to integer"))
            }
        }
        PointType::String => {
            let bytes: Vec<u8> = d.iter().fold(vec![], |mut x, elem| {
                let f = elem.to_be_bytes();
                x.append(&mut f.to_vec());
                x
            });

            match String::from_utf8(bytes) {
                Ok(s) => Ok(ValueType::String(s.trim_matches(char::from(0)).parse()?)),
                Err(e) => Err(anyhow!("Couldn't parse data to string")),
            }
        }
        PointType::Uint32 | PointType::Int32 | PointType::Bitfield32 => {
            let val = (d[0] as u32) << 16 | (d[1] as u32);
            if val == NOT_IMPLEMENTED_U32 {
                return Err(anyhow!("Device reports Not implemented"));
            } else {
                Ok(ValueType::Integer(val as i64))
            }
        }
        PointType::Uint64 => {
            let val =
                (d[0] as u64) << 48 | (d[1] as u64) << 32 | (d[2] as u64) << 16 | (d[3] as u64);
            if val == NOT_IMPLEMENTED_U64 {
                return Err(anyhow!("Device reports Not implemented"));
            } else {
                Ok(ValueType::Integer(val as i64))
            }
        }
        PointType::Pad => Ok(ValueType::Pad),
        _ => Err(anyhow!("Point type is not implemented")),
    }
}

#[derive(Deserialize, Debug, Clone, Builder)]
pub struct TlsConfig {
    pub domain: String,
    pub ca_file: String,
    pub client_cert_file: String,
    pub client_key_file: String,
    pub insecure_skip_verify: Option<bool>,
    pub password: Option<String>,
}

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path, password: Option<&str>) -> io::Result<PrivateKeyDer<'static>> {
    let expected_tag = match &password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    if expected_tag.eq("PRIVATE KEY") {
        match private_key(&mut BufReader::new(File::open(path)?)) {
            Ok(f) => Ok(f.unwrap()),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No private key found",
            )),
        }
    } else {
        let content = std::fs::read(path)?;
        let mut iter = pem::parse_many(content)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?
            .into_iter()
            .filter(|x| x.tag() == expected_tag)
            .map(|x| x.contents().to_vec());

        match iter.next() {
            Some(key) => match password {
                Some(password) => {
                    let encrypted =
                        pkcs8::EncryptedPrivateKeyInfo::from_der(&key).map_err(|err| {
                            io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                        })?;
                    let decrypted = encrypted.decrypt(password).map_err(|err| {
                        io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                    })?;
                    let key = decrypted.as_bytes().to_vec();
                    match rustls_pemfile::read_one_from_slice(&key)
                        .expect("cannot parse private key .pem file")
                    {
                        Some((rustls_pemfile::Item::Pkcs1Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        Some((rustls_pemfile::Item::Pkcs8Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        Some((rustls_pemfile::Item::Sec1Key(key), _keys)) => {
                            io::Result::Ok(key.into())
                        }
                        _ => io::Result::Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "invalid key",
                        )),
                    }
                }
                None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
            },
            None => io::Result::Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key")),
        }
    }
}
