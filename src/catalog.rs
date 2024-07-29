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
}

pub fn cmd_catalog_list(_args: &CatalogListArgs) -> Result<()> {
    let resp = ureq::get(LIST_URL).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("cannot connect ot the catalog")
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
    let url = format!("{BASE_URL}{}.json", args.id);
    let resp = ureq::get(&url).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("cannot connect ot the catalog")
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

fn col(name: &str) -> String {
    format!("{name:11}").blue().to_string()
}
