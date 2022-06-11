use std::{
	fs,
	path::Path,
	time::Duration,
};

use nodi::{
	midly::{
		Format,
		Smf,
		Timing,
	},
	timers::Ticker,
	Sheet,
	Timer,
};

pub struct Track {
	pub name: String,
	pub tpb: u16,
	pub sheet: Sheet,
	pub duration: Duration,
}

impl Track {
	pub fn new<P: AsRef<Path>>(p: P) -> Result<Self, String> {
		let p = p.as_ref();
		let data = fs::read(p).map_err(|e| format!("error reading {}: {}", p.display(), e))?;
		let Smf { header, tracks } =
			Smf::parse(&data).map_err(|e| format!("can't parse {} as midi: {}", p.display(), e))?;
		let tpb = match &header.timing {
			Timing::Metrical(n) => n.as_int(),
			_ => return Err(format!("{} has an unsupported time format", p.display())),
		};
		let sheet = match header.format {
			Format::SingleTrack | Format::Sequential => Sheet::sequential(&tracks),
			Format::Parallel => Sheet::parallel(&tracks),
		};
		let duration = Ticker::new(tpb).duration(&sheet);

		Ok(Self {
			name: p
				.file_stem()
				.unwrap_or(p.as_os_str())
				.to_string_lossy()
				.into_owned(),
			sheet,
			duration,
			tpb,
		})
	}
}
