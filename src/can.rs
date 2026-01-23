pub enum CANCommands {
    DutyCycle = 0x02,
}

struct CANFrame {
    /// the arbitration id.
    ///
    /// layout is roughly:
    /// `device_id || message_type || api_class || api_index || flags`
    id: u32,

    /// the data being transmitted, as bytes
    data: [u8; 8],
    len: usize,
}

impl CANFrame {
    pub fn set_percent(device_id: u8, percent: f32) {
        let percent = percent.clamp(-1.0, 1.0);

        let arbitration_id = ((device_id as u32) << 5) | CANCommands::DutyCycle as u32;

        let data = percent.to_le_bytes();

        wpihal::can::send_message(arbitration_id, &data, 100);
    }
}
