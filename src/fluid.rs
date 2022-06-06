use std::{
	fmt,
	path::{
		Path,
		PathBuf,
	},
	sync::Arc,
};

use cpal::{
	traits::{
		DeviceTrait,
		HostTrait,
		StreamTrait,
	},
	BuildStreamError,
	DefaultStreamConfigError,
	OutputCallbackInfo,
	PlayStreamError,
	SampleFormat,
	Stream,
	StreamConfig,
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
	pub fn new<P: AsRef<Path>>(sf: P) -> Result<Self, Error> {
		let fl = Synth::new(Settings::new()?)?;
		fl.sfload(sf.as_ref(), true)
			.map_err(|error| Error::Soundfont {
				path: sf.as_ref().into(),
				error,
			})?;

		// Initialize the audio stream.
		let err_fn = |e| error!("error [audio stream]: {e}");
		let host = cpal::default_host();
		let dev = host.default_output_device().ok_or(Error::NoOutputDevice)?;

		let def_config = dev
			.default_output_config()
			.map_err(Error::DefaultStreamConfig)?;
		fl.set_sample_rate(def_config.sample_rate().0 as f32);
		let synth = Arc::new(Mutex::new(fl));
		let fl = Arc::clone(&synth);
		let config = def_config.config();

		let stream = match def_config.sample_format() {
			SampleFormat::I16 | SampleFormat::U16 => dev.build_output_stream(
				&config,
				move |data: &mut [i16], _: &OutputCallbackInfo| {
					let fl = fl.lock();

					if let Err(e) = fl.write(data) {
						error!("error writing samples: {e}");
					}
				},
				err_fn,
			),
			SampleFormat::F32 => dev.build_output_stream(
				&config,
				move |data: &mut [f32], _: &OutputCallbackInfo| {
					let fl = fl.lock();

					if let Err(e) = fl.write(data) {
						error!("error writing samples: {e}");
					}
				},
				err_fn,
			),
		}
		.map_err(|error| Error::BuildStream {
			config: config.clone(),
			error,
		})?;

		stream
			.play()
			.map_err(|error| Error::PlayStream { config, error })?;

		Ok(Self {
			synth,
			_stream: stream,
		})
	}
}

impl Connection for Fluid {
	fn play(&mut self, msg: &MidiEvent) -> bool {
		use nodi::midly::MidiMessage as M;

		let fl = self.synth.lock();
		let c = msg.channel.as_int() as u32;
		let res = match msg.message {
			M::NoteOff { key, .. } => fl.note_off(c, key.as_int() as _),
			M::NoteOn { key, vel } => fl.note_on(c, key.as_int() as _, vel.as_int() as _),
			M::ProgramChange { program } => fl.program_change(c, program.as_int() as _),
			M::Aftertouch { key, vel } => fl.key_pressure(c, key.as_int() as _, vel.as_int() as _),
			M::ChannelAftertouch { vel } => fl.channel_pressure(c, vel.as_int() as _),
			M::PitchBend { bend } => fl.pitch_bend(c, bend.0.as_int() as _),
			M::Controller { controller, value } => {
				fl.cc(c, controller.as_int() as _, value.as_int() as _)
			}
		};
		if let Err(e) = res {
			log::debug!(target: "midi_event", "{e}");
		}
		true
	}
}

#[derive(Debug)]
pub enum Error {
	Soundfont {
		path: PathBuf,
		error: fluidlite::Error,
	},
	Fluidlite(fluidlite::Error),
	NoOutputDevice,
	DefaultStreamConfig(DefaultStreamConfigError),
	BuildStream {
		config: StreamConfig,
		error: BuildStreamError,
	},
	PlayStream {
		config: StreamConfig,
		error: PlayStreamError,
	},
}

impl std::error::Error for Error {}

impl From<fluidlite::Error> for Error {
	fn from(e: fluidlite::Error) -> Self {
		Self::Fluidlite(e)
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Soundfont { path, error } => write!(
				f,
				"failed loading the soundfont {} ({})",
				path.display(),
				error
			),
			Self::Fluidlite(e) => e.fmt(f),
			Self::NoOutputDevice => f.write_str("no audio output device detected"),
			Self::DefaultStreamConfig(e) => e.fmt(f),
			Self::BuildStream { error, .. } => error.fmt(f),
			Self::PlayStream { error, .. } => error.fmt(f),
		}
	}
}
