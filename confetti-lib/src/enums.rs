use wpihal::driver_station::ControlWord;

pub enum RobotMode {
    Disabled,
    Autonomous,
    Teleoperated,
    Test,
    EStopped,
}

impl RobotMode {
    pub fn from_control_word(control_word: &ControlWord) -> Self {
        if !control_word.enabled() {
            Self::Disabled
        } else if control_word.autonomous() {
            Self::Autonomous
        } else if control_word.test() {
            Self::Test
        } else if control_word.estop() {
            Self::EStopped
        } else {
            Self::Teleoperated
        }
    }
}
