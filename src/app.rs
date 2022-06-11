use cfg_if::cfg_if;
use clap::{
	arg,
	crate_authors,
	crate_version,
	Command,
};

#[cfg(feature = "fluidlite")]
const DEFAULT_SOUNDFONT: &str = {
	if cfg!(windows) {
		r"C:\soundfonts\default.sf2"
	} else {
		"/usr/share/soundfonts/default.sf2"
	}
};

pub fn new() -> Command<'static> {
	cfg_if! {
		if #[cfg(feature = "midir")] {
			let files = arg!([file] ... "MIDI (*.mid) files to play.").required_unless_present("list");
		} else {
			let files = arg!(<file> ... "MIDI (*.mid) files to play.");
		}
	}

	let c = Command::new("plmidi")
		.about("Play MIDI files.")
		.version(crate_version!())
		.author(crate_authors!())
		.arg_required_else_help(true)
		.args(&[
			arg!(-s --shuffle "Shuffle given tracks."),
			arg!(-r --repeat "Repeat playback."),
			arg!(-v --verbose ... "Verbosity level."),
			arg!(-x --speed [MODIFIER] "The playback speed. 1.0 is the normal speed.")
				.default_value("1.0")
				.validator(|f| match f.parse::<f32>() {
					Ok(f) if f > 0.0 => Ok(()),
					_ => Err(String::from("the value must be a number greater than 0.0")),
				}),
			arg!(-t --transpose [N] "Transpose the track N semi-tones.")
				.default_value("0")
				.validator(validate::<i8>("the value must be between -64 and 64."))
				.allow_hyphen_values(true),
			files,
		]);

	#[cfg(feature = "fluidlite")]
	let c = c.arg(
		arg!(-f --fluid [SOUNDFONT] "The soundfont to use with the embedded fluidsynth.")
			.env("SOUNDFONT")
			.default_value(DEFAULT_SOUNDFONT),
	);

	cfg_if! {
		if #[cfg(all(feature = "fluidlite", feature = "midir"))] {
			c.args(&[
			arg!(-l --list "List available MIDI output devices."),
			arg!(-d --device [INDEX] "Bind to the given MIDI output device instead of using the embedded synthesizer. Use --list to list available devices.")
			.validator(validate::<usize>("the value msut be a number greater than or equal to 0")),
			])
		} else if #[cfg(feature = "midir")] {
			c.args(&[
			arg!(-l --list "List available MIDI output devices."),
			arg!(-d --device [INDEX] "Bind to the given MIDI output device instead of using the embedded synthesizer. Use --list to list available devices.")
			.default_value("0")
			.validator(validate::<usize>("the value msut be a number greater than or equal to 0")),
			])
		} else {
			c
		}
	}
}

fn validate<T: std::str::FromStr>(msg: &'static str) -> impl Fn(&str) -> Result<(), String> {
	move |s| s.parse::<T>().map(|_| {}).map_err(|_| String::from(msg))
}
