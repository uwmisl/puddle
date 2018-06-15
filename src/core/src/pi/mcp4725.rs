// https://cdn-shop.adafruit.com/datasheets/mcp4725.pdf

use super::{Result, I2cHandle};

// From Table 6.2
enum Command {
    WriteDac = 0b0100_0000,
    WriteDacAndEeprom = 0b1100_0000,
}

pub struct MCP4725 {
    i2c: I2cHandle,
}

pub const MCP4725_DEFAULT_ADDRESS: u16 = 0x60;

impl MCP4725 {
    pub fn new(i2c: I2cHandle) -> MCP4725 {
        MCP4725 { i2c }
    }

    pub fn write(&mut self, data: u16) -> Result<()> {
        self.do_write(data, Command::WriteDac)
    }

    pub fn write_and_save(&mut self, data: u16) -> Result<()> {
        self.do_write(data, Command::WriteDacAndEeprom)
    }

    fn do_write(&mut self, value: u16, cmd: Command) -> Result<()> {
        assert!(value < (1 << 12));
        let value_hi_8 = (value >> 4) as u8;
        let value_lo_4 = (value << 4) as u8;

        self.i2c.write(&[
            cmd as u8,
            value_hi_8,
            value_lo_4,
        ])
    }

}
