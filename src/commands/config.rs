use crate::args::ConfigGetArgs;
use anyhow::{Context, Result, bail};
use firefly_types::Encode;
use std::path::Path;

pub fn cmd_config_get(vfs: &Path, _args: &ConfigGetArgs) -> Result<()> {
    if !vfs.exists() {
        bail!("vfs is not created yet")
    }
    let settings_path = vfs.join("sys").join("config");
    if !settings_path.exists() {
        bail!("settings file not found")
    }
    let raw = std::fs::read(settings_path).context("read settings")?;
    let s = firefly_types::Settings::decode(&raw).context("parse settings")?;
    println!("{{");
    println!(r#"  "xp":       {},"#, s.xp);
    println!(r#"  "badges":   {},"#, s.badges);
    println!(r#"  "country":  "{}","#, p(&s.country));
    println!(r#"  "lang":     "{}","#, p(&s.lang));
    println!(r#"  "name":     "{}","#, s.name);
    println!(r#"  "timezone": "{}","#, s.timezone);

    println!();
    println!(r#"  "auto_lock":         {},"#, s.auto_lock);
    println!(r#"  "font_size":         {},"#, s.font_size);
    println!(r#"  "headphones_volume": {},"#, s.headphones_volume);
    println!(r#"  "leds_brightness":   {},"#, s.leds_brightness);
    println!(r#"  "screen_brightness": {},"#, s.screen_brightness);
    println!(r#"  "speakers_volume":   {},"#, s.speakers_volume);

    println!();
    println!(r#"  "contrast":        {},"#, s.contrast);
    println!(r#"  "easter_eggs":     {},"#, s.easter_eggs);
    println!(r#"  "gamepad_mode":    {},"#, s.gamepad_mode);
    println!(r#"  "reduce_flashing": {},"#, s.reduce_flashing);
    println!(r#"  "rotate_screen":   {},"#, s.rotate_screen);
    println!(r#"  "telemetry":       {}"#, s.telemetry);

    println!("}}");
    Ok(())
}

fn p(r: &[u8]) -> &str {
    str::from_utf8(r).unwrap_or_default()
}
