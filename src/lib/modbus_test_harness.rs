use crate::sunspec_connection::{SunSpecConn, Word};
use async_trait::async_trait;
use bitvec::macros::internal::funty::Fundamental;
use std::fmt::{Debug, Formatter};
use std::io::Error;
use thiserror::Error;
use tokio_modbus::client::{Client, Reader, Writer};
use tokio_modbus::prelude::SlaveContext;
use tokio_modbus::{Address, Quantity, Request, Response, Slave};

#[derive(Error, Debug)]
pub enum TestError {
    #[error("misc")]
    Misc(String),
}

pub struct ModbusTestHarness {
    pub buf: Vec<u16>,
}
type Coil = bool;

#[async_trait]
impl Writer for ModbusTestHarness {
    async fn write_single_coil(&mut self, _: Address, _: Coil) -> Result<(), Error> {
        todo!()
    }

    async fn write_single_register(&mut self, _: Address, _: Word) -> Result<(), Error> {
        todo!()
    }

    async fn write_multiple_coils(&mut self, addr: Address, data: &'_ [Coil]) -> Result<(), Error> {
        todo!()
    }

    async fn write_multiple_registers(
        &mut self,
        addr: Address,
        data: &[Word],
    ) -> Result<(), Error> {
        todo!()
    }

    async fn masked_write_register(&mut self, _: Address, _: Word, _: Word) -> Result<(), Error> {
        todo!()
    }
}

impl SunSpecConn for ModbusTestHarness {}
#[async_trait]
impl Client for ModbusTestHarness {
    async fn call(&mut self, request: Request<'_>) -> Result<Response, Error> {
        todo!()
    }
}

impl SlaveContext for ModbusTestHarness {
    fn set_slave(&mut self, slave: Slave) {
        todo!()
    }
}

impl Debug for ModbusTestHarness {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[async_trait]
impl Reader for ModbusTestHarness {
    async fn read_coils(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>, Error> {
        todo!()
    }

    async fn read_discrete_inputs(&mut self, _: Address, _: Quantity) -> Result<Vec<Coil>, Error> {
        todo!()
    }

    async fn read_holding_registers(
        &mut self,
        addr: Address,
        _: Quantity,
    ) -> Result<Vec<Word>, Error> {
        let address = addr.as_u16();
        return match address {
            40002 => {
                let resp: Vec<u16> = vec![1_u16];
                Ok(resp)
            }
            40003 => {
                let resp: Vec<u16> = vec![66_u16];
                Ok(resp)
            }
            40004 => {
                let resp = "Test".to_string();
                Ok(string_to_vec_word(resp))
            }
            40020 => {
                let resp = "TestHarness".to_string();
                Ok(string_to_vec_word(resp))
            }
            40052 => {
                let resp = "1234567890".to_string();
                Ok(string_to_vec_word(resp))
            }
            40070 => {
                let resp: Vec<u16> = vec![65535_u16];
                Ok(resp)
            }
            40071 => {
                let resp: Vec<u16> = vec![0_u16];
                Ok(resp)
            }
            _ => Ok(self.buf.clone()),
        };
    }

    async fn read_input_registers(&mut self, _: Address, _: Quantity) -> Result<Vec<Word>, Error> {
        todo!()
    }

    async fn read_write_multiple_registers(
        &mut self,
        read_addr: Address,
        read_count: Quantity,
        write_addr: Address,
        write_data: &[Word],
    ) -> Result<Vec<Word>, Error> {
        todo!()
    }
}

pub fn string_to_vec_word(input: String) -> Vec<u16> {
    debug_assert!(input.len() % 2 == 0);
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes(chunk.try_into().expect("slice has exactly 2 bytes")))
        .collect()
}

#[test]
fn test_stvw() {
    let s = "test".to_string();
    assert_eq!(vec![0x7465, 0x7374], string_to_vec_word(s));
}
