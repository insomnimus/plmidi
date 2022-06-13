use std::{
	io::{
		self,
		Write,
	},
	time::Duration,
};

use crossterm::{
	terminal::{
		is_raw_mode_enabled,
		Clear,
		ClearType,
	},
	ExecutableCommand,
};
use futures::{
	channel::mpsc::Receiver,
	executor::block_on,
	prelude::*,
};
use nodi::{
	midly::live::SystemRealtime,
	timers::Ticker,
	Connection,
	Event,
	Timer,
};

use crate::{
	track::Track,
	Command,
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

enum Print {
	Append,
	ReplaceLast,
	ReplaceAll,
}

impl Print {
	fn print(self, s: &str) {
		fn inner(p: Print, s: &str) -> io::Result<()> {
			let mut stdout = io::stdout();
			match p {
				Print::ReplaceAll => {
					stdout.execute(Clear(ClearType::All))?;
				}
				Print::Append => writeln!(stdout)?,
				Print::ReplaceLast => {
					stdout.execute(Clear(ClearType::UntilNewLine))?;
				}
			}

			for (i, ln) in s.lines().enumerate() {
				if i > 0 {
					writeln!(stdout)?;
				}
				write!(stdout, "{ln}\r")?;
				stdout.flush()?;
			}
			Ok(())
		}

		if let Ok(true) = is_raw_mode_enabled() {
			let _ = inner(self, s);
		} else {
			println!("{s}");
		}
	}
}

fn gen_header(tracks: &[Track], speed: f32) -> String {
	let total_duration: Duration = tracks.iter().map(|t| t.duration).sum();
	let dur = Duration::from_micros((total_duration.as_micros() as f64 * speed as f64) as u64);
	format!(
		"Playing {n} track{s}
Total duration: {total}
Press the spacebar to play/pause, ctrl-left/right to play previous/next track
Press the esc key or ctrl-c to exit",
		n = tracks.len(),
		s = if tracks.len() == 1 { "" } else { "s" },
		total = format_duration(dur)
	)
}

pub(crate) fn play<C: Connection>(
	mut con: C,
	tracks: &[Track],
	mut commands: Receiver<Command>,
	repeat: bool,
	speed: f32,
) {
	let header = gen_header(tracks, speed);
	let mut n_track = 0;

	'outer: loop {
		// Reset the synth.
		con.send_sys_rt(SystemRealtime::Reset);

		let mut counter = 0_u32;
		let track = &tracks[n_track];
		let mut timer = Ticker::new(track.tpb);
		timer.speed = speed;
		let dur = Duration::from_micros((track.duration.as_micros() as f64 * speed as f64) as u64);
		Print::ReplaceAll.print(&format!(
			"{header}
Current: {name} [{n_track}/{total}]
Duration = {dur}",
			n_track = n_track + 1,
			total = tracks.len(),
			name = track.name,
			dur = format_duration(dur),
		));

		let mut paused = false;

		for moment in track.sheet.iter() {
			match commands.try_next() {
				Err(_) => (),
				Ok(None) => break 'outer,
				Ok(Some(Command::Next)) => break,
				Ok(Some(Command::Prev)) => {
					n_track = n_track.saturating_sub(1);
					continue 'outer;
				}
				Ok(Some(Command::Pause)) => {
					con.all_notes_off();
					if paused {
						Print::ReplaceLast.print("paused");
					} else {
						Print::Append.print("paused");
						paused = true;
					}
					// Wait for the next command.
					match block_on(commands.next()) {
						None => break 'outer,
						Some(Command::Pause) => Print::ReplaceLast.print("unpaused"),
						Some(Command::Next) => break,
						Some(Command::Prev) => {
							n_track = n_track.saturating_sub(1);
							continue 'outer;
						}
					}
				}
			};

			// Play the moment.
			if !moment.is_empty() {
				timer.sleep(counter);
				counter = 0;
			}
			for event in &moment.events {
				match event {
					Event::Tempo(val) => timer.change_tempo(*val),
					Event::Midi(msg) => {
						con.play(*msg);
					}
					_ => (),
				};
			}

			counter += 1;
		}

		// Current track is over.
		n_track += 1;
		if n_track >= tracks.len() {
			if repeat {
				n_track = 0;
			} else {
				break;
			}
		}
	}
}
