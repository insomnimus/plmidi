use std::{
	path::Path,
	sync::Arc,
};

use anyhow::{
	anyhow,
	Context,
	Result,
};
use cpal::{
	traits::{
		DeviceTrait,
		HostTrait,
		StreamTrait,
	},
	OutputCallbackInfo,
	SampleFormat,
	Stream,
};
use fluidlite::{
	Settings,
	Synth,
};
use log::error;
use nodi::{
	Connection,
	MidiEvent,
};
use parking_lot::Mutex;

pub struct Fluid {
	synth: Arc<Mutex<Synth>>,
	_stream: Stream,
}

impl Fluid {
	pub fn new<P: AsRef<Path>>(sf: P) -> Result<Self> {
		let fl = Synth::new(Settings::new()?)?;
		fl.sfload(sf.as_ref(), true)
			.context("failed to load soundfont")?;

		// Initialize the audio stream.
		let err_fn = |e| error!("error [audio stream]: {e}");
		let host = cpal::default_host();
		let dev = host
			.default_output_device()
			.ok_or_else(|| anyhow!("no audio output device detected"))?;
		let config = dev.default_output_config()?;
		fl.set_sample_rate(config.sample_rate().0 as f32);
		let synth = Arc::new(Mutex::new(fl));
		let fl = Arc::clone(&synth);

		let stream = match config.sample_format() {
			SampleFormat::I16 | SampleFormat::U16 => dev.build_output_stream(
				&config.config(),
				move |data: &mut [i16], _: &OutputCallbackInfo| {
					let fl = fl.lock();

					if let Err(e) = fl.write(data) {
						error!("error writing samples: {e}");
					}
				},
				err_fn,
			),
			SampleFormat::F32 => dev.build_output_stream(
				&config.config(),
				move |data: &mut [f32], _: &OutputCallbackInfo| {
					let fl = fl.lock();

					if let Err(e) = fl.write(data) {
						error!("error writing samples: {e}");
					}
				},
				err_fn,
			),
		}?;

		stream.play()?;

		Ok(Self {
			synth,
			_stream: stream,
		})
	}
}

impl Connection for Fluid {
	type Error = fluidlite::Error;

	fn play(&mut self, msg: &MidiEvent) -> Result<(), Self::Error> {
		use midly::MidiMessage as M;

		let fl = self.synth.lock();
		let c = msg.channel.as_int() as u32;
		match msg.message {
			M::NoteOff { key, .. } => fl.note_off(c, key.as_int() as _),
			M::NoteOn { key, vel } => fl.note_on(c, key.as_int() as _, vel.as_int() as _),
			M::ProgramChange { program } => fl.program_change(c, program.as_int() as _),
			M::Aftertouch { key, vel } => fl.key_pressure(c, key.as_int() as _, vel.as_int() as _),
			M::ChannelAftertouch { vel } => fl.channel_pressure(c, vel.as_int() as _),
			M::PitchBend { bend } => fl.pitch_bend(c, bend.0.as_int() as _),
			M::Controller { controller, value } => {
				fl.cc(c, controller.as_int() as _, value.as_int() as _)
			}
		}
	}
}
