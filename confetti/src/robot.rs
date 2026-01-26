use std::time::Duration;

use crate::enums;

pub struct RobotContext {
    pub mode: enums::RobotMode, // just something random for now as a placeholder
}

/// the main robot type. you should start by implementing this trait for your own class, like this:
/// ```
/// struct MyRobot {}
///
/// impl Robot for MyRobot {
///     fn autonomous_periodic(&self) {
///         println!("started autonomous")
///     }
/// }
///
/// ```
pub trait Robot {
    /// code to run to get the robot ready for teleoperation.
    fn teleop_init(&mut self) {}

    /// periodic function for the teleoperated period
    fn teleop_periodic(&mut self) {}

    /// runs on every loop (every 20ms) when periodic is enabled
    fn teleop_exit(&mut self) {}

    fn disabled_periodic(&mut self) {}
    fn autonomous_periodic(&mut self) {}
    fn test_periodic(&mut self) {}
}
