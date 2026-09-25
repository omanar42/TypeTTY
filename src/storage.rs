use std::{
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::model::{Config, Stats, SCHEMA_VERSION};

pub struct Storage {
    config_path: PathBuf,
    stats_path: PathBuf,
}

impl Storage {
    pub fn discover() -> io::Result<Self> {
        let home_dir = env::var_os("HOME")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        let config_root = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&home_dir).join(".config"));
        let data_root = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&home_dir).join(".local/share"));
        Ok(Self::from_roots(config_root, data_root))
    }

    pub fn from_roots(config_root: PathBuf, data_root: PathBuf) -> Self {
        Self {
            config_path: config_root.join("typetty/config.json"),
            stats_path: data_root.join("typetty/stats.json"),
        }
    }

    pub fn load_config(&self) -> io::Result<Config> {
        let config: Config = self.load_or_default(&self.config_path)?;
        if config.schema_version != SCHEMA_VERSION {
            self.quarantine(&self.config_path)?;
            return Ok(Config::default());
        }
        Ok(config)
    }

    pub fn load_stats(&self) -> io::Result<Stats> {
        let stats: Stats = self.load_or_default(&self.stats_path)?;
        if stats.schema_version != SCHEMA_VERSION {
            self.quarantine(&self.stats_path)?;
            return Ok(Stats::default());
        }
        Ok(stats)
    }

    pub fn save_config(&self, config: &Config) -> io::Result<()> {
        write_json_atomically(&self.config_path, config)
    }

    pub fn save_stats(&self, stats: &Stats) -> io::Result<()> {
        write_json_atomically(&self.stats_path, stats)
    }

    fn load_or_default<T>(&self, path: &Path) -> io::Result<T>
    where
        T: DeserializeOwned + Default,
    {
        let content = match fs::read(path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(T::default()),
            Err(error) => return Err(error),
        };

        match serde_json::from_slice(&content) {
            Ok(value) => Ok(value),
            Err(_) => {
                self.quarantine(path)?;
                Ok(T::default())
            }
        }
    }

    fn quarantine(&self, path: &Path) -> io::Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let file_name = path
            .file_name()
            .map(OsString::from)
            .unwrap_or_else(|| OsString::from("data.json"));
        let mut backup_name = file_name;
        backup_name.push(format!(".corrupt-{timestamp}-{}", std::process::id()));
        fs::rename(path, path.with_file_name(backup_name))
    }
}

fn write_json_atomically<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "storage path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp_path = parent.join(format!(
        ".{}.tmp-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("data"),
        std::process::id()
    ));
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    let mut file = fs::File::create(&temp_path)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(temp_path, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mode;
    use tempfile::tempdir;

    #[test]
    fn round_trips_config_and_stats() {
        let root = tempdir().unwrap();
        let storage = Storage::from_roots(root.path().join("config"), root.path().join("data"));
        let config = Config {
            mode: Mode::Words(50),
            ..Config::default()
        };
        let stats = Stats {
            total_tests: 4,
            ..Stats::default()
        };

        storage.save_config(&config).unwrap();
        storage.save_stats(&stats).unwrap();

        assert_eq!(storage.load_config().unwrap().mode, Mode::Words(50));
        assert_eq!(storage.load_stats().unwrap().total_tests, 4);
    }

    #[test]
    fn quarantines_corrupt_data() {
        let root = tempdir().unwrap();
        let storage = Storage::from_roots(root.path().join("config"), root.path().join("data"));
        fs::create_dir_all(storage.config_path.parent().unwrap()).unwrap();
        fs::write(&storage.config_path, b"not json").unwrap();

        let config = storage.load_config().unwrap();
        assert_eq!(config.mode, Mode::Timed(30));
        assert!(!storage.config_path.exists());
        let backups = fs::read_dir(storage.config_path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .count();
        assert_eq!(backups, 1);
    }
}
