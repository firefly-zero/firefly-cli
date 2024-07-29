use crate::args::{CatalogListArgs, CatalogShowArgs};
use anyhow::{bail, Context, Result};
use crossterm::style::Stylize;
use serde::Deserialize;
use std::collections::HashMap;

const BASE_URL: &str = "https://catalog.fireflyzero.com/";
const LIST_URL: &str = "https://catalog.fireflyzero.com/apps.json";

#[derive(Deserialize)]
struct ShortApp {
    id: String,
    name: String,
    author: String,
    short: String,
    added: String,
}

#[derive(Deserialize)]
struct App {
    name: String,
    author: Author,
    short: String,
    added: String,
    download: String,
    desc: String,
    links: Option<HashMap<String, String>>,
    categories: Vec<String>,
}

#[derive(Deserialize)]
struct Author {
    name: String,
    pronouns: Option<String>,
    links: HashMap<String, String>,
    short: String,
    about: Option<String>,
}

pub fn cmd_catalog_list(_args: &CatalogListArgs) -> Result<()> {
    let resp = ureq::get(LIST_URL).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("cannot connect to the catalog")
    }
    let apps: Vec<ShortApp> =
        serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
    let id_width = apps.iter().map(|app| app.id.len()).max().unwrap();
    for app in apps {
        println!(
            "{} | {:5$} | {} by {}: {}",
            app.added,
            app.id,
            app.name.blue(),
            app.author.cyan(),
            app.short,
            id_width,
        );
    }
    Ok(())
}

pub fn cmd_catalog_show(args: &CatalogShowArgs) -> Result<()> {
    if args.id.contains('.') {
        show_app(args)
    } else {
        show_author(args)
    }
}

pub fn show_app(args: &CatalogShowArgs) -> Result<()> {
    let url = format!("{BASE_URL}{}.json", args.id);
    let resp = ureq::get(&url).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("the app not found")
    }
    let app: App = serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
    println!("{} {}", col("title"), app.name);
    println!("{} {}", col("author"), app.author.name);
    println!("{} {}", col("added"), app.added);
    println!("{} {}", col("short"), app.short);
    println!("{}", col("categories"));
    for category in app.categories {
        println!("  {category}");
    }
    if let Some(links) = app.links {
        println!("{}", col("links"));
        for (name, url) in links {
            println!("  {}: {}", name.cyan(), url);
        }
    }
    println!("{} {}", col("download"), app.download);
    println!("{}\n{}", col("description"), app.desc);
    Ok(())
}

pub fn show_author(args: &CatalogShowArgs) -> Result<()> {
    let url = format!("{BASE_URL}{}.json", args.id);
    let resp = ureq::get(&url).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("the author not found")
    }
    let aut: Author = serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
    println!("{} {}", col("name"), aut.name);
    if let Some(pronouns) = aut.pronouns {
        println!("{} {}", col("pronouns"), pronouns);
    }
    println!("{} {}", col("short"), aut.short);
    if !aut.links.is_empty() {
        println!("{}", col("links"));
        for (name, url) in aut.links {
            println!("  {}: {}", name.cyan(), url);
        }
    }
    if let Some(about) = aut.about {
        println!("{}\n{}", col("about"), about);
    }
    Ok(())
}

fn col(name: &str) -> String {
    format!("{name:11}").blue().to_string()
}
