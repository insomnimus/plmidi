mod app;
#[cfg(feature = "fluidlite")]
mod fluid;

use std::{
	convert::TryFrom,
	fs,
	io::{
		self,
		Write,
	},
	path::PathBuf,
	process,
	sync::mpsc::{
		self,
		SyncSender,
	},
	thread,
	time::Duration,
};

use anyhow::anyhow;
use crossterm::{
	event::{
		self,
		Event,
		KeyCode,
		KeyModifiers,
	},
	terminal::{
		disable_raw_mode,
		enable_raw_mode,
		is_raw_mode_enabled,
		Clear,
		ClearType,
	},
	ExecutableCommand,
};
use log::Level;
use midir::{
	MidiOutput,
	MidiOutputConnection,
};
use midly::{
	Format,
	Smf,
};
use nodi::{
	timers::{
		ControlTicker,
		Ticker,
	},
	Connection,
	Player,
	Sheet,
	Timer,
};

#[cfg(unix)]
type Error = Box<dyn std::error::Error>;
#[cfg(not(unix))]
type Error = anyhow::Error;
type Result<T, E = Error> = ::std::result::Result<T, E>;

fn print(s: &str) {
	fn inner(s: &str) -> io::Result<()> {
		let mut stdout = io::stdout();
		stdout.execute(Clear(ClearType::UntilNewLine))?;
		for (i, ln) in s.lines().filter(|s| !s.is_empty()).enumerate() {
			if i > 0 {
				writeln!(stdout)?;
			}
			write!(stdout, "{}\r", ln)?;
			stdout.flush()?;
		}
		Ok(())
	}
	if let Ok(true) = is_raw_mode_enabled() {
		let _ = inner(s);
	} else {
		println!("{}", s);
	}
}

fn format_duration(t: Duration) -> String {
	let secs = t.as_secs();
	let mins = secs / 60;
	let secs = secs % 60;
	if mins > 0 {
		format!("{}m{}s", mins, secs)
	} else {
		format!("{}s", secs)
	}
}

fn list_devices() -> Result<()> {
	let midi_out = MidiOutput::new("nodi")?;

	let out_ports = midi_out.ports();

	if out_ports.is_empty() {
		println!("No active MIDI output device detected.");
	} else {
		for (i, p) in out_ports.iter().enumerate() {
			println!(
				"#{}: {}",
				i,
				midi_out
					.port_name(p)
					.as_deref()
					.unwrap_or("<no device name>")
			);
		}
	}

	Ok(())
}

fn get_midi(n: usize) -> Result<MidiOutputConnection, Error> {
	#![cfg_attr(not(unix), allow(clippy::useless_conversion))]
	// NOTE: On *NIX, the error this function returns is not Sync so anyhow doesn't
	// work.
	let midi_out = MidiOutput::new("nodi")?;

	let out_ports = midi_out.ports();
	if out_ports.is_empty() {
		return Err(anyhow!("no midi output device detected").into());
	}
	if n >= out_ports.len() {
		return Err(anyhow!(
			"only {} devices detected; run with --list to see them",
			out_ports.len()
		)
		.into());
	}

	let out_port = &out_ports[n];
	let out = midi_out.connect(out_port, "plmidi")?;
	Ok(out)
}

fn run() -> Result<()> {
	let m = app::new().get_matches();
	if m.is_present("list") {
		return list_devices();
	}

	match m.occurrences_of("verbose") {
		1 => simple_logger::init_with_level(Level::Info)?,
		2 => simple_logger::init_with_level(Level::Debug)?,
		_ => (),
	};

	let speed = m.value_of_t_or_exit::<f32>("speed");
	let transpose = m.value_of_t_or_exit::<i8>("transpose");
	let n_device = m.value_of_t_or_exit::<usize>("device");
	let file_name = PathBuf::from(m.value_of("file").unwrap());

	let data = fs::read(&file_name)?;

	let Smf { header, tracks } = Smf::parse(&data)?;
	let (sender, receiver) = mpsc::sync_channel(1);

	let mut timer = Ticker::try_from(header.timing)?;
	timer.speed = speed;

	let mut sheet = match header.format {
		Format::SingleTrack | Format::Sequential => Sheet::sequential(&tracks),
		Format::Parallel => Sheet::parallel(&tracks),
	};

	sheet.transpose(transpose, false);

	println!(
		"Playing {}",
		&file_name.file_name().unwrap_or_default().to_string_lossy()
	);
	let mut t = timer;
	println!(
		"Total duration: {}",
		format_duration(t.duration(&sheet[..]))
	);

	let timer = timer.to_control(receiver);
	thread::spawn(move || {
		listen_keys(sender);
	});
	fn inner<C: Connection>(con: C, sheet: Sheet, timer: ControlTicker) {
		let mut player = Player::new(timer, con);
		player.play_sheet(&sheet);
	}

	#[cfg(not(feature = "fluidlite"))]
	inner(get_midi(n_device)?, sheet, timer);

	#[cfg(feature = "fluidlite")]
	match m.value_of("fluidsynth") {
		Some(p) => inner(fluid::Fluid::new(p)?, sheet, timer),
		None => inner(get_midi(n_device)?, sheet, timer),
	};
	Ok(())
}

fn listen_keys(sender: SyncSender<()>) {
	print("press the spacebar to play/pause, esc to quit");
	let mut paused = false;
	loop {
		let k = match event::read() {
			Ok(Event::Key(k)) => k,
			_ => continue,
		};
		match k.code {
			KeyCode::Esc => break,
			KeyCode::Char('c') if k.modifiers == KeyModifiers::CONTROL => break,
			KeyCode::Char(' ') => {
				sender.send(()).unwrap();
				if paused {
					print("unpaused");
				} else {
					print("paused");
				}
				paused = !paused;
			}
			_ => (),
		};
	}
}

fn main() {
	if let Err(e) = enable_raw_mode() {
		eprintln!("warning: failed to enable raw input mode: {}", e);
	}
	if let Err(e) = run() {
		#[cfg(unix)]
		eprintln!("error: {e}");
		#[cfg(not(unix))]
		eprintln!("error: {e:?}");
		let _ = disable_raw_mode();
		process::exit(1);
	}

	let _ = disable_raw_mode();
}
