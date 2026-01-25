use confetti::prelude::*;

struct MyRobot {
    drivetrain: DifferentialDrive,
}

impl Robot for MyRobot {
    fn teleop_periodic(&mut self) {
        self.drivetrain.arcade_drive(0.5, 0.25);
    }
}

fn main() -> anyhow::Result<()> {
    let left = MotorGroup::from_motors(vec![SparkMAX::new(0), SparkMAX::new(1)]);
    let right = MotorGroup::from_motors(vec![SparkMAX::new(2), SparkMAX::new(3)]);

    let drivetrain = DifferentialDriveBuilder::default()
        .left_motor(left)
        .right_motor(right)
        .build()
        .unwrap();

    drivetrain.arcade_drive(0.65, 0.2);

    let bot = MyRobot { drivetrain };

    run(bot)
}
