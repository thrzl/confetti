use anyhow::Result;
use thiserror::Error;
use wpihal::can::CANStreamMessage;

#[derive(Error, Debug)]
#[error("internal HAL error")]
pub struct HALError(#[from] wpihal::error::HALError);

// see https://github.com/grayson-arendt/sparkcan/blob/25167e908c9350a0047edc041e0a6420b6b77a76/include/SparkBase.hpp#L54C1-L78C3
pub enum CANCommands {
    ClearFaults = (6 << 4) | 14,
    FactoryDefaults = (7 << 4) | 4,
    FactoryReset = (7 << 4) | 5,
    Identify = (7 << 4) | 6,
    Heartbeat = (11 << 4) | 2,
    BurnFlash = (63 << 4) | 2,
    FirmwareVersion = (9 << 4) | 8,

    Setpoint = (0 << 4) | 1,
    DutyCycle = (0 << 4) | 2,
    Velocity = (1 << 4) | 2,
    SmartVelocity = (1 << 4) | 3,
    Position = (3 << 4) | 2,
    Voltage = (4 << 4) | 2,
    Current = (4 << 4) | 3,
    SmartMotion = (5 << 4) | 2,

    Period0 = (6 << 4) | 0,
    Period1 = (6 << 4) | 1,
    Period2 = (6 << 4) | 2,
    Period3 = (6 << 4) | 3,
    Period4 = (6 << 4) | 4,
}

pub fn set_percent(device_id: u8, percent: f32) -> Result<HALError> {
    let percent = percent.clamp(-1.0, 1.0);

    let arbitration_id = ((device_id as u32) << 5) | CANCommands::DutyCycle as u32;

    let data = percent.to_le_bytes();

    wpihal::can::send_message(arbitration_id, &data, 2000)?;
    Ok(())
}

pub fn set_voltage(device_id: u8, voltage: f32) -> Result<HALError> {
    let arbitration_id = ((device_id as u32) << 5) | CANCommands::DutyCycle as u32;
}
