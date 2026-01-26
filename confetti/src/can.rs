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

#[repr(u32)]
pub enum ApiClass {
    Setpoint = 0,
    Control = 1,
    Status = 46,
}

#[derive(Clone, Copy)]
struct LimitStatuses {
    pub hard_forward_limit_reached: bool,
    pub hard_reverse_limit_reached: bool,
    pub soft_forward_limit_reached: bool,
    pub soft_reverse_limit_reached: bool,
}

impl LimitStatuses {
    pub fn from_byte(b: u8) -> Self {
        Self {
            hard_forward_limit_reached: b & (1 << 0) != 0,
            hard_reverse_limit_reached: b & (1 << 1) != 0,
            soft_forward_limit_reached: b & (1 << 2) != 0,
            soft_reverse_limit_reached: b & (1 << 3) != 0,
        }
    }
}

// see https://github.com/grayson-arendt/sparkcan/blob/25167e908c9350a0047edc041e0a6420b6b77a76/include/SparkBase.hpp#L54C1-L78C3
#[derive(Clone, Copy)]
pub enum SparkCANFrame {
    Null,

    Heartbeat,

    DutyCycle {
        setpoint: f32,
        arb_feedforward: i16,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Velocity {
        setpoint: f32,
        arb_feedforward: i16,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Position {
        setpoint: f32,
        arb_feedforward: i16,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },

    Voltage {
        setpoint: f32,
        arb_feedforward: i16,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Current {
        setpoint: f32,
        arb_feedforward: i16,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },

    // statuses
    Status0 {
        applied_output: f32,
        voltage: f32,
        current: f32,
        motor_temperature: u8,
        limits: LimitStatuses,
        is_inverted: bool,
    },

    Status2 {
        velocity: f32,
        position: f32,
    },
}

impl SparkCANFrame {
    pub fn arb_id(&self, device_id: u32) -> u32 {
        let frame_arb_id = match self {
            Self::Velocity { .. } => 0x2050_000,
            Self::DutyCycle { .. } => 0x2050_080,
            Self::Position { .. } => 0x2050_100,
            Self::Voltage { .. } => 0x2050_140,
            Self::Current { .. } => 0x2050_180,
            Self::Heartbeat => 0xB2,
            _ => unimplemented!("we will never need to get the arb ID of a status"),
        };
        frame_arb_id | (device_id << 6)
    }
    pub fn to_can_bytes(&self) -> [u8; 8] {
        let (sp, ff, slot, units) = match *self {
            Self::Velocity {
                setpoint,
                arb_feedforward,
                pid_slot,
                ff_units,
            }
            | Self::DutyCycle {
                setpoint,
                arb_feedforward,
                pid_slot,
                ff_units,
            }
            | Self::Position {
                setpoint,
                arb_feedforward,
                pid_slot,
                ff_units,
            }
            | Self::Voltage {
                setpoint,
                arb_feedforward,
                pid_slot,
                ff_units,
            }
            | Self::Current {
                setpoint,
                arb_feedforward,
                pid_slot,
                ff_units,
            } => (setpoint, arb_feedforward, pid_slot & 0x03, ff_units),
            _ => unimplemented!("we will never need to convert statuses to CAN bytes"),
        };

        let mut data = [0u8; 8];

        data[0..4].copy_from_slice(&sp.to_le_bytes());
        data[4..6].copy_from_slice(&ff.to_le_bytes());
        data[6] = slot | ((units as u8) << 2);
        data
    }

    pub fn heartbeat(device_id: u32) -> u32 {
        Self::Heartbeat.arb_id(device_id)
    }
}

#[derive(Clone, Copy)]
pub enum FeedforwardUnits {
    Voltage = 0,
    DutyCycle = 1,
}

pub enum IdleMode {
    Brake,
    Coast,
}

pub struct CANClient {
    device_id: u32,
    session: hal_can::StreamSession,
}

impl CANClient {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            session: hal_can::StreamSession::open(device_id, 0x3F, 64).unwrap(),
        }
    }

    pub fn send_heartbeat(&self) -> HALResult<()> {
        let arbitration_id = SparkCANFrame::heartbeat(self.device_id);

        hal_can::send_message(arbitration_id, &[0u8; 8], 2000)?;
        Ok(())
    }

    pub fn set_percent(&self, percent: f32) -> HALResult<()> {
        let percent = percent.clamp(-1.0, 1.0);

        let arbitration_id = SparkCANFrame::DutyCycle {
            setpoint: percent,
            arb_feedforward: 0,
            pid_slot: 0,
            ff_units: FeedforwardUnits::DutyCycle,
        }
        .arb_id(self.device_id);

        let data = percent.to_le_bytes();

        hal_can::send_message(arbitration_id, &data, 2000)?;
        Ok(())
    }

    pub fn set_voltage(&self, voltage: f32) -> HALResult<()> {
        let arbitration_id = SparkCANFrame::Voltage {
            setpoint: voltage,
            arb_feedforward: 0,
            pid_slot: 0,
            ff_units: FeedforwardUnits::Voltage,
        }
        .arb_id(self.device_id);

        let data = voltage.to_le_bytes();

        hal_can::send_message(arbitration_id, &data, 2000)?;
        Ok(())
    }

    pub fn create_mask(&self) -> u32 {
        // most of these params dont actually matter since its gonna be masked out anyway

        0u32 | 0x3F << 16 | 0x3F << 22 | 0x3F << 28
    }

    pub fn create_arb_id_from_info(device_id: u32, frame: SparkCANFrame) -> u32 {
        frame.arb_id(device_id) | device_id << 6 // this needs to be double checked
    }

    pub fn read_frames(&self) -> HALResult<Vec<SparkCANFrame>> {
        let mut messages = [CANStreamMessage::default(); 32];
        let (_, error) = self.session.read_into(&mut messages[..32]);

        if let Some(error) = error {
            return Err(HALError::from(error));
        };

        let mut can_responses: Vec<SparkCANFrame> = Vec::with_capacity(messages.len());

        for message in messages {
            let message_id = message.messageID;

            let base_id = message_id & !0x3F;
            let _device_id = message_id & 0x3F; // just in case i need it later

            let data = message.data;
            let frame = match base_id {
                0x205B_800 => SparkCANFrame::Status0 {
                    applied_output: (u16::from_le_bytes([data[0], data[1]]) as f32)
                        * 0.00003082369457075716,
                    voltage: (u16::from_le_bytes([data[2], data[3]]) & 0x0FFF) as f32
                        * 0.0073260073260073,
                    current: (u16::from_le_bytes([data[3], data[4]]) & 0x0FFF) as f32
                        * 0.0366300366300366,
                    motor_temperature: u8::from_le_bytes([data[5]]),
                    limits: LimitStatuses::from_byte(data[6]),
                    is_inverted: data[6] & (1 << 4) != 0,
                },
                0x205B_880 => SparkCANFrame::Status2 {
                    velocity: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                    position: f32::from_le_bytes([data[4], data[5], data[6], data[7]]),
                },
                _ => continue,
            };
            can_responses.push(frame);
        }
        Ok(can_responses)
    }
}
