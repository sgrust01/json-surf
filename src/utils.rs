use std::{thread::sleep, time::Duration, time::Instant};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rand::{Rng};
use rand::distributions::Alphanumeric;

use serde::{Serialize};
use serde;
use serde_value;
use serde_value::Value;

use tantivy::schema::{Schema, TextOptions, TEXT, IntOptions, STORED, SchemaBuilder};

use crate::prelude::*;
use crate::registry::SurferFieldTypes;

/// Convert a JSON serializable struct as JSON
pub(crate) fn as_value<T>(data: &T) -> Result<Value, IndexError>
    where
        T: Serialize,

{
    let result = serde_value::to_value(data).map_err(|e| {
        IndexError::new(
            "Unable to serialize data",
            &e.to_string(),
        )
    })?;
    Ok(result)
}

/// Get all field names
pub fn field_names(data: &Value) -> Option<Vec<String>> {
    if let Value::Map(kv) = data {
        let keys = kv.keys();
        let mut fields = Vec::with_capacity(keys.len());
        for key in keys {
            if let Value::String(name) = key {
                fields.push(name.to_owned());
            }
        }
        return Some(fields);
    };
    None
}


/// Store and index text by default
fn resolve_text_option(key: &str, control: Option<&HashMap<String, Control>>) -> TextOptions {
    let default = TEXT | STORED;

    match control {
        Some(c) => {
            let x = c.get(key);
            if x.is_none() {
                return default;
            }
            let x = x.unwrap();
            match x {
                Control::ControlTextOptions(opt) => {
                    let option = opt.clone();
                    option
                }
                _ => default
            }
        }
        None => default
    }
}

/// Store and index numbers by default
fn resolve_number_option(key: &str, control: Option<&HashMap<String, Control>>) -> IntOptions {
    let default = IntOptions::default();
    let default = default.set_indexed();
    let default = default.set_stored();
    match control {
        Some(c) => {
            let x = c.get(key);
            if x.is_none() {
                return default;
            }
            let x = x.unwrap();
            match x {
                Control::ControlIntOptions(opt) => {
                    let option = opt.clone();
                    option
                }
                _ => default
            }
        }
        None => default
    }
}

/// Join to path
pub fn join(head: &str, tail: &str) -> Option<String> {
    let head = Path::new(head);
    let tail = Path::new(tail);
    let path = head.join(tail);
    let path = path.to_str();
    match path {
        Some(p) => Some(p.to_string()),
        None => None
    }
}

/// Maps flat JSON structures
pub(crate) fn as_schema_builder<T: Serialize>(payload: &T, control: Option<&HashMap<String, Control>>) -> Result<(SchemaBuilder, HashMap<String, SurferFieldTypes>), IndexError> {
    let value = as_value(payload)?;
    let data = serde_json::to_string(payload).unwrap();
    let kv = match &value {
        Value::Map(kv) => kv,
        _ => {
            return Err(IndexError::new(
                "Unable to create schema",
                "Expected BTree Map", )
            );
        }
    };
    let mut names = Vec::new();
    let keys = kv.keys();
    for key in keys {
        match key {
            Value::String(k) => names.push(k.clone()),
            _ => {
                return Err(IndexError::new(
                    "Unable to create schema",
                    "keys were not string", )
                );
            }
        }
    };

    let mut field_names = HashMap::new();
    let mut indexes = Vec::new();
    let mut field_type_mappings = HashMap::<String, SurferFieldTypes>::new();

    for name in names {
        let lookup = format!("\"{}\":", name);
        let index = data.find(lookup.as_str()).unwrap();
        field_names.insert(index, name);
        indexes.push(index);
    };
    indexes.sort();
    let mut adjusted_field_names = Vec::new();

    for index in indexes {
        let field = field_names.get(&index).unwrap();
        adjusted_field_names.push(field.clone())
    }


    if let Value::Map(kv) = &value {
        let mut builder = Schema::builder();
        for key in adjusted_field_names {
            let sss = Value::String(key);
            let value = kv.get(&sss);
            if value.is_none() {
                continue;
            };
            let value = value.unwrap();
            if let Value::String(k) = &sss {
                match value {
                    Value::String(_) => {
                        let options = resolve_text_option(k, control);
                        builder.add_text_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::String);
                    }
                    Value::Bool(_) => {
                        let options = resolve_text_option(k, control);
                        builder.add_text_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::String);
                    }
                    Value::U64(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_u64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::U64);
                    }
                    Value::U32(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_u64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::U64);
                    }
                    Value::U16(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_u64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::U64);
                    }
                    Value::U8(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_u64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::U64);
                    }
                    Value::I64(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_i64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::I64);
                    }
                    Value::I32(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_i64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::I64);
                    }
                    Value::I16(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_i64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::I64);
                    }
                    Value::I8(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_i64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::I64);
                    }
                    Value::F64(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_f64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::F64);
                    }
                    Value::F32(_) => {
                        let options = resolve_number_option(k, control);
                        builder.add_f64_field(k, options);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::F64);
                    }
                    Value::Seq(_) => {
                        builder.add_bytes_field(k);
                        field_type_mappings.insert(k.to_string(), SurferFieldTypes::Bytes);
                    }
                    _ => {
                        return Err(IndexError::new(
                            "Unable to create schema",
                            "Unhandled value types", )
                        );
                    }
                }
            } else {
                return Err(IndexError::new(
                    "Unable to create schema",
                    "keys were not string", )
                );
            }
        }
        // TODO: Throw up for empty json
        // return Err(IndexError::new(
        //     "Unable to create schema",
        //     "Empty json", )
        // );

        return Ok((builder, field_type_mappings));
    };
    let error = IndexError::new(
        "Unable to create schema",
        "Invalid JSON",
    );
    Err(error)
}

/// List files within a dir
pub fn ls<T: AsRef<str>>(home: T) -> Result<Vec<PathBuf>, IndexError> {
    let paths = std::fs::read_dir(home.as_ref())?;
    let mut entries = Vec::<PathBuf>::new();
    for path in paths {
        let entry = path?;
        let value = entry.path();
        entries.push(value);
    };
    Ok(entries)
}


/// Convenience method to get schema
pub(crate) fn to_schema<T: Serialize>(payload: &T, control: Option<&HashMap<String, Control>>) -> Result<(Schema, HashMap<String, SurferFieldTypes>), IndexError> {
    let (builder, mappings) = as_schema_builder(payload, control)?;
    Ok((builder.build(), mappings))
}

/// block thread
pub fn block_thread(sleep_in_seconds: u64) -> u64 {
    let duration = Duration::from_secs(sleep_in_seconds);
    let now = Instant::now();
    sleep(duration);
    let result = Instant::now() - now;
    result.as_secs()
}

pub fn random_string(size: Option<usize>) -> String {
    let size = if size.is_none() {
        10
    } else {
        size.unwrap()
    };
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .collect::<String>()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Empty;

    #[derive(Serialize)]
    struct Emptish {
        value: Option<String>
    }

    #[derive(Serialize)]
    struct DataVec {
        identity: String,
        buffer: Vec<u8>,
    }


    #[test]
    fn validate_resolve_text_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        let text_options = TEXT;
        control.insert(key.to_string(), Control::ControlTextOptions(text_options));
        let options = resolve_text_option(key, Some(&control));
        assert_eq!(options.is_stored(), false);
    }

    #[test]
    fn validate_resolve_default_text_option() {
        let key = "dummy";
        let options = resolve_text_option(key, None);
        assert_eq!(options.is_stored(), true);
    }

    #[test]
    fn validate_resolve_number_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        let int_options = IntOptions::default();
        control.insert(key.to_string(), Control::ControlIntOptions(int_options));
        let options = resolve_number_option(key, Some(&control));
        assert_eq!(options.is_stored(), false);
    }

    #[test]
    fn validate_resolve_default_number_option() {
        let key = "dummy";
        let options = resolve_number_option(key, None);
        assert_eq!(options.is_stored(), true);
    }

    #[test]
    fn invalid_resolve_text_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        let int_options = IntOptions::default();
        control.insert(key.to_string(), Control::ControlIntOptions(int_options));
        let options = resolve_text_option(key, Some(&control));
        assert_eq!(options.is_stored(), true);
    }

    #[test]
    fn default_resolve_text_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        control.insert(key.to_string(), Control::ControlTextOptions(TextOptions::default()));
        let options = resolve_text_option(key, Some(&control));
        assert_eq!(options.is_stored(), false);
    }

    #[test]
    fn default_resolve_number_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        control.insert(key.to_string(), Control::ControlIntOptions(IntOptions::default()));
        let options = resolve_number_option(key, Some(&control));
        assert_eq!(options.is_stored(), false);
    }


    #[test]
    fn invalid_resolve_number_option() {
        let key = "dummy";
        let mut control = HashMap::new();
        let text_options = TEXT;
        control.insert(key.to_string(), Control::ControlTextOptions(text_options));
        let options = resolve_number_option(key, Some(&control));
        assert_eq!(options.is_stored(), true);
    }

    #[test]
    fn invalid_field_names() {
        let empty = Empty;
        let value = as_value(&empty).unwrap();
        let result = field_names(&value);
        assert!(result.is_none());
    }

    #[test]
    fn validate_error_on_empty_struct() {
        let data = Empty;
        let value = as_value(&data);
        assert!(value.is_ok());
        let value = value.unwrap();
        let result = as_schema_builder(&value, None);
        assert!(result.is_err());
    }

    #[test]
    fn validate_schema_builder_for_emptish() {
        let data = Emptish {
            value: None
        };
        let value = as_value(&data);
        assert!(value.is_ok());
        let value = value.unwrap();
        let result = as_schema_builder(&value, None);
        assert!(result.is_err());
    }

    #[test]
    fn validate_schema_builder_for_vec_does_not_work() {
        let identity = "Hello".to_string();
        let buffer = "World".as_bytes().to_vec();
        let data = DataVec {
            identity,
            buffer,
        };
        let result = as_value(&data);
        let data = result.unwrap();
        let (schema, _) = to_schema(&data, None).unwrap();
        let data = serde_json::to_string(&data).unwrap();
        let document = schema.parse_document(&data);
        assert!(document.is_err())
    }
}