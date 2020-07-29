use std::path::{Path, PathBuf};
use std::fs::create_dir_all;

use tantivy::directory::MmapDirectory;
use tantivy::{Index, ReloadPolicy, IndexWriter, IndexReader};

use crate::prelude::*;
use tantivy::schema::Schema;


/// Resolve home
pub(crate) fn resolve_home<T: AsRef<str>>(home: Option<T>) -> Result<PathBuf, IndexError> {
    let home = match &home {
        Some(h) => h.as_ref(),
        None => "indexes"
    };
    let home = Path::new(home);
    let _ = create_dir_all(home)?;
    Ok(home.to_owned())
}

/// Resolve Index
pub(crate) fn resolve_index_directory_path<T: AsRef<str>>(name: T, home: Option<T>) -> Result<PathBuf, IndexError> {
    let home = resolve_home(home)?;
    let path = home.join(name.as_ref());
    Ok(path)
}


/// Create a MMap dir
pub(crate) fn open_mmap_directory(path: PathBuf) -> Result<MmapDirectory, IndexError> {
    if !path.exists() {
        let _ = create_dir_all(&path)?;
    }
    let dir = MmapDirectory::open(path)?;
    Ok(dir)
}


/// Open a store or create & open using a schema
pub(crate) fn open_index(dir: MmapDirectory, schema: Option<&Schema>) -> Result<Index, IndexError> {
    let index = if Index::exists(&dir) {
        Index::open(dir)
    } else {
        if let None = schema {
            let error = IndexError::new(
                "Unable to create index",
                "Schema is required for new index",
            );
            return Err(error);
        }
        let schema = schema.unwrap();
        Index::create(dir, schema.clone())
    }?;

    Ok(index)
}

/// Convenience method to open writer
pub(crate) fn open_index_writer(index: &Index) -> Result<IndexWriter, IndexError> {
    let index_writer = index.writer(50_000_000)
        .map_err(|e| {
            let reason = e.to_string();
            let error = IndexError::new(
                "Unable to create index writer",
                reason.as_str(),
            );
            error
        })?;
    Ok(index_writer)
}


/// Convenience method to open reader
pub(crate) fn open_index_reader(index: &Index) -> Result<IndexReader, IndexError> {
    let index_reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into().map_err(|e| {
        let reason = e.to_string();
        let error = IndexError::new(
            "Unable to create index reader",
            reason.as_str(),
        );
        error
    })?;
    Ok(index_reader)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::fs::remove_dir_all;
    use super::super::utils;
    use serde::Serialize;
    use crate::utils::as_value;

    #[derive(Serialize)]
    struct Dummy {
        x: String,
        y: String,
        z: u64,
    }

    impl Default for Dummy {
        fn default() -> Self {
            let x: String = "".to_string();
            let y: String = "".to_string();
            let z: u64 = 1u64;
            Self {
                x,
                y,
                z,
            }
        }
    }


    #[test]
    fn validate_open_mmap_on_missing_dir() {
        let path = random_string(Some(10));
        let p = Path::new(&path);
        assert!(!p.exists());
        let path = PathBuf::from_str(&path);
        assert!(path.is_ok());
        let path = path.ok().unwrap();
        let path = open_mmap_directory(path);
        assert!(path.is_ok());
        assert!(p.exists());
        let _ = remove_dir_all(&p);
    }

    #[test]
    fn validate_open_index_on_missing_dir() {
        let data = Dummy {
            x: "A".to_owned(),
            y: "B".to_owned(),
            z: 100,
        };
        let data = utils::as_value(&data).unwrap();
        let schema = utils::to_schema(&data, None).unwrap();
        let path = random_string(Some(10));
        let p = Path::new(&path);
        assert!(!p.exists());
        let path = PathBuf::from_str(&path).unwrap();
        let path = open_mmap_directory(path).unwrap();
        let result = open_index(path, Some(&schema));
        assert!(result.is_ok());
        assert!(p.exists());
        let _ = remove_dir_all(&p);
    }

    #[test]
    fn error_while_opening_new_index_without_schema() {
        let tmp_error_while_opening_new_index_without_schema = "error_while_opening_new_index_without_schema";

        let path = Path::new(tmp_error_while_opening_new_index_without_schema);
        assert!(!path.exists());
        let _ = std::fs::create_dir_all(path);
        assert!(path.exists());


        let dir = MmapDirectory::open(path).unwrap();
        let index = open_index(dir, None);
        assert!(index.is_err());
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn error_while_opening_open_index_writer() {
        let tmp_error_while_opening_new_index_without_schema = "error_while_opening_open_index_writer";

        let path = Path::new(tmp_error_while_opening_new_index_without_schema);
        assert!(!path.exists());
        let _ = std::fs::create_dir_all(path);
        assert!(path.exists());


        let dir = MmapDirectory::open(path).unwrap();


        let dummy = Dummy::default();
        let data = as_value(&dummy).unwrap();
        let schema = to_schema(&data, None).unwrap();
        let index = open_index(dir, Some(&schema)).unwrap();

        let _ = std::fs::remove_dir_all(path);

        let writer = open_index_writer(&index);
        assert!(writer.is_err());
    }

    #[test]
    fn error_while_opening_open_index_reader() {
        let tmp_error_while_opening_new_index_without_schema = "error_while_opening_open_index_reader";

        let path = Path::new(tmp_error_while_opening_new_index_without_schema);
        assert!(!path.exists());
        let _ = std::fs::create_dir_all(path);
        assert!(path.exists());


        let dir = MmapDirectory::open(path).unwrap();


        let dummy = Dummy::default();
        let data = as_value(&dummy).unwrap();
        let schema = to_schema(&data, None).unwrap();
        let index = open_index(dir, Some(&schema)).unwrap();

        let _ = std::fs::remove_dir_all(path);

        let reader = open_index_reader(&index);
        assert!(reader.is_err());
    }
}