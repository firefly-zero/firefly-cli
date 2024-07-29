use crate::args::{CatalogListArgs, CatalogShowArgs};
use anyhow::{bail, Context, Result};
use serde::Deserialize;

const LIST_URL: &str = "https://catalog.fireflyzero.com/apps.json";

#[derive(Deserialize)]
struct ShortApp {
    id: String,
    name: String,
    // author: String,
    short: String,
    // added: String,
}

pub fn cmd_catalog_list(_args: &CatalogListArgs) -> Result<()> {
    let resp = ureq::get(LIST_URL).call().context("send request")?;
    if resp.status() != 200 || resp.header("Content-Type") != Some("application/json") {
        bail!("cannot connect ot the catalog")
    }
    let apps: Vec<ShortApp> =
        serde_json::from_reader(&mut resp.into_reader()).context("parse JSON")?;
    for app in apps {
        println!("{} ({}): {}", app.name, app.id, app.short);
    }
    Ok(())
}

pub fn cmd_catalog_show(_args: &CatalogShowArgs) -> Result<()> {
    todo!()
}
