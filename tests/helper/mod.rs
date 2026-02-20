use linux_embedded_hal::{I2cdev, I2CError};
use adxl343::adxl343_interface::{ADXL343Interface};

pub fn setup_i2c_interface_with_adxl343() 
-> Result<ADXL343Interface<I2cdev>, I2CError>
{
    	//initializes an i2c device through the embedded linux hal lib
    	let i2c = I2cdev::new("/dev/i2c-1")?;
	let mut sensor = ADXL343Interface::new(i2c);
	Ok(sensor)
}
