#![allow(unused)]
mod helper;
use linux_embedded_hal::{I2cdev, I2CError};
use adxl343::{
	adxl343_interface::{ADXL343Interface, ADXL343Error},
	registers::accel_configs::Alignment,
	utils::settings::ADXL343Settings
};
										
#[test]
fn confim_device() -> Result<(), ADXL343Error<I2CError>>
{
	let mut sensor  = helper::setup_i2c_interface_with_adxl343()?;
   	sensor.confirm_device()?;
    	Ok(())
}

//confirms that regardless of left or right justification, the resulting 
//measurement (in lsb's) is equal
#[test]
fn left_right_justification_invariance() -> Result<(), ADXL343Error<I2CError>>
{
	let mut sensor = helper::setup_i2c_interface_with_adxl343()?;
	let mut sensor_right_justified = create_sensor_with_justification(sensor, Alignment::right)?;	
	let z_axis_reading_r = read_axis(&mut sensor_right_justified, Axis::Z)?;
	
	let mut sensor_left_justified = create_sensor_with_justification(sensor_right_justified, Alignment::left)?;
	let z_axis_reading_l = read_axis(&mut sensor_left_justified, Axis::Z)?;
	
	println!("left: {:b}, right: {:b}", z_axis_reading_l, z_axis_reading_r);	

	assert_eq!(z_axis_reading_r, z_axis_reading_l);
	Ok(())	
}
//returns the sensor, but with specified justification and measurements turned off
fn create_sensor_with_justification(mut sensor: ADXL343Interface<I2cdev>, justification: Alignment)
-> Result<ADXL343Interface<I2cdev>, ADXL343Error<I2CError>>
{
	sensor.turn_off_measurements()?;
	let (i2c, mut settings) = sensor.destroy();
	settings.set_justification(justification);
	let mut sensor = ADXL343Interface::<I2cdev>::new(i2c);
	sensor.with_settings(settings)?;
	sensor.init()?; sensor.begin_measurements()?;
	let _ = sensor.read_full_sample()?; //flush data buffer within sensor hardware
	Ok(sensor)	
}  

//reads the specified axis (i16) corresponding to the ADXL343 sensor
fn read_axis(sensor: &mut ADXL343Interface<I2cdev>, axis: Axis)
-> Result<i16, ADXL343Error<I2CError>> 
{
	let reading: [u8; 6] = sensor.read_full_sample()?;
	let axis_reading = match axis {
		Axis::X => [reading[0], reading[1]],
		Axis::Y => [reading[2], reading[3]],
		Axis::Z => [reading[4], reading[5]],
	}; 
	Ok(sensor.axis_value_raw(axis_reading))
	 
}


enum Axis{
	X,
	Y,
	Z
}



