use crate::protocol::Mode;
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::{Path, PathBuf};

pub struct Settings {
    path: PathBuf,
    rows: Vec<String>,
}

impl Settings {
    pub fn open(path: PathBuf) -> Result<Self> {
        Self::open_with_legacy(path, None)
    }
    pub fn open_with_legacy(path: PathBuf, legacy: Option<&Path>) -> Result<Self> {
        let source = if path.exists() {
            Some(path.as_path())
        } else {
            legacy.filter(|p| p.is_file())
        };
        let mut rows: Vec<String> = if let Some(source) = source {
            std::fs::read_to_string(source)?
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
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
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
    fn user_config_imports_legacy_without_overwriting_it() {
        let directory =
            std::env::temp_dir().join(format!("jiaolong-migration-{}", std::process::id()));
        let legacy = directory.join("legacy/AppMachineInfo");
        let path = directory.join("config/jiaolong-control/AppMachineInfo");
        let mut old = Settings::open(legacy.clone()).unwrap();
        old.save(Mode::Balanced, 47).unwrap();
        let mut migrated = Settings::open_with_legacy(path.clone(), Some(&legacy)).unwrap();
        assert_eq!(migrated.speed(Mode::Balanced).unwrap(), 47);
        migrated.save(Mode::Balanced, 48).unwrap();
        assert_eq!(
            Settings::open(legacy.clone())
                .unwrap()
                .speed(Mode::Balanced)
                .unwrap(),
            47
        );
        assert_eq!(
            Settings::open_with_legacy(path.clone(), Some(&legacy))
                .unwrap()
                .speed(Mode::Balanced)
                .unwrap(),
            48
        );
        #[cfg(windows)]
        for file in [&legacy, &path] {
            let mut permissions = std::fs::metadata(file).unwrap().permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(file, permissions).unwrap();
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
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
