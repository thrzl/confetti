use parking_lot::Mutex;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};

use crate::can::CANClient;

pub trait Motor: Send {
    /// check if the watchdog timeout has been exceeded. if it has,
    /// this function should stop the motor immediately.
    fn check_timeout(&mut self);

    /// run a motor at a percentage of its capacity.
    fn set_percent(&mut self, percentage: f32);

    /// run a motor at a set voltage.
    fn set_voltage(&mut self, volts: f32);

    /// stop the motor.
    fn stop(&mut self);

    fn periodic(&mut self) {}
}

/// carries a motor that can be obtained via `.lock()`.
///
/// really an `Arc<parking_lot::Mutex<Motor>>` under the hood.
pub type MotorGuard<M> = Arc<Mutex<M>>;

/// group together some motors to control them all at once. implements `Motor`
pub struct MotorGroup {
    motors: Vec<MotorGuard<dyn Motor>>,
}

impl MotorGroup {
    pub fn from_motors(motors: Vec<MotorGuard<dyn Motor>>) -> MotorGuard<Self> {
        MotorGuard::new(Mutex::new(Self { motors }))
    }
}

impl Motor for MotorGroup {
    fn check_timeout(&mut self) {
        let _ = self
            .motors
            .iter_mut()
            .map(|motor| motor.lock().check_timeout());
    }

    fn set_percent(&mut self, percentage: f32) {
        let _ = self
            .motors
            .iter_mut()
            .map(|motor| motor.lock().set_percent(percentage));
    }

    fn set_voltage(&mut self, volts: f32) {
        let _ = self
            .motors
            .iter_mut()
            .map(|motor| motor.lock().set_voltage(volts));
    }

    fn stop(&mut self) {
        let _ = self.motors.iter_mut().map(|motor| motor.lock().stop());
    }
}

/// stores motors and essentially "ticks" them every loop.
///
/// ticked motors that have not received updates in too long should immediately stop.
pub(crate) struct MotorWatchdog {
    motors: Vec<Weak<Mutex<dyn Motor + Send>>>,
}

impl MotorWatchdog {
    /// initialize a new, empty motor watchdog.
    pub fn new() -> Self {
        return MotorWatchdog { motors: Vec::new() };
    }

    /// check all stored motors. automatically cleans up dropped motors.
    pub fn motor_periodic(&mut self) {
        self.motors.retain(|item: &Weak<Mutex<dyn Motor + Send>>| {
            if let Some(motor) = item.upgrade() {
                let mut motor = motor.lock();
                motor.check_timeout();
                motor.periodic();
                true
            } else {
                false
            }
        });
    }

    /// add a new motor from a weak reference. automatically run when a motor is initialized.
    fn add_motor(&mut self, motor: Weak<Mutex<dyn Motor + Send>>) {
        self.motors.push(motor)
    }
}

/// the global motor watchdog. ensures that motors receive updates and automatically stop them if they do not.
pub(crate) static MOTOR_REGISTRY: LazyLock<Mutex<MotorWatchdog>> =
    LazyLock::new(|| Mutex::new(MotorWatchdog::new()));
const WATCHDOG_TIMEOUT: Duration = Duration::from_millis(100);

pub struct SparkMAX {
    watchdog_time: Instant,
    can: CANClient,
}

impl Motor for SparkMAX {
    fn check_timeout(&mut self) {
        if self.watchdog_time.elapsed() > WATCHDOG_TIMEOUT {
            self.stop();
        }
    }

    fn set_percent(&mut self, percentage: f32) {
        let _ = percentage;
        self.can
            .set_percent(percentage.clamp(-1.0, 1.0))
            .expect("failed to set motor percent")
    }

    fn set_voltage(&mut self, volts: f32) {
        let _ = volts;
        self.can
            .set_voltage(volts)
            .expect("failed to set motor voltage")
    }

    fn stop(&mut self) {
        self.set_percent(0.0)
    }

    fn periodic(&mut self) {
        let _messages = self.can.read_frames().unwrap_or_else(|_| vec![]);
        // edit inner status with the new info...

        // TODO: need to handle errors here better
        let _ = self.can.send_heartbeat();
    }
}

impl SparkMAX {
    /// initialize a new REV SPARK MAX motor controller. will attempt to send a heartbeat and error if it fails.
    pub fn new(port: u32) -> anyhow::Result<MotorGuard<Self>> {
        let can_client = CANClient::new(port);
        can_client.send_heartbeat()?;
        let motor = MotorGuard::new(Mutex::new(Self {
            watchdog_time: Instant::now(),
            can: can_client,
        }));
        let trait_motor: Arc<Mutex<dyn Motor + Send>> = motor.clone();
        MOTOR_REGISTRY
            .lock()
            .add_motor(Arc::downgrade(&trait_motor));
        Ok(motor)
    }
}
