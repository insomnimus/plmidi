use std::time::Duration;

use futures::{
	channel::mpsc::Receiver,
	prelude::*,
};
use nodi::Timer;

pub struct Pausable {
	ticks_per_beat: u16,
	micros_per_tick: f64,
	pub speed: f32,
	pub pause: Receiver<()>,
}

impl Pausable {
	pub fn new(ticks_per_beat: u16, pause: Receiver<()>) -> Self {
		Self {
			ticks_per_beat,
			micros_per_tick: 0.0,
			speed: 1.0,
			pause,
		}
	}
}

impl Timer for Pausable {
	fn change_tempo(&mut self, tempo: u32) {
		let micros_per_tick = tempo as f64 / self.ticks_per_beat as f64;
		self.micros_per_tick = micros_per_tick;
	}

	fn sleep_duration(&mut self, n_ticks: u32) -> Duration {
		let t = self.micros_per_tick * n_ticks as f64 / self.speed as f64;
		if t > 0.0 {
			Duration::from_micros(t as u64)
		} else {
			Duration::default()
		}
	}

	fn sleep(&mut self, n_ticks: u32) {
		// Check if we're supposed to be paused.
		if self.pause.try_next().is_ok() {
			futures::executor::block_on(self.pause.next()).unwrap();
		}

		let t = self.sleep_duration(n_ticks);

		if !t.is_zero() {
			nodi::timers::sleep(t);
		}
	}
}
