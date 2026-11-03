use crate::protocol::Mode;
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::PathBuf;

pub struct Settings {
    path: PathBuf,
    rows: Vec<String>,
}

impl Settings {
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut rows: Vec<String> = if path.exists() {
            std::fs::read_to_string(&path)?
                .lines()
                .map(str::to_owned)
                .collect()
        } else {
            Vec::new()
        };
        if rows.first() != Some(&STANDARD.encode("0.3.12.0")) {
            rows = ["0.3.12.0", "53", "43", "27", "0", "99", "101", "105", "0"]
                .map(|s| STANDARD.encode(s))
                .to_vec();
        }
        ensure!(
            rows.len() >= 9,
            "AppMachineInfo 需要至少 9 行，当前 {} 行",
            rows.len()
        );
        let settings = Self { path, rows };
        for mode in Mode::ALL {
            settings.speed(mode)?;
        }
        Ok(settings)
    }
    pub fn speed(&self, mode: Mode) -> Result<u8> {
        String::from_utf8(STANDARD.decode(&self.rows[mode.index() + 1])?)?
            .parse()
            .context("AppMachineInfo 风扇值解析")
    }
    pub fn save(&mut self, mode: Mode, speed: u8) -> Result<()> {
        self.rows[mode.index() + 1] = STANDARD.encode(speed.to_string());
        if self.path.exists() {
            #[cfg(windows)]
            {
                let mut permissions = std::fs::metadata(&self.path)?.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(&self.path, permissions)?;
            }
        }
        std::fs::write(&self.path, self.rows.join("\r\n") + "\r\n")?;
        #[cfg(windows)]
        {
            let mut permissions = std::fs::metadata(&self.path)?.permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&self.path, permissions)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_file_format_preserves_other_settings() {
        let path = std::env::temp_dir().join(format!("jiaolong-settings-{}", std::process::id()));
        let mut settings = Settings::open(path.clone()).unwrap();
        settings.rows[7] = STANDARD.encode("115");
        settings.save(Mode::Balanced, 48).unwrap();
        let read = Settings::open(path.clone()).unwrap();
        assert_eq!(read.speed(Mode::Performance).unwrap(), 53);
        assert_eq!(read.speed(Mode::Balanced).unwrap(), 48);
        assert_eq!(read.rows[7], STANDARD.encode("115"));
        #[cfg(windows)]
        {
            let mut p = std::fs::metadata(&path).unwrap().permissions();
            p.set_readonly(false);
            std::fs::set_permissions(&path, p).unwrap();
        }
        std::fs::remove_file(path).unwrap();
    }
}
