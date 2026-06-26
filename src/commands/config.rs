use crate::args::ConfigGetArgs;
use anyhow::{Context, Result, bail};
use firefly_types::Encode;
use std::path::Path;

pub fn cmd_config_get(vfs: &Path, args: &ConfigGetArgs) -> Result<()> {
    if !vfs.exists() {
        bail!("vfs is not created yet")
    }
    let settings_path = vfs.join("sys").join("config");
    if !settings_path.exists() {
        bail!("settings file not found")
    }
    let raw = std::fs::read(settings_path).context("read settings")?;
    let s = firefly_types::Settings::decode(&raw).context("parse settings")?;

    if let Some(key) = &args.key {
        match key.as_str() {
            "xp" => println!("{}", s.xp),
            "badges" => println!("{}", s.badges),
            "country" => println!("{}", p(&s.country)),
            "lang" => println!("{}", p(&s.lang)),
            "name" => println!("{}", s.name),
            "timezone" => println!("{}", s.timezone),
            "auto_lock" => println!("{}", s.auto_lock),
            "font_size" => println!("{}", s.font_size),
            "headphones_volume" => println!("{}", s.headphones_volume),
            "leds_brightness" => println!("{}", s.leds_brightness),
            "screen_brightness" => println!("{}", s.screen_brightness),
            "speakers_volume" => println!("{}", s.speakers_volume),
            "contrast" => println!("{}", s.contrast),
            "easter_eggs" => println!("{}", s.easter_eggs),
            "gamepad_mode" => println!("{}", s.gamepad_mode),
            "reduce_flashing" => println!("{}", s.reduce_flashing),
            "rotate_screen" => println!("{}", s.rotate_screen),
            "telemetry" => println!("{}", s.telemetry),
            _ => bail!("unsupported key"),
        }
        return Ok(());
    }

    println!("{{");
    println!(r#"  "xp":       {},"#, s.xp);
    println!(r#"  "badges":   {},"#, s.badges);
    println!(r#"  "country":  "{}","#, p(&s.country));
    println!(r#"  "lang":     "{}","#, p(&s.lang));
    println!(r#"  "name":     "{}","#, s.name);
    println!(r#"  "timezone": "{}","#, s.timezone);

    println!();
    println!(r#"  "auto_lock":         {:>3},"#, s.auto_lock);
    println!(r#"  "font_size":         {:>3},"#, s.font_size);
    println!(r#"  "headphones_volume": {:>3},"#, s.headphones_volume);
    println!(r#"  "leds_brightness":   {:>3},"#, s.leds_brightness);
    println!(r#"  "screen_brightness": {:>3},"#, s.screen_brightness);
    println!(r#"  "speakers_volume":   {:>3},"#, s.speakers_volume);

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
