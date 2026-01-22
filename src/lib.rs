use crate::{enums::RobotMode, robot::Robot};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};
use wpihal::initialize_common as initialize_hal;

pub mod enums;
pub mod errors;
pub mod robot;

const LOOP_PERIOD: Duration = Duration::from_millis(20);

pub fn run(mut bot: impl Robot) -> ! {
    initialize_hal();

    loop {
        let loop_time = Instant::now();
        let control_word = wpihal::driver_station::get_control_word()
            .expect("failed to get control word from driverstation");
        match RobotMode::from_control_word(&control_word) {
            RobotMode::Disabled => bot.disabled_periodic(),
            RobotMode::Autonomous => bot.autonomous_periodic(),
            RobotMode::Teleoperated => bot.teleop_periodic(),
            RobotMode::Test => bot.test_periodic(),
            RobotMode::EStopped => (), // dont do nothing here
        }
        let elapsed = loop_time.elapsed();
        if elapsed > LOOP_PERIOD {
            println!("loop overrun") // do something better with this later
        } else {
            sleep(LOOP_PERIOD.saturating_sub(elapsed))
        }
    }
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
