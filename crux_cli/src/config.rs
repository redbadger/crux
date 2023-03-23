use termcolor::{ColorChoice, StandardStream};

#[allow(dead_code)]
pub struct GlobalConfig {
    level: Option<log::Level>,
    is_stderr_tty: bool,
    stdout: StandardStream,
    stderr: StandardStream,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalConfig {
    pub fn new() -> Self {
        let is_stdout_tty = atty::is(atty::Stream::Stdout);
        let is_stderr_tty = atty::is(atty::Stream::Stderr);

        let color_choice = match std::env::var("CARGO_TERM_COLOR").as_deref() {
            Ok("always") => Some(ColorChoice::Always),
            Ok("alwaysansi") => Some(ColorChoice::AlwaysAnsi),
            Ok("auto") => Some(ColorChoice::Auto),
            Ok("never") => Some(ColorChoice::Never),
            Ok(_) | Err(..) => None,
        };

        Self {
            level: None,
            is_stderr_tty,
            stdout: StandardStream::stdout(color_choice.unwrap_or({
                if is_stdout_tty {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            })),
            stderr: StandardStream::stderr(color_choice.unwrap_or({
                if is_stderr_tty {
                    ColorChoice::Auto
                } else {
                    ColorChoice::Never
                }
            })),
        }
    }

    pub fn set_level(mut self, level: Option<log::Level>) -> Self {
        self.level = level;
        self
    }

    pub fn is_verbose(&self) -> bool {
        log::Level::Debug <= self.level.unwrap_or(log::Level::Error)
    }

    pub fn verbose(
        &mut self,
        callback: impl Fn(&mut Self) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        if self.is_verbose() {
            callback(self)?;
        }
        Ok(())
    }

    pub fn is_extra_verbose(&self) -> bool {
        log::Level::Trace <= self.level.unwrap_or(log::Level::Error)
    }

    pub fn extra_verbose(
        &mut self,
        callback: impl Fn(&mut Self) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        if self.is_extra_verbose() {
            callback(self)?;
        }
        Ok(())
    }

    pub fn is_stderr_tty(&self) -> bool {
        self.is_stderr_tty
    }

    pub fn stdout(&mut self) -> &mut StandardStream {
        &mut self.stdout
    }

    pub fn stderr(&mut self) -> &mut StandardStream {
        &mut self.stderr
    }

    /// Print a message with a colored title in the style of Cargo shell messages.
    pub fn shell_print(
        &mut self,
        status: impl std::fmt::Display,
        message: impl std::fmt::Display,
        color: termcolor::Color,
        justified: bool,
    ) -> anyhow::Result<()> {
        use std::io::Write;
        use termcolor::WriteColor;

        self.stderr().set_color(
            termcolor::ColorSpec::new()
                .set_fg(Some(color))
                .set_bold(true),
        )?;
        if justified {
            write!(self.stderr(), "{status:>12}")?;
        } else {
            write!(self.stderr(), "{status}")?;
            self.stderr()
                .set_color(termcolor::ColorSpec::new().set_bold(true))?;
            write!(self.stderr(), ":")?;
        }
        self.stderr().reset()?;

        writeln!(self.stderr(), " {message}")?;

        Ok(())
    }

    /// Print a styled action message.
    pub fn shell_status(
        &mut self,
        action: impl std::fmt::Display,
        message: impl std::fmt::Display,
    ) -> anyhow::Result<()> {
        self.shell_print(action, message, termcolor::Color::Green, true)
    }

    pub fn shell_note(&mut self, message: impl std::fmt::Display) -> anyhow::Result<()> {
        self.shell_print("note", message, termcolor::Color::Cyan, false)
    }

    pub fn shell_warn(&mut self, message: impl std::fmt::Display) -> anyhow::Result<()> {
        self.shell_print("warning", message, termcolor::Color::Yellow, false)
    }
}
