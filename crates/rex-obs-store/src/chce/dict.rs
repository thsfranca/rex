use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::error::ObsStoreError;

/// Ordinal `0` is reserved for null / empty categorical values.
pub const NULL_ORDINAL: u16 = 0;

/// Global string dictionary persisted as `store.dict`.
pub struct DictionaryManager {
    path: PathBuf,
    next_ordinal: u16,
    ordinal_to_string: Vec<String>,
    string_to_ordinal: HashMap<String, u16>,
}

impl DictionaryManager {
    pub fn open(path: PathBuf) -> Result<Self, ObsStoreError> {
        let mut manager = Self {
            path,
            next_ordinal: 1,
            ordinal_to_string: vec![String::new()],
            string_to_ordinal: HashMap::new(),
        };
        manager.load()?;
        Ok(manager)
    }

    pub fn ordinal(&mut self, value: &str) -> Result<u16, ObsStoreError> {
        if value.is_empty() {
            return Ok(NULL_ORDINAL);
        }
        if let Some(&ordinal) = self.string_to_ordinal.get(value) {
            return Ok(ordinal);
        }
        if self.next_ordinal == u16::MAX {
            return Err(ObsStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "CHCE dictionary exhausted",
            )));
        }
        let ordinal = self.next_ordinal;
        self.next_ordinal += 1;
        self.string_to_ordinal.insert(value.to_string(), ordinal);
        self.ordinal_to_string.push(value.to_string());
        Ok(ordinal)
    }

    pub fn lookup(&self, ordinal: u16) -> Option<&str> {
        self.ordinal_to_string
            .get(ordinal as usize)
            .map(String::as_str)
    }

    pub fn persist(&self) -> Result<(), ObsStoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        let count = (self.ordinal_to_string.len() as u32).saturating_sub(1);
        file.write_all(&count.to_le_bytes())?;
        for ordinal in 1..self.ordinal_to_string.len() {
            let value = &self.ordinal_to_string[ordinal];
            let len = value.len() as u32;
            file.write_all(&len.to_le_bytes())?;
            file.write_all(value.as_bytes())?;
        }
        file.sync_data()?;
        Ok(())
    }

    fn load(&mut self) -> Result<(), ObsStoreError> {
        if !self.path.exists() {
            return Ok(());
        }
        let mut file = File::open(&self.path)?;
        let mut count_buf = [0_u8; 4];
        if file.read_exact(&mut count_buf).is_err() {
            return Ok(());
        }
        let count = u32::from_le_bytes(count_buf) as usize;
        for ordinal in 1..=count {
            let mut len_buf = [0_u8; 4];
            file.read_exact(&mut len_buf)?;
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut value = vec![0_u8; len];
            file.read_exact(&mut value)?;
            let value = String::from_utf8(value).map_err(|_| {
                ObsStoreError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid UTF-8 in store.dict",
                ))
            })?;
            self.string_to_ordinal.insert(value.clone(), ordinal as u16);
            if self.ordinal_to_string.len() <= ordinal {
                self.ordinal_to_string.resize(ordinal + 1, String::new());
            }
            self.ordinal_to_string[ordinal] = value;
            self.next_ordinal = self.next_ordinal.max((ordinal as u16) + 1);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn dict_round_trip_persists_ordinals() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("store.dict");
        let mut dict = DictionaryManager::open(path.clone()).unwrap();
        assert_eq!(dict.ordinal("done").unwrap(), 1);
        assert_eq!(dict.ordinal("done").unwrap(), 1);
        assert_eq!(dict.ordinal("grpc_error").unwrap(), 2);
        dict.persist().unwrap();

        let reloaded = DictionaryManager::open(path).unwrap();
        assert_eq!(reloaded.lookup(1), Some("done"));
        assert_eq!(reloaded.lookup(2), Some("grpc_error"));
    }
}
