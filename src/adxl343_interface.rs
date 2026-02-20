#![allow(unused)]
use embedded_hal::i2c::I2c;
use embedded_hal::i2c::Error as I2c_Error;
use crate::registers::REGISTER_SIZE;
use crate::{
    registers::{
        self, DEVID_ADDR, BW_RATE_ADDR, DATA_FORMAT_ADDR, DATAX0_ADDR, ADXL343_ADDR, DEVID_REG_VALUE, POWER_CTL_ADDR,
        accel_configs::{self, Alignment, POWER_CTL} 
    },
    utils::settings::ADXL343Settings,
};
use core::fmt::Debug;
use embedded_hal_mock::eh1::i2c::{Mock};

use core::{error::Error, fmt::{Display, Pointer}};

/// Device driver
pub struct ADXL343Interface<I>
where
    I: I2c,
{
    i2c: I,
    settings: ADXL343Settings,
}


impl<I> ADXL343Interface<I>
where
    I: I2c,
{
    /// Returns uninitialized device object with default settings
    pub fn new(i2c: I) -> Self {
        Self {
            i2c,
            settings: Default::default()
        }
    }

    /// Returns uninitialized device object with provided settings
    pub fn with_settings(&mut self, settings: ADXL343Settings) -> Result<(), ADXL343Error<I::Error>>{

        //prevents entering into measurement mode before the configs are specified
        if settings.in_measurement_mode(){
            return Err(ADXL343Error::MeasurementModeBeforeConfig);
        }
        self.settings = settings;
        Ok(())
    }

    /// Initializes the DATA_FORMAT register and BW_RATE registers with the configs located in 
    /// the settings field (type ADXL343Settings). Will not place the device in measurement mode
    pub fn init(&mut self) -> Result<(), ADXL343Error<I::Error>> {
        self.write_to_register(BW_RATE_ADDR, self.settings.BW_RATE_reg_value())?;
        self.write_to_register(DATA_FORMAT_ADDR, self.settings.DATA_FORMAT_reg_value())?;
        Ok(())
    }

    /// Ensures that the device responding to the device address 0xE5 has DEVID 0xE5 
    pub fn confirm_device(&mut self) -> Result<(), ADXL343Error<I::Error>>{

	let returned_value = self.read_register(DEVID_ADDR)?;
		match returned_value{
		    DEVID_REG_VALUE => Ok(()),
		    _ => Err(ADXL343Error::DeviceIdMismatch)
		}
		

    }

    /// toggles measurement bit to 1 in the POWER_CTL register to begin measurements
    /// does nothing in the event that measurement mode is already enabled
    /// 
    pub fn begin_measurements(&mut self) -> Result<(), ADXL343Error<I::Error>>{
        if (!self.settings.in_measurement_mode()){
            self.settings.toggle_measurement_mode();
            self.write_to_register(
                POWER_CTL_ADDR,
                POWER_CTL::default().with_measure(0x1).into_bytes()[0]
            )?;
        }
        Ok(())
    }

    pub fn turn_off_measurements(&mut self) ->  Result<(), ADXL343Error<I::Error>>{
        if (self.settings.in_measurement_mode()){
            self.settings.toggle_measurement_mode();
            self.write_to_register(
                POWER_CTL_ADDR,
                POWER_CTL::default().with_measure(0x0).into_bytes()[0]
            )?;
        }
        Ok(())
    }

    /// Returns raw accelerometer readings in the format:
    /// [x_low, x_high, y_low, y_high, z_low, z_high] (called DATA_0 and DATA_1 in the datasheet)
    pub fn read_full_sample(&mut self) -> Result<[u8; 6], ADXL343Error<I::Error>> {
        let mut read_buff = [0u8; 6];
        self.i2c.write_read(ADXL343_ADDR, &[DATAX0_ADDR], &mut read_buff)?;
        Ok(read_buff)
    }

    
    /// converts accel_data (represented as [lowbits, highbits]) into equivalent i16 representation
    #[inline]
    pub fn axis_value_raw(&mut self, accel_data: [u8; 2] ) -> i16{
        let mut axis_value = ((accel_data[1] as u16) << REGISTER_SIZE) | accel_data[0] as u16;
        let shift = 16 - self.settings.resolution_to_bits(); 
    
        //change right aligned reading into a left aligned reading
        if self.settings.get_justification() == Alignment::right {
            axis_value = axis_value << 6;
        };
        (axis_value as i16) >> shift
    }

    /// converts accel_data into its equivalent f32 representation
    #[inline]
    pub fn axis_value(&mut self, accel_data: i16) -> f32{
        (accel_data as f32) * self.settings.g_per_lsb()
    }

    /// accel reading [x_axis, y_axis, z_axis]
    pub fn read_accel(&mut self) -> Result<[f32; 3], ADXL343Error<I::Error>>{
        let binding = self.read_full_sample()?;
        let (axis_samples,_) = binding.as_chunks::<2>();
        let x_raw = self.axis_value_raw(axis_samples[0]);
        let y_raw = self.axis_value_raw(axis_samples[1]);
        let z_raw = self.axis_value_raw(axis_samples[2]);
        
        Ok(
            [
                self.axis_value(x_raw), 
                self.axis_value(y_raw), 
                self.axis_value(z_raw)
            ]
        )
    }

    pub fn read_register(&mut self, reg_address: u8) -> Result<u8, ADXL343Error<I::Error>> {
        let mut read_buff = [0u8];
        self.i2c.write_read(ADXL343_ADDR, &[reg_address], &mut read_buff)?;
        Ok(read_buff[0])
    }

    fn write_to_register(&mut self, reg_address: u8, value: u8) -> Result<(), ADXL343Error<I::Error>> {
        self.i2c.write(ADXL343_ADDR, &mut [reg_address, value])?;
        Ok(())
    }
    
    pub fn destroy(mut self) -> (I, ADXL343Settings) {
        self.turn_off_measurements();
        (self.i2c, self.settings)
    }

}

//embedded_hal::i2c::I2c::Error has been labeled as I2c_Error

#[derive(Debug)]
pub enum ADXL343Error<E: I2c_Error>
{
    Interface(E),         // error from I2C/SPI interface
    DeviceIdMismatch,     
    MeasurementModeBeforeConfig
}

impl<E: I2c_Error+ Debug> Error for ADXL343Error<E>{}

impl<E: I2c_Error+ Debug> Display for ADXL343Error<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ADXL343Error::Interface(i2c_error) => i2c_error.fmt(f),
            ADXL343Error::DeviceIdMismatch => {
                f.write_str("Wrong device ID returned")
            },
            ADXL343Error::MeasurementModeBeforeConfig => {
                f.write_str("Attempted to turn on measurement mode prior to configuration")
            }
        }
    }
}

impl<E: I2c_Error> From<E> for ADXL343Error<E> {
    fn from(value: E) -> Self {
        ADXL343Error::Interface(value)
    }
}

#[cfg(test)]
mod tests {
    use embedded_hal::i2c::ErrorKind;

    use super::*;
    use crate::adxl343_interface::{ADXL343Interface, ADXL343Settings};

    #[test]
    fn to_chunks_test(){
        let arr: [i16; 6] = [1, 2, 3, 4, 5, 6];
        let (binding, _) = arr.as_chunks::<2>();
        assert_eq!(binding, [[1,2], [3,4], [5,6]]);
    }

    //left justified, full resolution, 16g range (bits -> f32 pipeline test)
    #[test]
    fn bits_to_f32_pipeline_test() -> Result<(), ADXL343Error<ErrorKind>>{
        extern crate std;
        use std::println;

        //set to range = _16g's, justification (alignment) = left, and resolution = full => (13 bits of 16)
        let mut test_settings = ADXL343Settings::default()
        .range(accel_configs::AccelRange::_16g)
        .justification(accel_configs::Alignment::left)
        .resolution(accel_configs::FullRes::full_res);

        let mut test_interface = ADXL343Interface::new(Mock::new(&[]));
        test_interface.with_settings(test_settings)?;

        //example: 9g and -9g, both in lsb's  (least standard bits)

        let _9g_as_i16: i16 = test_interface.axis_value_raw(((9*256 as u16) << 3).to_le_bytes());
        let _minus9g_as_i16: i16 = test_interface.axis_value_raw(((0b1011100000000 as u16) << 3).to_le_bytes());
        
        assert_eq!(test_interface.axis_value(_9g_as_i16), 9.0);
        assert_eq!(test_interface.axis_value(_minus9g_as_i16), -9.0);
        
        let (mut i2c, _) = test_interface.destroy();
        i2c.done();

        Ok(())
    }
    
}
