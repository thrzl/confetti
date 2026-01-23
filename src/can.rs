use std::time::{Duration, SystemTime};
use std::{ops::Add, time::Instant};

use std::sync::LazyLock;
use thiserror::Error;
use wpihal::can::CANStreamMessage;
pub use wpihal::{can as hal_can, can_api};

#[derive(Error, Debug)]
pub enum HALError {
    #[error("internal HAL error")]
    Internal(#[from] wpihal::error::HALError),

    #[error("error decoding CAN message")]
    DecodeError,
}

type HALResult<T> = Result<T, HALError>;

// see https://github.com/grayson-arendt/sparkcan/blob/25167e908c9350a0047edc041e0a6420b6b77a76/include/SparkBase.hpp#L54C1-L78C3
#[derive(Clone, Copy)]
pub enum CANCommand {
    /// for when we don't care what the value is
    Null = 0,

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

pub enum IdleMode {
    Brake,
    Coast,
}

pub enum CANResponse {
    Period0(Period0Status),
    Period1(Period1Status),
    Period2(Period2Status),
    Period3(Period3Status),
    Period4(Period4Status),
}

pub struct Period0Status {
    pub timestamp: SystemTime,

    pub duty_cycle: f32,
    pub faults: u64,
    pub sticky_faults: u64,
    pub is_inverted: bool,
    pub idle_mode: IdleMode,
    pub is_follower: bool,
}
pub struct Period1Status {
    pub timestamp: SystemTime,

    pub temperature: f32,
    pub velocity: f32,
    pub voltage: f32,
    pub current: f32,
}
pub struct Period2Status {
    pub timestamp: SystemTime,

    pub iAccum: f32,
    pub position: f32,
}
pub struct Period3Status {
    pub timestamp: SystemTime,

    pub analog_voltage: f32,
    pub analog_velocity: f32,
    pub analog_position: f32,
}
pub struct Period4Status {
    pub timestamp: SystemTime,
}

pub trait CANResponseStatus {
    fn decode(raw_value: u64, timestamp: u32) -> Self;
}
impl CANResponseStatus for Period0Status {
    fn decode(raw_value: u64, timestamp: u32) -> Self {
        // TODO: need to make sure idle mode actually is correct
        Self {
            duty_cycle: ((raw_value & 0xFFFF) as f32) / 32768.0,
            faults: (raw_value >> 16) & 0xFFFF,
            sticky_faults: (raw_value >> 32) & 0xFFFF,
            is_inverted: ((raw_value >> 49) & 1) != 0,
            idle_mode: match (raw_value >> 57) & 1 {
                1 => IdleMode::Brake,
                _ => IdleMode::Coast,
            },
            is_follower: ((raw_value >> 58) & 1) != 0,
            timestamp: std::time::UNIX_EPOCH.add(Duration::from_millis(timestamp as u64)),
        }
    }
}
impl CANResponseStatus for Period1Status {
    fn decode(raw_value: u64, timestamp: u32) -> Self {
        todo!()
    }
}
impl CANResponseStatus for Period2Status {
    fn decode(raw_value: u64, timestamp: u32) -> Self {
        todo!()
    }
}
impl CANResponseStatus for Period3Status {
    fn decode(raw_value: u64, timestamp: u32) -> Self {
        todo!()
    }
}
impl CANResponseStatus for Period4Status {
    fn decode(raw_value: u64, timestamp: u32) -> Self {
        todo!()
    }
}

pub struct CANClient {
    device_id: u8,
    device_type: can_api::CANDeviceType,
    manufacturer: can_api::CANManufacturer,
    session: hal_can::StreamSession,
}

impl CANClient {
    pub fn new(
        device_id: u8,
        device_type: can_api::CANDeviceType,
        manufacturer: can_api::CANManufacturer,
    ) -> Self {
        let arb_id = CANClient::create_arb_id_from_info(
            device_id,
            device_type,
            manufacturer,
            CANCommand::Null,
        );
        Self {
            device_id,
            device_type,
            manufacturer,
            session: hal_can::StreamSession::open(
                arb_id,
                0u32 | 0x3F << 16 | 0x3F << 22 | 0x3F << 28,
                8,
            )
            .unwrap(),
        }
    }

    pub fn create_arb_id(&self, command: CANCommand) -> u32 {
        CANClient::create_arb_id_from_info(
            self.device_id,
            self.device_type,
            self.manufacturer,
            command,
        )
    }

    pub fn send_heartbeat(&self) -> HALResult<()> {
        let arbitration_id = self.create_arb_id(CANCommand::Heartbeat);

        hal_can::send_message(arbitration_id, &[0u8; 8], 2000)?;
        Ok(())
    }

    pub fn set_percent(&self, percent: f32) -> HALResult<()> {
        let percent = percent.clamp(-1.0, 1.0);

        let arbitration_id = self.create_arb_id(CANCommand::DutyCycle);

        let data = percent.to_le_bytes();

        hal_can::send_message(arbitration_id, &data, 2000)?;
        Ok(())
    }

    pub fn set_voltage(&self, voltage: f32) -> HALResult<()> {
        let arbitration_id = self.create_arb_id(CANCommand::Voltage);

        let data = voltage.to_le_bytes();

        hal_can::send_message(arbitration_id, &data, 2000)?;
        Ok(())
    }

    pub fn create_mask(&self) -> u32 {
        // most of these params dont actually matter since its gonna be masked out anyway

        0u32 | 0x3F << 16 | 0x3F << 22 | 0x3F << 28
    }

    pub fn create_arb_id_from_info(
        device_id: u8,
        device_type: can_api::CANDeviceType,
        manufacturer: can_api::CANManufacturer,
        command: CANCommand,
    ) -> u32 {
        let api_class = (command as u32) << 4;
        let api_index = (command as u32) & 0x0F;
        return ((device_type as u32) << 24)
            | ((manufacturer as u32) << 16)
            | api_class << 10
            | api_index << 6
            | device_id as u32;
    }

    pub fn read_frames(&self) -> HALResult<Vec<CANResponse>> {
        let mut messages = [];
        let (_, error) = self.session.read_into(&mut messages);

        if let Some(error) = error {
            return Err(HALError::from(error));
        };

        let mut can_responses: Vec<CANResponse> = Vec::with_capacity(messages.len());

        for message in messages {
            let message_id = message.messageID;
            let mut raw_value = 0u64;
            for i in 0..message.data.len() {
                raw_value |= (message.data[i] as u64) << (8 * i);
            }

            if message_id == self.create_arb_id(CANCommand::Period0) {
                can_responses.push(CANResponse::Period0(Period0Status::decode(
                    raw_value,
                    message.timeStamp,
                )))
            } else if message_id == self.create_arb_id(CANCommand::Period1) {
                can_responses.push(CANResponse::Period1(Period1Status::decode(
                    raw_value,
                    message.timeStamp,
                )))
            } else if message_id == self.create_arb_id(CANCommand::Period2) {
                can_responses.push(CANResponse::Period2(Period2Status::decode(
                    raw_value,
                    message.timeStamp,
                )))
            } else if message_id == self.create_arb_id(CANCommand::Period3) {
                can_responses.push(CANResponse::Period3(Period3Status::decode(
                    raw_value,
                    message.timeStamp,
                )))
            } else if message_id == self.create_arb_id(CANCommand::Period4) {
                can_responses.push(CANResponse::Period4(Period4Status::decode(
                    raw_value,
                    message.timeStamp,
                )))
            } else {
                return Err(HALError::DecodeError);
            }
        }
        Ok(can_responses)
    }
}
