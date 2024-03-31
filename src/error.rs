use colored::Colorize;
use std::fmt::Display;

pub(crate) enum CLIError {
    /// A custom error.
    /// Printing is already handled by the command, just exit.
    Exit,
    IO(std::io::Error),
    LangNotDetected,
}

impl From<std::io::Error> for CLIError {
    fn from(value: std::io::Error) -> Self {
        Self::IO(value)
    }
}

impl CLIError {
    pub fn get_code(&self) -> i32 {
        match self {
            CLIError::Exit => 1,
            CLIError::IO(_) => 2,
            CLIError::LangNotDetected => 3,
        }
    }

    pub fn exit(&self) -> ! {
        if !matches!(self, CLIError::Exit) {
            eprintln!("{}\n{}", "💥 Error:".red(), self);
        }
        let code = self.get_code();
        std::process::exit(code);
    }
}

impl Display for CLIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CLIError::*;
        match self {
            Exit => Ok(()),
            IO(err) => write!(f, "{err}"),
            LangNotDetected => write!(f, "cannot detect programming language"),
        }
    }
}
