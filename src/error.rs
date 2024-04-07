use colored::Colorize;
use std::fmt::Display;

pub(crate) enum CLIError {
    IO(std::io::Error),
    Image(image::ImageError),
    Toml(toml::de::Error),
    TooManyColors,
    LangNotDetected,
    FileExtNotDetected(String),
    UnknownFileExt(String),
    Subprocess(i32),
}

impl From<std::io::Error> for CLIError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<image::ImageError> for CLIError {
    fn from(value: image::ImageError) -> Self {
        Self::Image(value)
    }
}

impl From<toml::de::Error> for CLIError {
    fn from(value: toml::de::Error) -> Self {
        Self::Toml(value)
    }
}

impl CLIError {
    pub fn get_code(&self) -> i32 {
        match self {
            CLIError::IO(_) => 2,
            CLIError::LangNotDetected => 3,
            CLIError::Image(_) => 4,
            CLIError::TooManyColors => 5,
            CLIError::Toml(_) => 6,
            CLIError::FileExtNotDetected(_) => 7,
            CLIError::UnknownFileExt(_) => 8,
            CLIError::Subprocess(_) => 9,
        }
    }

    pub fn exit(&self) -> ! {
        eprintln!("{} {}", "💥 Error:".red(), self);
        let code = self.get_code();
        std::process::exit(code);
    }
}

impl Display for CLIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CLIError::*;
        match self {
            IO(err) => write!(f, "IO error: {err}"),
            Image(err) => write!(f, "image error: {err}"),
            Toml(err) => write!(f, "toml deserialization error: {err}"),
            TooManyColors => write!(f, "the image contains more than 4 colors"),
            LangNotDetected => write!(f, "cannot detect programming language"),
            FileExtNotDetected(fname) => write!(f, "cannot detect file extension for {fname}"),
            UnknownFileExt(ext) => write!(f, "unsupported file type: {ext}"),
            Subprocess(code) => write!(f, "subprocess exited with status code {code}"),
        }
    }
}
