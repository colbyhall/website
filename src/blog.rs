use std::fs;
use std::path::{
	Path,
	PathBuf,
};

use pulldown_cmark::{
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

impl ToString for Date {
	fn to_string(&self) -> String {
		static MONTHS: [&str; 13] = [
			"invalid",
			"January",
			"February",
			"March",
			"April",
			"May",
			"June",
			"July",
			"August",
			"September",
			"October",
			"November",
			"Decemeber",
		];

		format!(
			"{} {}, {}",
			MONTHS[self.month as usize], self.day, self.year
		)
	}
}

pub struct Blog {
	// Current version is 0
	//
	// + Version 0
	//		- Added Title
	//      - Added Date
	pub version: u32,
	pub title: String,
	pub date: Date,

	pub body: String,
}

#[derive(Debug)]
pub enum BlogError {
	PathNotValid(PathBuf),
	MissingVersion,
	InvalidVersion,
	MissingTitle,
	MissingDate,
	InvalidDate,
}

impl Blog {
	pub fn new(path: &Path) -> Result<Self, BlogError> {
		let file =
			fs::read_to_string(path).map_err(|_| BlogError::PathNotValid(PathBuf::from(path)))?;

		let mut lines = file.lines();

		let version = lines
			.next()
			.ok_or(BlogError::MissingVersion)?
			.parse::<u32>()
			.map_err(|_| BlogError::InvalidVersion)?;

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
		HtmlWriter::new(parser, &mut body).run().unwrap();

		Ok(Blog {
			version,
			title,
			date,
			body,
		})
	}
}

// This is a custom HTML writer. Its just a copy from pulldown_cmark with some mods

use std::collections::HashMap;
use std::io::{self,};

use pulldown_cmark::escape::{
	escape_href,
	escape_html,
	StrWrite,
};
use pulldown_cmark::CowStr;
use pulldown_cmark::Event::*;
use pulldown_cmark::{
	Alignment,
	CodeBlockKind,
	Event,
	LinkType,
	Tag,
};

enum TableState {
	Head,
	Body,
}

struct HtmlWriter<'a, I, W> {
	/// Iterator supplying events.
	iter: I,

	/// Writer to write to.
	writer: W,

	/// Whether or not the last write wrote a newline.
	end_newline: bool,

	table_state: TableState,
	table_alignments: Vec<Alignment>,
	table_cell_index: usize,
	numbers: HashMap<CowStr<'a>, usize>,
}

impl<'a, I, W> HtmlWriter<'a, I, W>
where
	I: Iterator<Item = Event<'a>>,
	W: StrWrite,
{
	fn new(iter: I, writer: W) -> Self {
		Self {
			iter,
			writer,
			end_newline: true,
			table_state: TableState::Head,
			table_alignments: vec![],
			table_cell_index: 0,
			numbers: HashMap::new(),
		}
	}

	/// Writes a new line.
	fn write_newline(&mut self) -> io::Result<()> {
		self.end_newline = true;
		self.writer.write_str("\n")
	}

	/// Writes a buffer, and tracks whether or not a newline was written.
	#[inline]
	fn write(&mut self, s: &str) -> io::Result<()> {
		self.writer.write_str(s)?;

		if !s.is_empty() {
			self.end_newline = s.ends_with('\n');
		}
		Ok(())
	}

	pub fn run(mut self) -> io::Result<()> {
		while let Some(event) = self.iter.next() {
			match event {
				Start(tag) => {
					self.start_tag(tag)?;
				}
				End(tag) => {
					self.end_tag(tag)?;
				}
				Text(text) => {
					escape_html(&mut self.writer, &text)?;
					self.end_newline = text.ends_with('\n');
				}
				Code(text) => {
					if text.contains('\n') {
						self.write("<pre><code>")?;
						escape_html(&mut self.writer, &text)?;
						self.write("</code></pre>")?;
					} else {
						self.write("<code>")?;
						escape_html(&mut self.writer, &text)?;
						self.write("</code>")?;
					}
				}
				Html(html) => {
					self.write(&html)?;
				}
				SoftBreak => {
					self.write_newline()?;
				}
				HardBreak => {
					self.write("<br />\n")?;
				}
				Rule => {
					if self.end_newline {
						self.write("<hr />\n")?;
					} else {
						self.write("\n<hr />\n")?;
					}
				}
				FootnoteReference(name) => {
					let len = self.numbers.len() + 1;
					self.write("<sup class=\"footnote-reference\"><a href=\"#")?;
					escape_html(&mut self.writer, &name)?;
					self.write("\">")?;
					let number = *self.numbers.entry(name).or_insert(len);
					write!(&mut self.writer, "{}", number)?;
					self.write("</a></sup>")?;
				}
				TaskListMarker(true) => {
					self.write("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n")?;
				}
				TaskListMarker(false) => {
					self.write("<input disabled=\"\" type=\"checkbox\"/>\n")?;
				}
			}
		}
		Ok(())
	}

	/// Writes the start of an HTML tag.
	fn start_tag(&mut self, tag: Tag<'a>) -> io::Result<()> {
		match tag {
			Tag::Paragraph => {
				if self.end_newline {
					self.write("<p>")
				} else {
					self.write("\n<p>")
				}
			}
			Tag::Heading(level) => {
				if self.end_newline {
					self.end_newline = false;
					write!(&mut self.writer, "<h{}>", level)
				} else {
					write!(&mut self.writer, "\n<h{}>", level)
				}
			}
			Tag::Table(alignments) => {
				self.table_alignments = alignments;
				self.write("<table>")
			}
			Tag::TableHead => {
				self.table_state = TableState::Head;
				self.table_cell_index = 0;
				self.write("<thead><tr>")
			}
			Tag::TableRow => {
				self.table_cell_index = 0;
				self.write("<tr>")
			}
			Tag::TableCell => {
				match self.table_state {
					TableState::Head => {
						self.write("<th")?;
					}
					TableState::Body => {
						self.write("<td")?;
					}
				}
				match self.table_alignments.get(self.table_cell_index) {
					Some(&Alignment::Left) => self.write(" align=\"left\">"),
					Some(&Alignment::Center) => self.write(" align=\"center\">"),
					Some(&Alignment::Right) => self.write(" align=\"right\">"),
					_ => self.write(">"),
				}
			}
			Tag::BlockQuote => {
				if self.end_newline {
					self.write("<blockquote>\n")
				} else {
					self.write("\n<blockquote>\n")
				}
			}
			Tag::CodeBlock(info) => {
				if !self.end_newline {
					self.write_newline()?;
				}
				match info {
					CodeBlockKind::Fenced(info) => {
						let lang = info.split(' ').next().unwrap();
						if lang.is_empty() {
							self.write("<pre><code>")
						} else {
							self.write("<pre><code class=\"language-")?;
							escape_html(&mut self.writer, lang)?;
							self.write("\">")
						}
					}
					CodeBlockKind::Indented => self.write("<pre><code>"),
				}
			}
			Tag::List(Some(1)) => {
				if self.end_newline {
					self.write("<ol>\n")
				} else {
					self.write("\n<ol>\n")
				}
			}
			Tag::List(Some(start)) => {
				if self.end_newline {
					self.write("<ol start=\"")?;
				} else {
					self.write("\n<ol start=\"")?;
				}
				write!(&mut self.writer, "{}", start)?;
				self.write("\">\n")
			}
			Tag::List(None) => {
				if self.end_newline {
					self.write("<ul>\n")
				} else {
					self.write("\n<ul>\n")
				}
			}
			Tag::Item => {
				if self.end_newline {
					self.write("<li>")
				} else {
					self.write("\n<li>")
				}
			}
			Tag::Emphasis => self.write("<em>"),
			Tag::Strong => self.write("<strong>"),
			Tag::Strikethrough => self.write("<del>"),
			Tag::Link(LinkType::Email, dest, title) => {
				self.write("<a href=\"mailto:")?;
				escape_href(&mut self.writer, &dest)?;
				if !title.is_empty() {
					self.write("\" title=\"")?;
					escape_html(&mut self.writer, &title)?;
				}
				self.write("\">")
			}
			Tag::Link(_link_type, dest, title) => {
				self.write("<a href=\"")?;
				escape_href(&mut self.writer, &dest)?;
				if !title.is_empty() {
					self.write("\" title=\"")?;
					escape_html(&mut self.writer, &title)?;
				}
				self.write("\">")
			}
			Tag::Image(_link_type, dest, title) => {
				self.write("<img src=\"")?;
				escape_href(&mut self.writer, &dest)?;
				self.write("\" alt=\"")?;
				self.raw_text()?;
				if !title.is_empty() {
					self.write("\" title=\"")?;
					escape_html(&mut self.writer, &title)?;
				}
				self.write("\" />")
			}
			Tag::FootnoteDefinition(name) => {
				if self.end_newline {
					self.write("<div class=\"footnote-definition\" id=\"")?;
				} else {
					self.write("\n<div class=\"footnote-definition\" id=\"")?;
				}
				escape_html(&mut self.writer, &*name)?;
				self.write("\"><sup class=\"footnote-definition-label\">")?;
				let len = self.numbers.len() + 1;
				let number = *self.numbers.entry(name).or_insert(len);
				write!(&mut self.writer, "{}", number)?;
				self.write("</sup>")
			}
		}
	}

	fn end_tag(&mut self, tag: Tag) -> io::Result<()> {
		match tag {
			Tag::Paragraph => {
				self.write("</p>\n")?;
			}
			Tag::Heading(level) => {
				self.write("</h")?;
				write!(&mut self.writer, "{}", level)?;
				self.write(">\n")?;
			}
			Tag::Table(_) => {
				self.write("</tbody></table>\n")?;
			}
			Tag::TableHead => {
				self.write("</tr></thead><tbody>\n")?;
				self.table_state = TableState::Body;
			}
			Tag::TableRow => {
				self.write("</tr>\n")?;
			}
			Tag::TableCell => {
				match self.table_state {
					TableState::Head => {
						self.write("</th>")?;
					}
					TableState::Body => {
						self.write("</td>")?;
					}
				}
				self.table_cell_index += 1;
			}
			Tag::BlockQuote => {
				self.write("</blockquote>\n")?;
			}
			Tag::CodeBlock(_) => {
				self.write("</code></pre>\n")?;
			}
			Tag::List(Some(_)) => {
				self.write("</ol>\n")?;
			}
			Tag::List(None) => {
				self.write("</ul>\n")?;
			}
			Tag::Item => {
				self.write("</li>\n")?;
			}
			Tag::Emphasis => {
				self.write("</em>")?;
			}
			Tag::Strong => {
				self.write("</strong>")?;
			}
			Tag::Strikethrough => {
				self.write("</del>")?;
			}
			Tag::Link(_, _, _) => {
				self.write("</a>")?;
			}
			Tag::Image(_, _, _) => (), // shouldn't happen, handled in start
			Tag::FootnoteDefinition(_) => {
				self.write("</div>\n")?;
			}
		}
		Ok(())
	}

	// run raw text, consuming end tag
	fn raw_text(&mut self) -> io::Result<()> {
		let mut nest = 0;
		while let Some(event) = self.iter.next() {
			match event {
				Start(_) => nest += 1,
				End(_) => {
					if nest == 0 {
						break;
					}
					nest -= 1;
				}
				Html(text) | Code(text) | Text(text) => {
					escape_html(&mut self.writer, &text)?;
					self.end_newline = text.ends_with('\n');
				}
				SoftBreak | HardBreak | Rule => {
					self.write(" ")?;
				}
				FootnoteReference(name) => {
					let len = self.numbers.len() + 1;
					let number = *self.numbers.entry(name).or_insert(len);
					write!(&mut self.writer, "[{}]", number)?;
				}
				TaskListMarker(true) => self.write("[x]")?,
				TaskListMarker(false) => self.write("[ ]")?,
			}
		}
		Ok(())
	}
}
