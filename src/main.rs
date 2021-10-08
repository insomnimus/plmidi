mod app;

use std::{
	convert::TryFrom,
	fs,
	io::{self, Write},
	path::PathBuf,
	process,
	sync::mpsc::{self, SyncSender},
	thread,
	time::Duration,
};

use crossterm::{
	event::{self, Event, KeyCode, KeyModifiers},
	terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, Clear, ClearType},
	ExecutableCommand,
};
use log::Level;
use midir::{MidiOutput, MidiOutputConnection};
use midly::{Format, Smf};
use nodi::{timers::Ticker, Player, Sheet, Timer};

type Error = Box<dyn ::std::error::Error>;

fn print(s: &str) {
	fn inner(s: &str) -> Result<(), io::Error> {
		let mut stdout = io::stdout();
		stdout.execute(Clear(ClearType::UntilNewLine))?;
		for (i, ln) in s.lines().filter(|s| !s.is_empty()).enumerate() {
			if i > 0 {
				writeln!(&mut stdout)?;
			}
			write!(&mut stdout, "{}\r", ln)?;
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

fn list_devices() -> Result<(), Error> {
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
	let midi_out = MidiOutput::new("nodi")?;

	let out_ports = midi_out.ports();
	if out_ports.is_empty() {
		return Err("no midi output device detected".into());
	}

	if n >= out_ports.len() {
		return Err(format!(
			"only {} MIDI devices detected; run with --list  to see them",
			out_ports.len()
		)
		.into());
	}

	let out_port = &out_ports[n];
	let out = midi_out.connect(out_port, "plmidi")?;
	Ok(out)
}

fn run() -> Result<(), Error> {
	let m = app::new().get_matches();
	if m.is_present("list") {
		return list_devices();
	}

	match m.occurrences_of("verbose") {
		1 => simple_logger::init_with_level(Level::Info)?,
		2 => simple_logger::init_with_level(Level::Debug)?,
		_ => (),
	};

	let speed = m.value_of("speed").unwrap().parse::<f32>().unwrap();
	let transpose = m
		.value_of("transpose")
		.map(|s| s.parse::<i8>().unwrap())
		.unwrap_or(0);
	let n_device = m.value_of("device").unwrap().parse::<usize>().unwrap();
	let file_name = PathBuf::from(m.value_of("file").unwrap());

	let out = get_midi(n_device)?;
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

	let mut player = Player::new(timer, out);
	player.play_sheet(&sheet);
	Ok(())
}

fn listen_keys(sender: SyncSender<()>) {
	if let Err(e) = enable_raw_mode() {
		eprintln!("warning: failed to enable raw input mode: {}", e);
	}
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
	process::exit(if let Err(e) = disable_raw_mode() {
		eprintln!("warning: failed to disable raw input mode: {}", e);
		1
	} else {
		0
	});
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {}", e);
		process::exit(2);
	}
}
