use clap::{arg, crate_authors, crate_version, App, AppSettings, Arg, ArgSettings};

pub fn new() -> App<'static> {
	App::new("plmidi")
		.about("Play MIDI files.")
		.version(crate_version!())
		.author(crate_authors!())
		.setting(AppSettings::ArgRequiredElseHelp)
		.args(&[
			arg!(-d --device <NO> "The MIDI output device number.")
				.default_value("0")
				.validator(validate::<usize>(
					"the value must be an integer greater than or equal to 0",
				)),
			arg!(-l --list "List available MIDI output devices."),
			arg!(-x --speed [MODIFIER] "The playback speed. 1.0 is the normal speed.")
				.default_value("1.0")
				.validator(|f| match f.parse::<f32>() {
					Ok(f) if f > 0.0 => Ok(()),
					_ => Err(String::from("the value must be a number greater than 0.0")),
				}),
			arg!(-t --transpose [N] "Transpose the track N semi-tones.")
				.default_value("0")
				.validator(validate::<i8>("the value must be between -64 and 64."))
				.setting(ArgSettings::AllowHyphenValues),
			arg!(-v --verbose ... "Verbosity level"),
			Arg::new("file")
				.required_unless_present("list")
				.help("A MIDI file (.mid) to be played."),
		])
}

fn validate<T: std::str::FromStr>(msg: &'static str) -> impl Fn(&str) -> Result<(), String> {
	move |s| s.parse::<T>().map(|_| {}).map_err(|_| String::from(msg))
}
