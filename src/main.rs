use handlebars::Handlebars;
use serde::Serialize;
use warp::Filter;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

mod blog;
use blog::*;

#[derive(Serialize)]
struct BaseLayout<'a> {
	title: String,
	body: &'a str,
}

impl<'a> BaseLayout<'a> {
	fn new(title: impl ToString, body: &'a str) -> Self {
		Self {
			title: title.to_string(),
			body,
		}
	}
}

#[derive(Serialize)]
struct BlogLayout<'a> {
	title: &'a str,
	date: &'a str,
	body: &'a str,
}

#[tokio::main]
async fn main() {
	let mut hbs = Handlebars::new();

	let footer_str = fs::read_to_string("views/partials/footer.hbs").unwrap();
	hbs.register_partial("footer", footer_str).unwrap();

	let navbar_str = fs::read_to_string("views/partials/navbar.hbs").unwrap();
	hbs.register_partial("navbar", navbar_str).unwrap();

	hbs.register_template_file("base", "views/layouts/base.hbs")
		.unwrap();

	hbs.register_template_file("blog", "views/layouts/blog.hbs")
		.unwrap();

	let hbs = Arc::new(hbs);

	let root_hbs = hbs.clone();
	let root = warp::path::end().map(move || {
		let html = fs::read_to_string("views/root.hbs").unwrap();
		let render = root_hbs
			.render("base", &BaseLayout::new("Colby Hall | Portfolio", &html))
			.unwrap_or_else(|err| err.to_string());
		warp::reply::html(render)
	});

	let blog_entry_hbs = hbs.clone();
	let blog_entry = warp::path!("blog" / String).map(move |blog| {
		let mut path = PathBuf::from("blogs");
		let blog_file = format!("{}.md", blog);
		path.push(&blog_file);

		let blog = Blog::new(&path).unwrap();

		let base = blog_entry_hbs.render("blog", &BlogLayout{ title: &blog.title, date: &blog.date.to_string(), body: &blog.body }).unwrap_or_else(|err| err.to_string());

		let render = blog_entry_hbs
			.render("base", &BaseLayout::new(&format!("Colby Hall | {}", &blog.title), &base))
			.unwrap_or_else(|err| err.to_string());

		warp::reply::html(render)
	});

	let public = warp::path("public").and(warp::fs::dir("public"));

	let route = warp::get().and(root.or(blog_entry).or(public));

	warp::serve(route).run(([127, 0, 0, 1], 5050)).await;
}
