// https://cdn-shop.adafruit.com/datasheets/mcp4725.pdf
use rppal::i2c::I2c;


// From Table 6.2
enum Command {
    WriteDac = 0b0100_0000,
    WriteDacAndEeprom = 0b1100_0000,
}

pub struct MCP4725 {
    address: u16,
    i2c: I2c,
}

impl MCP4725 {
    pub fn new(address: u16) -> MCP4725 {
        let i2c = I2c::new().unwrap();
        MCP4725 { address, i2c }
    }

    pub fn write(&mut self, data: u16) {
        self.do_write(data, Command::WriteDac)
    }

    pub fn write_and_save(&mut self, data: u16) {
        self.do_write(data, Command::WriteDacAndEeprom)
    }

    fn do_write(&mut self, data: u16, cmd: Command) {
        assert!(data < (1 << 12));
        let data_hi_8 = (data >> 4) as u8;
        let data_lo_4 = (data << 4) as u8;

        self.i2c.set_slave_address(self.address).unwrap();
        self.i2c.write(&[
            cmd as u8,
            data_hi_8,
            data_lo_4,
        ]).unwrap();
    }

}
