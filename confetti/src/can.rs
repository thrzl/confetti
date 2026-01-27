use bitvec::{field::BitField, order::Lsb0, view::BitView};
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
pub struct SparkCANSetpoint {
    setpoint: f32,
    arb_feedforward: f32,
    pid_slot: u8,
    ff_units: FeedforwardUnits,
}

#[derive(Clone, Copy)]
pub struct Status0 {
    pub applied_output: f32,
    pub voltage: f32,
    pub current: f32,
    pub motor_temperature: u8,
    pub hard_forward_limit_reached: bool,
    pub hard_reverse_limit_reached: bool,
    pub soft_forward_limit_reached: bool,
    pub soft_reverse_limit_reached: bool,
    pub is_inverted: bool,
}

#[derive(Clone, Copy)]
pub struct Status1 {
    pub other_fault: bool,
    pub motor_type_fault: bool,
    pub sensor_fault: bool,
    pub can_fault: bool,
    pub temperature_fault: bool,
    pub drv_fault: bool,
    pub esc_eeprom_fault: bool,
    pub firmware_fault: bool,
    pub reserved_actives: u8,
    pub brownout_warning: bool,
    pub overcurrent_warning: bool,
    pub esc_eeprom_warning: bool,
    pub ext_eeprom_warning: bool,
    pub sensor_warning: bool,
    pub stall_warning: bool,
    pub has_reset_warning: bool,
    pub other_warning: bool,
    pub other_sticky_fault: bool,
    pub motor_type_sticky_fault: bool,
    pub sensor_sticky_fault: bool,
    pub can_sticky_fault: bool,
    pub temperature_sticky_fault: bool,
    pub drv_sticky_fault: bool,
    pub esc_eeprom_sticky_fault: bool,
    pub firmware_sticky_fault: bool,
    pub brownout_sticky_warning: bool,
    pub overcurrent_sticky_warning: bool,
    pub esc_eeprom_sticky_warning: bool,
    pub ext_eeprom_sticky_warning: bool,
    pub sensor_sticky_warning: bool,
    pub stall_sticky_warning: bool,
    pub has_reset_sticky_warning: bool,
    pub other_sticky_warning: bool,
    pub is_follower: bool,
}
#[derive(Clone, Copy)]
pub struct Status2 {
    pub velocity: f32,
    pub position: f32,
}
#[derive(Clone, Copy)]
pub struct Status3 {
    pub analog_voltage: f32,
    pub analog_velocity: f32,
    pub analog_position: f32,
}
#[derive(Clone, Copy)]
pub struct Status4 {
    pub external_or_alt_encoder_velocity: f32,
    pub external_or_alt_encoder_position: f32,
}
#[derive(Clone, Copy)]
pub struct Status5 {
    pub duty_cycle_encoder_velocity: f32,
    pub duty_cycle_encoder_position: f32,
}
#[derive(Clone, Copy)]
pub struct Status6 {
    pub unadjusted_duty_cycle: f32,
    pub duty_cycle_period: u16,
    pub duty_cycle_no_signal: bool,
    pub duty_cycle_reserved: i32,
}
#[derive(Clone, Copy)]
pub struct Status7 {
    pub i_accumulation: f32,
}
#[derive(Clone, Copy)]
pub struct Status8 {
    pub setpoint: f32,
    pub is_at_setpoint: bool,
    pub selected_pid_slot: u8,
}
#[derive(Clone, Copy)]
pub struct Status9 {
    pub maxmotion_position_setpoint: f32,
    pub maxmotion_velocity_setpoint: f32,
}

// see https://github.com/grayson-arendt/sparkcan/blob/25167e908c9350a0047edc041e0a6420b6b77a76/include/SparkBase.hpp#L54C1-L78C3
#[derive(Clone, Copy)]
pub enum SparkCANFrame {
    Null,

    DutyCycleSetpoint(SparkCANSetpoint),
    VelocitySetpoint(SparkCANSetpoint),
    PositionSetpoint(SparkCANSetpoint),
    MAXMotionVelocitySetpoint(SparkCANSetpoint),
    MAXMotionPositionSetpoint(SparkCANSetpoint),

    VoltageSetpoint(SparkCANSetpoint),
    CurrentSetpoint(SparkCANSetpoint),

    ClearFaults,

    SetIAccumulation { i_accumulation: f32 },

    SetAnalogPosition { position: f32 },

    SetPrimaryEncoderPosition { position: f32 },

    SetExtOrAltEncoderPosition { position: f32 },

    SetDutyCyclePosition { position: f32 },

    StartFollowerMode,
    StopFollowerMode,

    PersistParameters, // need to include the 16-bit MAGIC_NUMBER 15011

    // statuses
    Status0(Status0),

    Status1(Status1),

    Status2(Status2),

    Status3(Status3),

    Status4(Status4),

    Status5(Status5),

    Status6(Status6),

    Status7(Status7),

    Status8(Status8),

    Status9(Status9),
}

impl SparkCANFrame {
    pub fn arb_id(&self, device_id: u32) -> u32 {
        let frame_arb_id = match self {
            Self::VelocitySetpoint { .. } => 0x2050_000,
            Self::DutyCycleSetpoint { .. } => 0x2050_080,
            Self::PositionSetpoint { .. } => 0x2050_100,
            Self::VoltageSetpoint { .. } => 0x2050_140,
            Self::CurrentSetpoint { .. } => 0x2050_180,
            Self::StopFollowerMode => 0x2057_C80,
            Self::ClearFaults => 0x2051_B80,
            _ => unimplemented!("we will never need to get the arb ID of a status"),
        };
        frame_arb_id | (device_id)
    }
    pub fn to_can_bytes(&self) -> [u8; 8] {
        match *self {
            Self::VelocitySetpoint(frame)
            | Self::DutyCycleSetpoint(frame)
            | Self::PositionSetpoint(frame)
            | Self::VoltageSetpoint(frame)
            | Self::CurrentSetpoint(frame) => {
                let (setpoint, arb_feedforward, pid_slot, ff_units) = (
                    frame.setpoint,
                    frame.arb_feedforward,
                    frame.pid_slot,
                    frame.ff_units,
                );
                let mut buf = [0u8; 8]; // 8 byte buffer

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
            Self::ClearFaults | Self::StartFollowerMode | Self::StopFollowerMode => [0u8; 8],
            Self::SetIAccumulation { i_accumulation } => {
                let mut buf = [0u8; 8];
                buf[0..4].copy_from_slice(&i_accumulation.to_le_bytes());
                buf[4..8].copy_from_slice(&3u8.to_le_bytes());
                buf
            }
            Self::SetAnalogPosition { position }
            | Self::SetDutyCyclePosition { position }
            | Self::SetPrimaryEncoderPosition { position }
            | Self::SetExtOrAltEncoderPosition { position } => {
                let mut buf = [0u8; 8];
                buf[0..4].copy_from_slice(&position.to_le_bytes());
                buf[4..6].copy_from_slice(&3u8.to_le_bytes()); // magic number DATA_TYPE
                buf
            }
            Self::PersistParameters => {
                let mut buf = [0u8; 8];
                buf[0..2].copy_from_slice(&15011u16.to_le_bytes());
                buf
            }
            _ => unimplemented!("we will never need to convert statuses to CAN bytes"),
        }
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

    pub fn send_heartbeat() -> HALResult<()> {
        hal_can::send_message(0x2052_C80, &[1u8; 8], 2000)?;
        Ok(())
    }

    pub fn duty_cycle_setpoint(
        &self,
        percent: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::DutyCycleSetpoint(SparkCANSetpoint {
            setpoint: percent.clamp(-1.0, 1.0),
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn voltage_setpoint(
        &self,
        voltage: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::VoltageSetpoint(SparkCANSetpoint {
            setpoint: voltage,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn velocity_setpoint(
        &self,
        velocity: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::VelocitySetpoint(SparkCANSetpoint {
            setpoint: velocity,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn position_setpoint(
        &self,
        position: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::PositionSetpoint(SparkCANSetpoint {
            setpoint: position,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn max_motion_velocity_setpoin(
        &self,
        velocity: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::MAXMotionVelocitySetpoint(SparkCANSetpoint {
            setpoint: velocity,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn max_motion_position_setpoint(
        &self,
        position: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::MAXMotionPositionSetpoint(SparkCANSetpoint {
            setpoint: position,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn current_setpoint(
        &self,
        current: f32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> HALResult<()> {
        self.send_frame(SparkCANFrame::CurrentSetpoint(SparkCANSetpoint {
            setpoint: current,
            arb_feedforward: feedforward,
            pid_slot,
            ff_units: feedforward_units,
        }))
    }

    pub fn set_i_accumulation(&self, i_accumulation: f32) -> HALResult<()> {
        let frame = SparkCANFrame::SetIAccumulation { i_accumulation };
        self.send_frame(frame)
    }

    pub fn set_primary_encoder_position(&self, position: f32) -> HALResult<()> {
        self.send_frame(SparkCANFrame::SetPrimaryEncoderPosition { position })
    }

    pub fn set_analog_position(&self, position: f32) -> HALResult<()> {
        self.send_frame(SparkCANFrame::SetAnalogPosition { position })
    }

    pub fn set_ext_or_alt_encoder_position(&self, position: f32) -> HALResult<()> {
        self.send_frame(SparkCANFrame::SetExtOrAltEncoderPosition { position })
    }

    pub fn set_duty_cycle_position(&self, position: f32) -> HALResult<()> {
        self.send_frame(SparkCANFrame::SetDutyCyclePosition { position })
    }

    pub fn start_follower_mode(&self) -> HALResult<()> {
        self.send_frame(SparkCANFrame::StartFollowerMode)
    }

    pub fn stop_follower_mode(&self) -> HALResult<()> {
        self.send_frame(SparkCANFrame::StopFollowerMode)
    }

    pub fn persist_parameters(&self) -> HALResult<()> {
        self.send_frame(SparkCANFrame::PersistParameters)
    }

    pub fn clear_faults(&self) -> HALResult<()> {
        self.send_frame(SparkCANFrame::ClearFaults)
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
                0x205B_800 => SparkCANFrame::Status0(Status0 {
                    applied_output: (sign_extend(bits[0..16].load_le::<i32>(), 16) as f32)
                        * 0.00003082369457075716,
                    voltage: (bits[16..28].load_le::<u16>()) as f32 * 0.0073260073260073,
                    current: (bits[28..40].load_le::<u16>()) as f32 * 0.0366300366300366,
                    motor_temperature: u8::from_le_bytes([data[5]]),
                    hard_forward_limit_reached: bits[48],
                    soft_forward_limit_reached: bits[49],
                    hard_reverse_limit_reached: bits[50],
                    soft_reverse_limit_reached: bits[51],
                    is_inverted: bits[52],
                }),
                0x205B_840 => SparkCANFrame::Status1(Status1 {
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
                }),
                0x205B_880 => SparkCANFrame::Status2(Status2 {
                    velocity: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                    position: f32::from_le_bytes([data[4], data[5], data[6], data[7]]),
                }),
                0x205B_8C0 => SparkCANFrame::Status3(Status3 {
                    analog_voltage: (bits[0..10].load_le::<u16>() as f32) * 0.0048973607038123,
                    analog_velocity: (sign_extend(bits[10..32].load_le::<i32>(), 22) as f32)
                        * 0.007812026887906498,
                    analog_position: bits[32..64].load_le::<u32>() as f32,
                }),
                0x205B_900 => SparkCANFrame::Status4(Status4 {
                    external_or_alt_encoder_velocity: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    external_or_alt_encoder_position: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                }),
                0x205B_940 => SparkCANFrame::Status5(Status5 {
                    duty_cycle_encoder_velocity: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    duty_cycle_encoder_position: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                }),
                0x205B_980 => SparkCANFrame::Status6(Status6 {
                    unadjusted_duty_cycle: (bits[0..16].load_le::<u16>() as f32)
                        * 0.00001541161211566339,
                    duty_cycle_period: bits[16..32].load_le::<u16>(),
                    duty_cycle_no_signal: bits[32],
                    duty_cycle_reserved: sign_extend(bits[33..64].load_le::<i32>(), 31),
                }),
                0x205B_9C0 => SparkCANFrame::Status7(Status7 {
                    i_accumulation: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                }),
                0x205B_A00 => SparkCANFrame::Status8(Status8 {
                    setpoint: f32::from_le_bytes([data[0], data[1], data[2], data[3]]),
                    is_at_setpoint: bits[32],
                    selected_pid_slot: bits[33..37].load_le::<u8>(),
                }),
                0x205B_A40 => SparkCANFrame::Status9(Status9 {
                    maxmotion_position_setpoint: f32::from_le_bytes([
                        data[0], data[1], data[2], data[3],
                    ]),
                    maxmotion_velocity_setpoint: f32::from_le_bytes([
                        data[4], data[5], data[6], data[7],
                    ]),
                }),
                _ => continue,
            };
            can_responses.push(frame);
        }
        Ok(can_responses)
    }
}
