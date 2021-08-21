use std::fs;
use std::path::{
	Path,
	PathBuf,
};

use pulldown_cmark::{
	html,
	Options,
	Parser,
};

pub struct Date {
	pub month: u8,
	pub day: u8,
	pub year: u16,
}

impl Date {
	fn new(text: &str) -> Option<Self> {
		let mut split = text.split('/');

		let month = split.next()?;
		let day = split.next()?;
		let year = split.next()?;

		Some(Self {
			month: month.parse().ok()?,
			day: day.parse().ok()?,
			year: year.parse().ok()?,
		})
	}
}

pub struct Blog {
	pub title: String,
	pub date: Date,

	pub body: String,
}

#[derive(Debug)]
pub enum BlogError {
	PathNotValid(PathBuf),
	MissingTitle,
	MissingDate,
	InvalidDate,
}

impl Blog {
	pub fn new(path: &Path) -> Result<Self, BlogError> {
		let file =
			fs::read_to_string(path).map_err(|_| BlogError::PathNotValid(PathBuf::from(path)))?;

		let mut lines = file.lines();
		let title = lines.next().ok_or(BlogError::MissingTitle)?.to_string();
		let date_string = lines.next().ok_or(BlogError::MissingDate)?;
		let date = Date::new(date_string).ok_or(BlogError::InvalidDate)?;

		let lines: Vec<&str> = lines.collect();

		let mut to_parse = String::new();
		lines.iter().for_each(|s| {
			to_parse.push_str(s);
			to_parse.push('\n');
		});

		let parser = Parser::new_ext(&to_parse, Options::all());

		let mut body = String::new();
		html::push_html(&mut body, parser);

		Ok(Blog { title, date, body })
	}
}
