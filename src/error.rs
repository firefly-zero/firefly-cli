use colored::Colorize;

pub(crate) enum CLIError {
    /// A custom error.
    /// Printing is already handled by the command, just exit.
    Exit,
}

impl CLIError {
    pub fn get_code(&self) -> i32 {
        match &self {
            CLIError::Exit => 1,
        }
    }

    fn get_wrapped(&self) -> &dyn std::error::Error {
        match &self {
            CLIError::Exit => unreachable!(),
        }
    }

    pub fn exit(&self) -> ! {
        if !matches!(self, CLIError::Exit) {
            let err = self.get_wrapped();
            eprintln!("{}\n{}", "💥 Error:".red(), err);
        }
        let code = self.get_code();
        std::process::exit(code);
    }
}
