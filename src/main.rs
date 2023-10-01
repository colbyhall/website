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

mod article;
use article::*;

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
struct ArticleLayout<'a> {
	browser_title: &'a str,
	title: &'a str,
	date: &'a str,
	body: &'a str,
	read_time: &'a str,
}

#[derive(Serialize)]
struct ArticleEntry<'a> {
	title: &'a str,
	link: &'a str,
	read_time: String
}

#[derive(Serialize)]
struct ArticlesLayout<'a> {
	articles: Vec<ArticleEntry<'a>>,
}

struct GlobalState {
	articles: HashMap<String, Article>,
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

	hbs.register_template_file("base", "views/layouts/base.hbs")
		.unwrap();

	hbs.register_template_file("article", "views/layouts/article.hbs")
		.unwrap();

		
	hbs.register_template_file("articles", "views/layouts/articles.hbs")
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

	let mut articles = HashMap::new();
	for e in fs::read_dir("articles").unwrap() {
		let e = e.unwrap();
		if e.file_type().unwrap().is_file() {
			let path = e.path();
			let article = Article::new(&path).unwrap();
			articles.insert(
				path.file_stem().unwrap().to_str().unwrap().to_string(),
				article,
			);
		}
	}

	let global_state = GlobalState { articles };
	unsafe { GLOBAL = Some(global_state) };

	let articles_entry_hbs = hbs.clone();
	let articles_entry = warp::path!("articles")
		.map(move || {
			let mut articles = Vec::with_capacity(GlobalState::get().articles.len());
			for (key, value) in GlobalState::get().articles.iter() {
				articles.push(ArticleEntry {
					title: &value.title,
					link: key,
					read_time: format!("{} min read", value.read_time)
				});
			}

			let render = articles_entry_hbs
				.render(
					"articles",
					&ArticlesLayout {
						articles
					},
				)
				.unwrap_or_else(|err| err.to_string());

			warp::reply::html(render)
		});

	let article_entry_hbs = hbs.clone();
	let article_entry = warp::path!("articles" / String)
		.and_then(|article| async move {
			match GlobalState::get().articles.get(&article) {
				Some(article) => Ok(article),
				None => Err(warp::reject::not_found()),
			}
		})
		.map(move |article: &Article| {
			let render = article_entry_hbs
				.render(
					"article",
					&ArticleLayout {
						browser_title: &format!("Colby Hall | {}", &article.title),
						title: &article.title,
						date: &article.date.to_string(),
						body: &article.body,
						read_time: &format!("{} min read", article.read_time)
					},
				)
				.unwrap_or_else(|err| err.to_string());

			warp::reply::html(render)
		});

	let public = warp::path("public").and(warp::fs::dir("public"));

	let routes = warp::get().and(root.or(articles_entry).or(article_entry).or(public));

	warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}
