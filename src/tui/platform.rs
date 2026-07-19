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
    pub fn host() -> Self {
        if cfg!(target_os = "macos") {
            Self {
                opener: "open",
                clipboard: "pbcopy",
                clipboard_args: &[],
            }
        } else {
            Self {
                opener: "xdg-open",
                clipboard: "xclip",
                clipboard_args: &["-selection", "clipboard"],
            }
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
