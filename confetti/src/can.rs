use bitvec::{bitvec, field::BitField, order::Lsb0, view::BitView};
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
        arb_feedforward: f32,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Velocity {
        setpoint: f32,
        arb_feedforward: f32,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Position {
        setpoint: f32,
        arb_feedforward: f32,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },

    Voltage {
        setpoint: f32,
        arb_feedforward: f32,
        pid_slot: u8,
        ff_units: FeedforwardUnits,
    },
    Current {
        setpoint: f32,
        arb_feedforward: f32,
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

    Status1 {
        other_fault: bool,
        motor_type_fault: bool,
        sensor_fault: bool,
        can_fault: bool,
        temperature_fault: bool,
        drv_fault: bool,
        esc_eeprom_fault: bool,
        firmware_fault: bool,
        reserved_actives: u8,
        brownout_warning: bool,
        overcurrent_warning: bool,
        esc_eeprom_warning: bool,
        ext_eeprom_warning: bool,
        sensor_warning: bool,
        stall_warning: bool,
        has_reset_warning: bool,
        other_warning: bool,
        other_sticky_fault: bool,
        motor_type_sticky_fault: bool,
        sensor_sticky_fault: bool,
        can_sticky_fault: bool,
        temperature_sticky_fault: bool,
        drv_sticky_fault: bool,
        esc_eeprom_sticky_fault: bool,
        firmware_sticky_fault: bool,
        brownout_sticky_warning: bool,
        overcurrent_sticky_warning: bool,
        esc_eeprom_sticky_warning: bool,
        ext_eeprom_sticky_warning: bool,
        sensor_sticky_warning: bool,
        stall_sticky_warning: bool,
        has_reset_sticky_warning: bool,
        other_sticky_warning: bool,
        is_follower: bool,
    },

    Status2 {
        velocity: f32,
        position: f32,
    },

    Status3 {
        analog_voltage: f32,
        analog_velocity: f32,
        analog_position: f32,
    },

    Status4 {
        external_or_alt_encoder_velocity: f32,
        external_or_alt_encoder_position: f32,
    },

    Status5 {
        duty_cycle_encoder_velocity: f32,
        duty_cycle_encoder_position: f32,
    },

    Status6 {
        unadjusted_duty_cycle: f32,
        duty_cycle_period: u16,
        duty_cycle_no_signal: bool,
        duty_cycle_reserved: i32,
    },

    Status7 {
        i_accumulation: f32,
    },

    Status8 {
        setpoint: f32,
        is_at_setpoint: bool,
        selected_pid_slot: u8,
    },

    Status9 {
        maxmotion_position_setpoint: f32,
        maxmotion_velocity_setpoint: f32,
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
        frame_arb_id | (device_id)
    }
    pub fn to_can_bytes(&self) -> [u8; 8] {
        let (setpoint, arb_feedforward, pid_slot, ff_units) = match *self {
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
            } => (setpoint, (arb_feedforward), pid_slot, ff_units),
            _ => unimplemented!("we will never need to convert statuses to CAN bytes"),
        };

        let mut buf = [0u8; 8]; // 8-bit buffer

        // voltage setpoint
        buf[0..4].copy_from_slice(&setpoint.to_le_bytes());

        // arbitrary feedforward
        buf[4..8].copy_from_slice(
            &(arb_feedforward.clamp(-32768.0, 32767.0) / 0.0009765923).to_le_bytes(),
        );

        let bits = buf.view_bits_mut::<Lsb0>();
        // pid slot
        bits[48..50].store_le(pid_slot);

        // arbitrary feedforward units
        bits.set(50, ff_units as u8 == 1);
        buf
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

fn sign_extend(raw_value: i32, bits: u16) -> i32 {
    let shift = 32 - bits;
    (raw_value << shift) >> shift
}

impl CANClient {
    pub fn new(device_id: u32) -> Self {
        Self {
            device_id,
            session: hal_can::StreamSession::open(device_id, 0x3F, 64).unwrap(),
        }
    }

    #[inline]
    fn send_frame(&self, frame: SparkCANFrame) -> HALResult<()> {
        hal_can::send_message(frame.arb_id(self.device_id), &frame.to_can_bytes(), 2000)?;
        Ok(())
    }

    pub fn send_heartbeat(&self) -> HALResult<()> {
        let arbitration_id = SparkCANFrame::heartbeat(self.device_id);

        hal_can::send_message(arbitration_id, &[0u8; 8], 2000)?;
        Ok(())
    }

    pub fn set_percent(
        &self,
        percent: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        let percent = percent.clamp(-1.0, 1.0);

        let frame = SparkCANFrame::DutyCycle {
            setpoint: percent,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        };

        self.send_frame(frame)
    }

    pub fn set_voltage(
        &self,
        voltage: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        let frame = SparkCANFrame::Voltage {
            setpoint: voltage,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        };

        self.send_frame(frame)
    }

    pub fn set_velocity(
        &self,
        velocity: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        let frame = SparkCANFrame::Velocity {
            setpoint: velocity,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        };

        self.send_frame(frame)
    }

    pub fn create_mask(&self) -> u32 {
        // most of these params dont actually matter since its gonna be masked out anyway

        0u32 | 0x3F << 16 | 0x3F << 22 | 0x3F << 28
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
            let bits = data.view_bits::<Lsb0>();
            let frame = match base_id {
                0x205B_800 => SparkCANFrame::Status0 {
                    applied_output: (sign_extend(bits[0..16].load_le::<i32>(), 16) as f32)
                        * 0.00003082369457075716,
                    voltage: (bits[16..28].load_le::<u16>()) as f32 * 0.0073260073260073,
                    current: (bits[28..40].load_le::<u16>()) as f32 * 0.0366300366300366,
                    motor_temperature: u8::from_le_bytes([data[5]]),
                    limits: LimitStatuses::from_byte(data[6]),
                    is_inverted: data[6] & (1 << 4) != 0,
                },
                0x205B_840 => SparkCANFrame::Status1 {
                    other_fault: bits[0],
                    motor_type_fault: bits[1],
                    sensor_fault: bits[2],
                    can_fault: bits[3],
                    temperature_fault: bits[4],
                    drv_fault: bits[5],
                    esc_eeprom_fault: bits[6],
                    firmware_fault: bits[7],
                    reserved_actives: bits[8..16].load_le::<u8>(),
                    brownout_warning: bits[16],
                    overcurrent_warning: bits[17],
                    esc_eeprom_warning: bits[18],
                    ext_eeprom_warning: bits[19],
                    sensor_warning: bits[20],
                    stall_warning: bits[21],
                    has_reset_warning: bits[22],
                    other_warning: bits[23],
                    other_sticky_fault: bits[24],
                    motor_type_sticky_fault: bits[25],
                    sensor_sticky_fault: bits[26],
                    can_sticky_fault: bits[27],
                    temperature_sticky_fault: bits[28],
                    drv_sticky_fault: bits[29],
                    esc_eeprom_sticky_fault: bits[30],
                    firmware_sticky_fault: bits[31],
                    // bits 32-39 are reserved for future use
                    brownout_sticky_warning: bits[40],
                    overcurrent_sticky_warning: bits[41],
                    esc_eeprom_sticky_warning: bits[42],
                    ext_eeprom_sticky_warning: bits[43],
                    sensor_sticky_warning: bits[44],
                    stall_sticky_warning: bits[45],
                    has_reset_sticky_warning: bits[46],
                    other_sticky_warning: bits[47],
                    is_follower: bits[48],
                },
                0x205B_880 => SparkCANFrame::Status2 {
                    velocity: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                    position: f32::from_le_bytes([data[4], data[5], data[6], data[7]]),
                },
                0x205B_8C0 => SparkCANFrame::Status3 {
                    analog_voltage: (bits[0..10].load_le::<u16>() as f32) * 0.0048973607038123,
                    analog_velocity: (sign_extend(bits[10..32].load_le::<i32>(), 22) as f32)
                        * 0.007812026887906498,
                    analog_position: bits[32..64].load_le::<u32>() as f32,
                },
                0x205B_900 => SparkCANFrame::Status4 {
                    external_or_alt_encoder_velocity: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    external_or_alt_encoder_position: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                },
                0x205B_940 => SparkCANFrame::Status5 {
                    duty_cycle_encoder_velocity: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    duty_cycle_encoder_position: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                },
                0x205B_980 => SparkCANFrame::Status6 {
                    unadjusted_duty_cycle: (bits[0..16].load_le::<u16>() as f32)
                        * 0.00001541161211566339,
                    duty_cycle_period: bits[16..32].load_le::<u16>(),
                    duty_cycle_no_signal: bits[32],
                    duty_cycle_reserved: sign_extend(bits[33..64].load_le::<i32>(), 31),
                },
                0x205B_9C0 => SparkCANFrame::Status7 {
                    i_accumulation: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                },
                0x205B_A00 => SparkCANFrame::Status8 {
                    setpoint: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                    is_at_setpoint: bits[32],
                    selected_pid_slot: bits[33..37].load_le::<u8>(),
                },
                0x205B_A40 => SparkCANFrame::Status9 {
                    maxmotion_position_setpoint: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    maxmotion_velocity_setpoint: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                },
                _ => continue,
            };
            can_responses.push(frame);
        }
        Ok(can_responses)
    }
}
