use anyhow::{Context, Result, bail};
use hound::{SampleFormat, WavReader};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn convert_wav(input_path: &Path, output_path: &Path) -> Result<()> {
    let mut reader = WavReader::open(input_path).context("open wav file")?;

    // Get and validate spec
    let spec = reader.spec();
    if spec.channels > 2 {
        bail!("wav files must have 1 or 2 channels, not {}", spec.channels)
    }
    let stereo = spec.channels > 1;
    let Ok(sample_rate) = u16::try_from(spec.sample_rate) else {
        bail!("sample rate is too high: {}", spec.sample_rate);
    };
    if sample_rate != 44_100 {
        bail!("sample rate must be 44100 Hz, got {} Hz", spec.sample_rate);
    }
    let bits = spec.bits_per_sample;

    // Write header
    let mut out = File::create(output_path).context("create output path")?;
    write_u8(&mut out, 0x31)?;
    let format = u8::from(stereo);
    let format = (format << 1) | u8::from(bits > 8);
    let format = format << 1; // last bit is reserved for ADPCM
    write_u8(&mut out, format)?;
    write_u16(&mut out, sample_rate)?;

    match (spec.sample_format, bits) {
        (SampleFormat::Int, 8) => {
            let samples = reader.samples::<i8>();
            for sample in samples {
                let sample = sample?;
                write_i8(&mut out, sample)?;
            }
        }
        (SampleFormat::Int, 16) => {
            let samples = reader.samples::<i16>();
            for sample in samples {
                let sample = sample?;
                write_i16(&mut out, sample)?;
            }
        }
        (SampleFormat::Float, 32) => {
            let samples = reader.samples::<f32>();
            for sample in samples {
                let sample = sample?;
                #[expect(clippy::cast_possible_truncation)]
                let sample = (f32::from(i16::MAX) * sample) as i16;
                write_i16(&mut out, sample)?;
            }
        }
        _ => {
            let letter = if spec.sample_format == SampleFormat::Float {
                "f"
            } else {
                "i"
            };
            bail!("unsupported sample format: {letter}{bits}",);
        }
    }
    Ok(())
}

fn write_u8(f: &mut File, v: u8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_u16(f: &mut File, v: u16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_i8(f: &mut File, v: i8) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}

fn write_i16(f: &mut File, v: i16) -> std::io::Result<()> {
    f.write_all(&v.to_le_bytes())
}
