#![feature(async_closure)]

mod article;

use {
	article::*,
	handlebars::Handlebars,
	serde::Serialize,
	std::{
		collections::HashMap,
		env,
		fs,
		sync::Arc,
	},
	warp::Filter,
};

#[derive(Serialize)]
struct RootLayout<'a> {
	title: String,
	body: &'a str,
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
struct BrowseArticleLayout<'a> {
	title: &'a str,
	link: &'a str,
	read_time: String,
}

#[derive(Serialize)]
struct BrowseArticlesLayout<'a> {
	articles: Vec<BrowseArticleLayout<'a>>,
}

#[tokio::main]
async fn main() {
	// Create Handlebars renderer and then intialize
	let mut hbs = Handlebars::new();

	// Register templates and partials. These will not be reloaded during the runtime.
	let footer_str = fs::read_to_string("views/partials/footer.hbs").unwrap();
	hbs.register_partial("footer", footer_str).unwrap();
	hbs.register_template_file("base", "views/layouts/base.hbs")
		.unwrap();
	hbs.register_template_file("article", "views/layouts/article.hbs")
		.unwrap();
	hbs.register_template_file("articles", "views/layouts/articles.hbs")
		.unwrap();

	// Debug layout allows the server to reload the web pages with every request. This makes it easy to iterate on the actual pages
	let debug_layout = env::args().any(|arg| arg == "-debug_layout");

	if debug_layout {
		// By this point the renderer has been initialized and now needs to be shared across threads for live rendering.
		let hbs = Arc::new(hbs);

		let root = {
			let hbs = hbs.clone();
			warp::path::end().map(move || {
				let html = fs::read_to_string("views/root.hbs").unwrap();
				let render = hbs
					.render(
						"base",
						&RootLayout {
							title: "Colby Hall | Portfolio".to_string(),
							body: &html,
						},
					)
					.unwrap_or_else(|err| err.to_string());
				warp::reply::html(render)
			})
		};

		let browse_articles = {
			let hbs = hbs.clone();
			warp::path!("articles").map(move || {
				// Load and render all articles to gather description information
				let articles: Vec<(String, Article)> = fs::read_dir("articles")
					.unwrap()
					.filter_map(|e| {
						let e = e.unwrap();
						if e.file_type().unwrap().is_file() {
							let path = e.path();
							let article = Article::new(&path).unwrap();

							let name = path.file_stem().unwrap().to_str().unwrap().to_string();
							Some((name, article))
						} else {
							None
						}
					})
					.collect();

				let articles: Vec<BrowseArticleLayout<'_>> = articles
					.iter()
					.map(|article| BrowseArticleLayout {
						title: &article.1.title,
						link: &article.0,
						read_time: format!("{} min read", article.1.read_time),
					})
					.collect();

				let render = hbs
					.render("articles", &BrowseArticlesLayout { articles })
					.unwrap_or_else(|err| err.to_string());

				warp::reply::html(render)
			})
		};

		let article = {
			let hbs = hbs.clone();
			warp::path!("articles" / String)
				.and_then(|article| async move {
					let article = fs::read_dir("articles").unwrap().find_map(|e| {
						let e = e.unwrap();
						if e.file_type().unwrap().is_file() {
							let path = e.path();
							let name = path.file_stem().unwrap().to_str().unwrap().to_string();

							if article == name {
								return Some(Article::new(&path).unwrap());
							}
						}

						None
					});

					match article {
						Some(article) => Ok(article),
						None => Err(warp::reject::not_found()),
					}
				})
				.map(move |article: Article| {
					let render = hbs
						.render(
							"article",
							&ArticleLayout {
								browser_title: &format!("Colby Hall | {}", &article.title),
								title: &article.title,
								date: &article.date.to_string(),
								body: &article.body,
								read_time: &format!("{} min read", article.read_time),
							},
						)
						.unwrap_or_else(|err| err.to_string());

					warp::reply::html(render)
				})
		};

		let public = warp::path("public").and(warp::fs::dir("public"));
		let routes = warp::get().and(root.or(browse_articles).or(article).or(public));

		warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
	} else {
		// Render all pages ahead of time to reduce request work load.
		struct RenderCache {
			root: String,
			browse_articles: String,
			articles: HashMap<String, String>,
		}

		let root = {
			let body = fs::read_to_string("views/root.hbs").unwrap();
			hbs.render(
				"base",
				&RootLayout {
					title: "Colby Hall | Portfolio".to_string(),
					body: &body,
				},
			)
			.unwrap_or_else(|err| err.to_string())
		};

		let mut articles: HashMap<String, Article> = fs::read_dir("articles")
			.unwrap()
			.filter_map(|e| {
				let e = e.unwrap();
				if e.file_type().unwrap().is_file() {
					let path = e.path();
					let name = path.file_stem().unwrap().to_str().unwrap().to_string();
					let article = Article::new(&path).unwrap();
					Some((name, article))
				} else {
					None
				}
			})
			.collect();

		let browse_articles = {
			let articles: Vec<BrowseArticleLayout<'_>> = articles
				.iter()
				.map(|(name, article)| BrowseArticleLayout {
					title: &article.title,
					link: name,
					read_time: format!("{} min read", article.read_time),
				})
				.collect();

			hbs.render("articles", &BrowseArticlesLayout { articles })
				.unwrap_or_else(|err| err.to_string())
		};

		let articles: HashMap<String, String> = articles
			.drain()
			.map(|(name, article)| {
				let render = hbs
					.render(
						"article",
						&ArticleLayout {
							browser_title: &format!("Colby Hall | {}", &article.title),
							title: &article.title,
							date: &article.date.to_string(),
							body: &article.body,
							read_time: &format!("{} min read", article.read_time),
						},
					)
					.unwrap_or_else(|err| err.to_string());

				(name, render)
			})
			.collect();

		let render_cache = Arc::new(RenderCache {
			root,
			browse_articles,
			articles,
		});

		let root = {
			let render_cache = render_cache.clone();
			warp::path::end().map(move || warp::reply::html(render_cache.root.clone()))
		};

		let browse_articles = {
			let render_cache = render_cache.clone();
			warp::path!("articles")
				.map(move || warp::reply::html(render_cache.browse_articles.clone()))
		};

		let article = {
			let render_cache = render_cache.clone();
			warp::path!("articles" / String).map(move |article| {
				let article = render_cache.articles.get(&article);

				match article {
					Some(article) => warp::reply::html(article.clone()),
					None => warp::reply::html(render_cache.browse_articles.clone()),
				}
			})
		};

		let public = warp::path("public").and(warp::fs::dir("public"));
		let routes = warp::get().and(root.or(browse_articles).or(article).or(public));

		warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
	};
}
