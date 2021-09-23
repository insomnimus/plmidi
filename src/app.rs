use clap::{crate_authors, crate_version, App, AppSettings, Arg};

pub fn new() -> App<'static> {
	let app = App::new("plmidi")
		.about("Play MIDI files.")
		.version(crate_version!())
		.author(crate_authors!())
		.setting(AppSettings::UnifiedHelpMessage)
		.setting(AppSettings::ArgRequiredElseHelp);

	let file = Arg::new("file")
		.required_unless_present("list")
		.about("A MIDI file (.mid) to be played.");

	let device = Arg::new("device")
		.short('d')
		.long("device")
		.takes_value(true)
		.default_value("0")
		.about("The index of the MIDI device that will be used for synthesis.")
		.validator(|s| {
			s.parse::<usize>()
				.map(|_| {})
				.map_err(|_| String::from("the value must be a number greater than or equal to 0"))
		});

	let list = Arg::new("list")
		.short('l')
		.long("list")
		.about("List available MIDI output devices.");

	let speed = Arg::new("speed")
		.short('x')
		.long("speed")
		.about("The playback rate, 1.0 = normal.")
		.takes_value(true)
		.validator(|s| match s.parse::<f32>() {
			Err(_) => Err(String::from("the value must be any number above 0.0")),
			Ok(n) if n <= 0.0 => Err(String::from("the value must be any number above 0.0")),
			Ok(_) => Ok(()),
		})
		.default_value("1.0");

	app.arg(device).arg(list).arg(speed).arg(file)
}
