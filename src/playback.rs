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

fn print(s: &str, clear: usize) {
	fn inner(s: &str, clear: usize) -> io::Result<()> {
		let mut stdout = io::stdout();
		for _ in 0..clear {
			stdout.execute(Clear(ClearType::CurrentLine))?;
		}

		for (i, ln) in s.lines().filter(|s| !s.is_empty()).enumerate() {
			if i > 0 {
				writeln!(stdout)?;
			}
			write!(stdout, "{}\r", ln)?;
			stdout.flush()?;
		}

		Ok(())
	}

	match is_raw_mode_enabled() {
		Ok(true) => {
			if let Err(e) = inner(s, clear) {
				log::error!("failed to write: {e}");
			}
		}
		_ => println!("{s}"),
	}
}

pub(crate) fn play<C: Connection>(
	mut con: C,
	tracks: &[Track],
	mut commands: Receiver<Command>,
	repeat: bool,
	speed: f32,
) {
	let mut n_track = 0;
	let mut paused = false;

	'outer: loop {
		con.send_sys_rt(SystemRealtime::Reset);
		let mut counter = 0_u32;
		let track = &tracks[n_track];
		let mut timer = Ticker::new(track.tpb);
		timer.speed = speed;
		let dur = Duration::from_micros((track.duration.as_micros() as f64 * speed as f64) as u64);
		print(
			&format!(
				"playing {} [{}/{}]",
				track.name,
				n_track + 1,
				tracks.len() + 1
			),
			if paused { 3 } else { 2 },
		);
		print(&format!("duration: {}", format_duration(dur)), 0);
		paused = false;

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
					print("paused", 0);
					paused = true;
					// Wait for the next command.
					match block_on(commands.next()) {
						None => break 'outer,
						Some(Command::Pause) => print("unpaused", 1),
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

	// Send notification that we're done playing.
	// let _ = done.send(());
}
