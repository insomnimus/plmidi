mod app;

use std::{
	convert::TryFrom,
	error::Error,
	fs,
	process,
	time::Duration,
};

use midir::{
	MidiOutput,
	MidiOutputConnection,
};
use midly::{
	Format,
	Smf,
};
use nodi::{
	Player,
	Sheet,
	Ticker,
	Timer,
};

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

fn list_devices() -> Result<(), Box<dyn Error>> {
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

fn get_midi(n: usize) -> Result<MidiOutputConnection, Box<dyn Error>> {
	let midi_out = MidiOutput::new("nodi")?;

	let out_ports = midi_out.ports();
	if out_ports.is_empty() {
		return Err("no MIDI output device detected".into());
	}
	if n >= out_ports.len() {
		return Err(format!(
			"only {} MIDI devices detected; run with --list  to see them",
			out_ports.len()
		)
		.into());
	}

	let out_port = &out_ports[n];
	let out = midi_out.connect(out_port, "cello-tabs")?;
	Ok(out)
}

fn run() -> Result<(), Box<dyn Error>> {
	let m = app::new().get_matches();
	if m.is_present("list") {
		return list_devices();
	}

	let speed = m.value_of("speed").unwrap().parse::<f32>().unwrap();
	let n_device = m.value_of("device").unwrap().parse::<usize>().unwrap();
	let file_name = m.value_of("file").unwrap();

	let out = get_midi(n_device)?;
	let data = fs::read(file_name)?;

	let Smf { header, tracks } = Smf::parse(&data)?;

	let mut timer = Ticker::try_from(header.timing)?;
	timer.speed = speed;

	let sheet = match header.format {
		Format::SingleTrack | Format::Sequential => Sheet::sequential(&tracks),
		Format::Parallel => Sheet::parallel(&tracks),
	};

	println!("Playing {}", &file_name);
	let mut t = timer;
	println!(
		"Total duration: {}",
		format_duration(t.duration(&sheet[..]))
	);

	let mut player = Player::new(out, timer);
	player.play_sheet(&sheet);
	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {}", e);
		process::exit(2);
	}
}
