use parking_lot::Mutex;
use std::sync::{Arc, LazyLock, Weak};
use std::time::{Duration, Instant};

use crate::can::{CANClient, FeedforwardUnits, SparkCANFrame, Status0};

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

        // TODO: need to handle errors here better
        // we're sending this last though because
        // if it's delayed that means that we lack
        // proper control over the motors. they should
        // break in this situation
        let _ = CANClient::send_heartbeat();
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
    status0: Option<Status0>,
    pid_slot: u8,
    feedforward: f32,
    feedforward_units: FeedforwardUnits,
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
            .duty_cycle_setpoint(
                percentage.clamp(-1.0, 1.0),
                self.pid_slot,
                self.feedforward,
                self.feedforward_units,
            )
            .expect("failed to set motor percent")
    }

    fn set_voltage(&mut self, volts: f32) {
        let _ = volts;
        self.can
            .voltage_setpoint(
                volts,
                self.pid_slot,
                self.feedforward,
                self.feedforward_units,
            )
            .expect("failed to set motor voltage")
    }

    fn stop(&mut self) {
        self.set_percent(0.0)
    }

    fn periodic(&mut self) {
        let _messages = self.can.read_frames().unwrap_or_else(|_| vec![]);
        // edit inner status with the new info...
        //
        for message in _messages {
            match message {
                SparkCANFrame::Status0(status) => self.status0 = Some(status),
                _ => todo!(),
            }
        }
    }
}

impl SparkMAX {
    /// initialize a new REV SPARK MAX motor controller. will attempt to send a heartbeat and error if it fails.
    pub fn new(
        port: u32,
        pid_slot: u8,
        feedforward: f32,
        feedforward_units: FeedforwardUnits,
    ) -> anyhow::Result<MotorGuard<Self>> {
        let can_client = CANClient::new(port);
        // i should replace this with GET_MOTOR_INTERFACE or something later
        can_client.duty_cycle_setpoint(0.0, pid_slot, 0.0, FeedforwardUnits::DutyCycle)?;
        let motor = MotorGuard::new(Mutex::new(Self {
            watchdog_time: Instant::now(),
            can: can_client,
            status0: None,
            pid_slot,
            feedforward,
            feedforward_units,
        }));
        let trait_motor: Arc<Mutex<dyn Motor + Send>> = motor.clone();
        MOTOR_REGISTRY
            .lock()
            .add_motor(Arc::downgrade(&trait_motor));
        Ok(motor)
    }

    pub fn get_applied_output(&self) -> Option<f32> {
        self.status0.map(|status| status.applied_output)
    }

    pub fn get_voltage(&self) -> Option<f32> {
        self.status0.map(|status| status.voltage)
    }

    pub fn get_current(&self) -> Option<f32> {
        self.status0.map(|status| status.current)
    }

    pub fn get_motor_temperature(&self) -> Option<u8> {
        self.status0.map(|status| status.motor_temperature)
    }

    pub fn get_inverted(&self) -> Option<bool> {
        self.status0.map(|status| status.is_inverted)
    }

    pub fn set_pid_slot(&mut self, pid_slot: u8) {
        self.pid_slot = pid_slot
    }
}
