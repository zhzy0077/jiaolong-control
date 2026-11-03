use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn is_dark(self, system_dark: bool) -> bool {
        match self {
            Self::System => system_dark,
            Self::Light => false,
            Self::Dark => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    ZhCn,
    En,
}

impl Language {
    pub fn text(self, chinese: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::ZhCn => chinese,
            Self::En => english,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub language: Language,
}

impl Preferences {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("Read application preferences"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).context("Read application preferences"),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        let result = (|| -> Result<()> {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&serde_json::to_vec_pretty(self)?)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temporary);
        }
        result.context("Save application preferences")
    }
}

pub fn preferences_path() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    base.unwrap_or_else(|| std::env::temp_dir().join("jiaolong-user-config"))
        .join("jiaolong-control")
        .join("preferences.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_follows_system_only_when_selected() {
        for system in [false, true] {
            assert_eq!(Theme::System.is_dark(system), system);
            assert!(!Theme::Light.is_dark(system));
            assert!(Theme::Dark.is_dark(system));
        }
    }

    #[test]
    fn preferences_roundtrip_and_defaults() {
        let directory = std::env::temp_dir().join(format!("jiaolong-prefs-{}", std::process::id()));
        let path = directory.join("preferences.json");
        let _ = std::fs::remove_dir_all(&directory);
        assert_eq!(Preferences::load(&path).unwrap(), Preferences::default());
        let preferences = Preferences {
            theme: Theme::Dark,
            language: Language::En,
        };
        preferences.save(&path).unwrap();
        assert_eq!(Preferences::load(&path).unwrap(), preferences);
        Preferences::default().save(&path).unwrap();
        assert_eq!(Preferences::load(&path).unwrap(), Preferences::default());
        std::fs::write(&path, r#"{"theme":"light"}"#).unwrap();
        assert_eq!(Preferences::load(&path).unwrap().language, Language::ZhCn);
        std::fs::write(&path, r#"{"theme":"unknown"}"#).unwrap();
        assert!(Preferences::load(&path).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
