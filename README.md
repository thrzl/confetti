# confetti

a framework for building FRC robots in Rust

built on [guineawheek/wpihal-rs](https://github.com/guineawheek/wpihal-rs).

## contributing

help is more than welcome! even if you're not sure what you can contribute, please let me know or send me an email (my email is on my profile).

if you're familiar with how some of the low-level stuff works, please reach out! i'm learning this as i go, so that would be extremely helpful for me

even if you just have complaints about the API, i'd love to hear em. i'm far from a rust professional

## prerequisites

1. you need to have [WPILib](https://docs.wpilib.org/en/stable/docs/zero-to-robot/step-2/wpilib-setup.html) installed. choose "everything" (NOT "tools only").
2. download the proper ARM FRC toolchain for your platform from [wpilibsuite/opensdk](https://github.com/wpilibsuite/opensdk/releases). you're looking for something like "`cortexa9_vfpv3-roborio-academic-2025-YOUR-SYSTEM-TRIPLE-Toolchain-12.1.0.tgz`". as of right now, the v2025-2 toolchain is the latest and does compile.
3. set the `arm-frc202X-linux-gnueabi-gcc` as the linker for the `arm-unknown-linux-gnueabi` target in `~/.cargo/config.toml` file. for example, on my computer:
```toml
[target.arm-unknown-linux-gnueabi]
linker = "/Users/te/roborio-toolchain/bin/arm-frc2025-linux-gnueabi-gcc"
```
4. now, when you build, ensure that you use `--target arm-unknown-linux-gnueabi`, e.g. `cargo build --release --target arm-unknown-linux-gnueabi`


## example
```rust
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

```

## goals

### WILL NOT abide by the WPILIB API

there are a lot of things they do that would be bizarre at best to do in Rust (from my understanding). i also don't like it that much anyway

### WILL go for feature parity

unfortunately, my team has limited time, people, and resources, so this is largely a personal project and there is very limited hardware available for me to test. implementing hardware is something that i will likely have to rely on others for

### WILL go for ease-of-use

i want this library to be approachable for beginner Rust programmers, so i will try hard to make things make sense.

## roadmap
- [x] get robot loops to work*
- [ ] implement revlib
- [ ] implement command-based style framework
- [ ] implement wpimath (sigh)
- [ ] get CLI in order
  - [x] deploy*
  - [ ] project init

*untested. my team is not particularly rich in money, resources, or time, so getting a hold of a robot that i can test on is difficult, to say the least
