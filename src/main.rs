#![feature(async_closure)]

use {
	handlebars::Handlebars,
	serde::Serialize,
	std::{
		collections::HashMap,
		fs,
		path::PathBuf,
		sync::Arc,
	},
	warp::Filter,
};

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

struct GlobalState {
	blogs: HashMap<String, Blog>,
}

impl GlobalState {
	fn get() -> &'static GlobalState {
		unsafe { GLOBAL.as_ref().unwrap() }
	}
}

static mut GLOBAL: Option<GlobalState> = None;

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

	let mut blogs = HashMap::new();
	for e in fs::read_dir("blogs").unwrap() {
		let e = e.unwrap();
		if e.file_type().unwrap().is_file() {
			let path = e.path();
			let blog = Blog::new(&path).unwrap();
			blogs.insert(
				path.file_stem().unwrap().to_str().unwrap().to_string(),
				blog,
			);
		}
	}

	let global_state = GlobalState { blogs };
	unsafe { GLOBAL = Some(global_state) };

	let blog_entry_hbs = hbs.clone();
	// let blog_global_state = global_state.clone();
	let blog_entry = warp::path!("blog" / String)
		.and_then(|blog| async move {
			match GlobalState::get().blogs.get(&blog) {
				Some(blog) => Ok(blog),
				None => Err(warp::reject::not_found()),
			}
		})
		.map(move |blog: &Blog| {
			let base = blog_entry_hbs
				.render(
					"blog",
					&BlogLayout {
						title: &blog.title,
						date: &blog.date.to_string(),
						body: &blog.body,
					},
				)
				.unwrap_or_else(|err| err.to_string());

			let render = blog_entry_hbs
				.render(
					"base",
					&BaseLayout::new(&format!("Colby Hall | {}", &blog.title), &base),
				)
				.unwrap_or_else(|err| err.to_string());

			warp::reply::html(render)
		});

	let public = warp::path("public").and(warp::fs::dir("public"));

	let routes = warp::get().and(root.or(blog_entry).or(public));

	warp::serve(routes).run(([127, 0, 0, 1], 5050)).await;
}
