use std::io::Write;
use std::path::PathBuf;
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

    pub fn state_dir(&self) -> Option<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from)?;

        #[cfg(target_os = "macos")]
        let base = home.join("Library/Application Support");

        #[cfg(target_os = "linux")]
        let base = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"));

        Some(base.join(crate::APP_NAME))
    }

    #[cfg(target_os = "macos")]
    pub fn migrate_state_dir(&self) {
        let Some(new) = self.state_dir() else {
            return;
        };

        if new.exists() {
            return;
        }

        let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
            return;
        };

        let old = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/state"))
            .join(crate::APP_NAME);

        if old.exists() {
            if let Some(parent) = new.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::rename(&old, &new);
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn migrate_state_dir(&self) {}

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
