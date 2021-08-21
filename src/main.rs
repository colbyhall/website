use warp::Filter;
use handlebars::Handlebars;
use serde::Serialize;

use std::path::PathBuf;
use std::sync::Arc;
use std::fs;

use std::collections::HashMap;

mod blog;
use blog::*;

#[derive(Serialize)]
struct MainLayout<'a> {
    title: String,
    body: &'a str,
}

impl<'a> MainLayout<'a> {
    fn new(title: impl ToString, body: &'a str) -> Self {
        Self{
            title: title.to_string(),
            body,
        }
    }
}

#[tokio::main]
async fn main() {
    let mut hbs = Handlebars::new();

    let footer_str = fs::read_to_string("views/partials/footer.hbs").unwrap();
    hbs.register_partial("footer", footer_str).unwrap();
    hbs.register_template_file("page", "views/layouts/page.hbs").unwrap();

    let root_html = fs::read_to_string("views/root.hbs").unwrap();

    let mut blogs = HashMap::new();
    for entry in fs::read_dir("blogs").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let blog = Blog::new(&path).unwrap();
            blogs.insert(name, blog);
        }
    }

    let hbs = Arc::new(hbs);

    let root_hbs = hbs.clone();
    let root = warp::path::end().map(move ||{
        let render = root_hbs.render("page", &MainLayout::new("Root", &root_html)).unwrap_or_else(|err| err.to_string());
        warp::reply::html(render)
    });

    let blog_entry_hbs = hbs.clone();
    let blog_entry = warp::path!("blog" / String).map(move |blog| {
        let mut path = PathBuf::from("blogs");
        let blog_file = format!("{}.md", blog);
        path.push(&blog_file);
        
        let blog = Blog::new(&path).unwrap();
        let render = blog_entry_hbs.render("page", &MainLayout::new(&blog.title, &blog.body)).unwrap_or_else(|err| err.to_string());
        warp::reply::html(render)
    });

    let public = warp::path("public").and(warp::fs::dir("public"));

    let route = warp::get().and(root.or(blog_entry).or(public));

    warp::serve(route).run(([127, 0, 0, 1], 8080)).await;
}
