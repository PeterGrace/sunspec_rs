use tokio_modbus::client::{Context, Reader, tcp};
use anyhow;
use tokio_modbus::{Address, Quantity, Slave};

#[derive(Debug)]
pub struct ModbusConnection {
    ctx: Context,
    slave_id: Option<Slave>,

}

impl ModbusConnection {
    pub async fn new(socket_addr: String, slave_id: Option<Slave>) -> anyhow::Result<Self> {
        let socket_addr = "127.0.0.1:5083".parse().unwrap();

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
        Ok(
            ModbusConnection {
                ctx,
                slave_id,
            }
        )
    }
    pub async fn get_string(&mut self, addr: Address, quantity: Quantity) -> anyhow::Result<String> {
        let data = match self.ctx.read_holding_registers(addr, quantity).await {
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
    pub async fn get_u16(&mut self, addr: Address) -> anyhow::Result<u16> {
        let data = match self.ctx.read_holding_registers(addr, 1).await {
            Ok(data) => data[0],
            Err(e) => {
                anyhow::bail!("Can't read: {e}");
            }
        };
        Ok(data)
    }
}