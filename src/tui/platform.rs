use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy)]
pub struct Platform {
    opener: &'static str,
    clipboard: &'static str,
    clipboard_args: &'static [&'static str],
}

impl Platform {
    #[cfg(target_os = "macos")]
    pub fn host() -> Self {
        Self {
            opener: "open",
            clipboard: "pbcopy",
            clipboard_args: &[],
        }
    }

    #[cfg(target_os = "linux")]
    pub fn host() -> Self {
        Self {
            opener: "xdg-open",
            clipboard: "xclip",
            clipboard_args: &["-selection", "clipboard"],
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn host() -> Self {
        compile_error!("linear-tui supports macOS and Linux only")
    }

    #[cfg(test)]
    pub fn inert() -> Self {
        Self {
            opener: "true",
            clipboard: "true",
            clipboard_args: &[],
        }
    }

    #[cfg(test)]
    pub fn broken() -> Self {
        Self {
            opener: "linear-tui-no-such-opener",
            clipboard: "linear-tui-no-such-clipboard",
            clipboard_args: &[],
        }
    }

    pub fn open_url(&self, url: &str) -> Result<()> {
        Command::new(self.opener)
            .arg(url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to launch {}", self.opener))?;

        Ok(())
    }

    pub fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        let mut child = Command::new(self.clipboard)
            .args(self.clipboard_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to launch {}", self.clipboard))?;

        child
            .stdin
            .take()
            .context("clipboard stdin unavailable")?
            .write_all(text.as_bytes())?;
        child.wait()?;

        Ok(())
    }
}
