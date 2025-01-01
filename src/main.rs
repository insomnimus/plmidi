#[cfg(not(any(feature = "fluidlite", feature = "system")))]
compile_error!("you must enable at least one of fluid, fluid-bundled or system cargo features");

mod app;
#[cfg(feature = "fluidlite")]
mod fluid;
mod playback;
mod track;

use std::{
	io::{
		self,
	},
	process,
	thread,
};

use cfg_if::cfg_if;
use crossterm::{
	event::{
		Event,
		EventStream,
		KeyCode,
		KeyEventKind,
		KeyModifiers,
		KeyboardEnhancementFlags,
		PopKeyboardEnhancementFlags,
		PushKeyboardEnhancementFlags,
	},
	execute,
	terminal::{
		disable_raw_mode,
		enable_raw_mode,
		EnterAlternateScreen,
		LeaveAlternateScreen,
	},
};
use futures::{
	channel::mpsc::{
		self,
		Receiver,
		Sender,
	},
	executor::block_on,
	prelude::*,
	select,
};
use log::{
	error,
	Level,
};
#[cfg(feature = "system")]
use nodi::midir::{
	MidiOutput,
	MidiOutputConnection,
};
use rand::prelude::SliceRandom;

use self::track::Track;

type Result<T, E = Box<dyn std::error::Error>> = ::std::result::Result<T, E>;

enum Command {
	Pause,
	Next,
	Prev,
}

#[cfg(all(feature = "fluidlite", feature = "system"))]
enum Either<A, B> {
	Left(A),
	Right(B),
}

fn init_logger(n: u64) -> Result<(), log::SetLoggerError> {
	let log = match n {
		0 => Level::Error,
		1 => Level::Warn,
		2 => Level::Info,
		_ => Level::Debug,
	};

	#[cfg(feature = "fluidlite")]
	{
		use fluidlite::LogLevel;
		struct L;
		impl fluidlite::Logger for L {
			fn log(&mut self, level: LogLevel, msg: &str) {
				match level {
					LogLevel::Error | LogLevel::Panic => log::error!(target: "fluidsynth", "{msg}"),
					LogLevel::Warning => log::warn!(target: "fluidsynth", "{msg}"),
					LogLevel::Info => log::info!(target: "fluidsynth", "{msg}"),
					_ => log::debug!(target: "fluidsynth", "{msg}"),
				}
			}
		}
		fluidlite::Log::set(LogLevel::DEBUG, L);
	}
	simple_logger::init_with_level(log)?;
	Ok(())
}

#[cfg(feature = "system")]
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

#[cfg(feature = "system")]
fn get_midi(n: usize) -> Result<MidiOutputConnection> {
	let midi_out = MidiOutput::new("nodi")?;

	let out_ports = midi_out.ports();
	if out_ports.is_empty() {
		return Err("no midi output device detected".into());
	}
	if n >= out_ports.len() {
		return Err(format!(
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
	let m = app::new().get_matches_from(wild::args());
	#[cfg(feature = "system")]
	if m.is_present("list") {
		return list_devices();
	}

	init_logger(m.occurrences_of("verbose"))?;

	let speed = m.value_of_t_or_exit::<f32>("speed");
	let repeat = m.is_present("repeat");
	let shuffle = m.is_present("shuffle");
	let transpose = m.value_of_t_or_exit::<i8>("transpose");

	cfg_if! {
		if #[cfg(all(feature = "fluidlite", feature = "system"))] {
				let con = match m.value_of_t::<usize>("device") {
		Err(_) => Either::Left(fluid::Fluid::new(m.value_of("fluid").unwrap())?),
		Ok(n) => Either::Right(get_midi(n)?),
	};
		} else if #[cfg(feature = "fluidlite")] {
			let con = fluid::Fluid::new(m.value_of("fluid").unwrap())?;
		} else if #[cfg(feature = "system")] {
			let con = get_midi(m.value_of_t_or_exit("device"))?;
		} else {
			compile_error!("you must enable at least one of fluid, fluid-bundled or system cargo features");
		}
	}

	let mut tracks = m
		.values_of("file")
		.into_iter()
		.flatten()
		.map(Track::new)
		.collect::<Result<Vec<_>, _>>()?;

	for t in &mut tracks {
		t.sheet.transpose(transpose, false);
	}

	if shuffle {
		tracks.shuffle(&mut rand::thread_rng());
	}

	let (sender, receiver) = mpsc::channel(1);

	let (mut tx_done, rx_done) = mpsc::channel(0);
	let listen = thread::spawn(move || block_on(async move { listen_keys(sender, rx_done).await }));

	cfg_if! {
		if #[cfg(all(feature = "fluidlite", feature = "system"))] {
			match con {
		Either::Left(con) => playback::play(con, &tracks, receiver, repeat, speed),
		Either::Right(con) => playback::play(con, &tracks, receiver, repeat, speed),
	}
		} else {
			playback::play(con, &tracks, receiver, repeat, speed);
		}
	}

	let _ = block_on(tx_done.send(()));
	let _ = listen.join();
	Ok(())
}

async fn listen_keys(mut sender: Sender<Command>, done: Receiver<()>) {
	let alt = execute!(io::stdout(), EnterAlternateScreen).is_ok();
	if let Err(e) = enable_raw_mode() {
		eprintln!("warning: failed to enable raw mode; hotkeys may not work properly: {e}");
	} else {
		let _ = execute!(
			io::stdout(),
			PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
		);
	}

	let mut events = EventStream::new()
		.take_while(|x| std::future::ready(x.is_ok()))
		.fuse();
	let mut done = done.fuse();

	let received_done = loop {
		let res = select! {
			_ = done.next() => break true,
			e = events.next() => e,
		};

		let should_break = match res {
			None => true,
			Some(Err(e)) => {
				error!("input error: {e}");
				true
			}
			Some(Ok(Event::Key(k))) if k.kind == KeyEventKind::Press => match k.code {
				KeyCode::Esc => true,
				KeyCode::Char('c' | 'd' | 'q') if k.modifiers == KeyModifiers::CONTROL => true,
				KeyCode::Left if k.modifiers == KeyModifiers::CONTROL => {
					sender.send(Command::Prev).await.is_err()
				}
				KeyCode::Right if k.modifiers == KeyModifiers::CONTROL => {
					sender.send(Command::Next).await.is_err()
				}
				KeyCode::Char(' ') => sender.send(Command::Pause).await.is_err(),
				_ => false,
			},
			_ => false,
		};
		if should_break {
			break false;
		}
	};

	if !received_done {
		drop(sender);
		let _ = done.next().await;
	}

	let _ = disable_raw_mode();
	if alt {
		let _ = execute!(io::stdout(), LeaveAlternateScreen);
		let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
	}
	process::exit(0);
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {e}");
		process::exit(1);
	}
}
