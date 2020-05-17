use std::fmt::Debug;
use std::io;
use std::convert::From;

use serde::{Serialize};
use serde_json::error::Error as JsonError;

use failure::Fail;

use tantivy::directory::error::OpenDirectoryError;
use tantivy::TantivyError;
use tantivy::schema::DocParsingError;
use tantivy::query::QueryParserError;


#[derive(Debug, Fail, Clone, Serialize)]
#[fail(display = "Message: {}", message)]
pub struct IndexError {
    message: String,
    reason: String,

}

impl From<TantivyError> for IndexError {
    fn from(error: TantivyError) -> Self {
        Self::new("Unable to open Index", &error.to_string())
    }
}

impl IndexError {
    pub fn new<T: ToString>(message: T, reason: T) -> Self {
        let message = message.to_string();
        let reason = reason.to_string();
        Self {
            message,
            reason,
        }
    }
}

impl From<OpenDirectoryError> for IndexError {
    fn from(error: OpenDirectoryError) -> Self {
        let message = "Unable to MMap directory for indexing".to_string();
        let reason = error.to_string();
        Self {
            message,
            reason,
        }
    }
}


impl From<io::Error> for IndexError {
    fn from(error: io::Error) -> Self {
        let message = "Unable to create index dir".to_string();
        let reason = error.to_string();
        Self {
            message,
            reason,
        }
    }
}


impl From<DocParsingError> for IndexError {
    fn from(error: DocParsingError) -> Self {
        let message = "Unable to parse document".to_string();
        let reason = error.to_string();
        Self {
            message,
            reason,
        }
    }
}

impl From<QueryParserError> for IndexError {
    fn from(error: QueryParserError) -> Self {
        let message = "Unable to parse query".to_string();
        let reason = error.to_string();
        Self {
            message,
            reason,
        }
    }
}

impl From<JsonError> for IndexError {
    fn from(error: JsonError) -> Self {
        let message = "Unable to covert to json".to_string();
        let reason = error.to_string();
        Self {
            message,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::str::FromStr;
    use std::io::ErrorKind;
    use serde::{Deserialize};

    #[derive(Serialize, Deserialize)]
    struct Dummy {
        x: String,
        y: String,
        z: u64,
    }


    #[test]
    fn validate_index_error() {
        let message = "message".to_string();
        let reason = "reason".to_string();
        let error = IndexError {
            message,
            reason,
        };
        assert_eq!(format!("{}", error), error.to_string());
        assert_eq!(&error.message, "message");
        assert_eq!(&error.reason, "reason");
    }

    #[test]
    fn validate_index_error_from_tantivy_error() {
        let error = TantivyError::IndexAlreadyExists;
        let error: IndexError = error.into();
        assert_eq!(format!("{}", error), error.to_string());
    }

    #[test]
    fn validate_index_error_from_open_directory_error() {
        let path = PathBuf::from_str("doesnotexist").unwrap();
        assert_eq!(path.exists(), false);
        let error = OpenDirectoryError::DoesNotExist(path);
        let error: IndexError = error.into();
        assert_eq!(format!("{}", error), error.to_string());
    }

    #[test]
    fn validate_index_error_from_io_error() {
        let error = io::Error::from(ErrorKind::PermissionDenied);
        let error: IndexError = error.into();
        assert_eq!(format!("{}", error), error.to_string());
    }

    #[test]
    fn validate_index_error_from_doc_parsing_error() {
        let error = DocParsingError::NoSuchFieldInSchema("bs_field is not present".to_string());
        let error: IndexError = error.into();
        assert_eq!(format!("{}", error), error.to_string());
    }

    #[test]
    fn validate_index_error_from_query_parser_error() {
        let error = QueryParserError::FieldDoesNotExist("bs_field is not yet present".to_string());
        let error: IndexError = error.into();
        assert_eq!(format!("{}", error), error.to_string());
    }

    #[test]
    fn validate_index_error_from_json_error() {
        let json: serde_json::Result<Dummy> = serde_json::from_str("{\"ss\": \"ss\"}");
        assert!(json.is_err());
        let json = json.err().unwrap();

        let error: IndexError = json.into();
        assert_eq!(format!("{}", error), error.to_string());
    }
}
