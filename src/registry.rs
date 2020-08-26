use std::collections::{HashMap, BTreeMap};
use std::convert::TryFrom;
use std::ops::{Deref, DerefMut};
use std::fmt;

use tantivy::schema::{Schema, Field, TextOptions, IntOptions, IndexRecordOption};
use tantivy::{Index, IndexReader, IndexWriter, Document, Term};
use tantivy::query::{QueryParser, TermQuery};
use tantivy::collector::{TopDocs};
use tantivy::schema::Value as SchemaValue;


use crate::prelude::*;
use crate::prelude::join;

use serde::{Serialize};
use serde::de::DeserializeOwned;

#[derive(Clone, Eq, PartialEq)]
pub enum SurferFieldTypes {
    U64,
    I64,
    F64,
    String,
    Bytes,
}

// impl TryFrom<(Field, &str, SurferFieldTypes)> for Term {
//     type Error = IndexError;
//
//     fn try_from((field, field_value, field_type): (Field, &str, SurferFieldTypes)) -> Result<Self, Self::Error> {
//         let term = match field_type {
//             SurferFieldTypes::U64 => {
//                 let field_value = field_value.parse::<u64>().map_err(|e| {
//                     let message = format!("Invalid search: {}", query);
//                     let reason = e.to_string();
//                     IndexError::new(message, reason)
//                 })?;
//                 Term::from_field_u64(field, field_value)
//             }
//             SurferFieldTypes::I64 => {
//                 let field_value = field_value.parse::<i64>().map_err(|e| {
//                     let message = format!("Invalid search: {}", query);
//                     let reason = e.to_string();
//                     IndexError::new(message, reason)
//                 })?;
//                 Term::from_field_i64(field, field_value)
//             }
//             SurferFieldTypes::F64 => {
//                 let field_value = field_value.parse::<f64>().map_err(|e| {
//                     let message = format!("Invalid search: {}", query);
//                     let reason = e.to_string();
//                     IndexError::new(message, reason)
//                 })?;
//                 Term::from_field_f64(field, field_value)
//             }
//             SurferFieldTypes::String => {
//                 Term::from_field_text(field, field_value)
//             }
//             SurferFieldTypes::Bytes => {
//                 let message = format!("Invalid search: {}", query);
//                 let reason = "Cant search on bytes".to_string();
//                 IndexError::new(message, reason)
//             }
//         };
//     }
//     Ok(term)
// }


#[derive(Clone, Eq, PartialEq)]
pub struct SurferSchema {
    schema: Schema,
    mappings: HashMap<String, SurferFieldTypes>,
    track_tf: bool,
    track_tf_idf: bool,
}

impl SurferSchema {
    pub fn new(schema: Schema, mappings: HashMap<String, SurferFieldTypes>, track_tf: bool, track_tf_idf: bool) -> Self {
        Self {
            schema,
            mappings,
            track_tf,
            track_tf_idf,
        }
    }
    pub fn resolve_mapping(&self) -> &HashMap<String, SurferFieldTypes> {
        &self.mappings
    }
}

impl Deref for SurferSchema {
    type Target = Schema;
    fn deref(&self) -> &Self::Target {
        &self.schema
    }
}

impl DerefMut for SurferSchema {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.schema
    }
}

impl fmt::Debug for SurferSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let itr = self.schema.fields();
        let mut fields = Vec::new();
        for (field, entry) in itr {
            let debug = format!("Index: {} Name: {} Type: {:?}", field.field_id(), entry.name(), entry.field_type().value_type());
            fields.push(debug);
        };
        write!(f, "{:?}", fields)
    }
}

impl fmt::Display for SurferSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let itr = self.schema.fields();
        for (_, entry) in itr {
            let debug = format!("Name: {} Type: {:?}\n", entry.name(), entry.field_type().value_type());
            let _ = write!(f, "{}", debug);
        };
        write!(f, "\n")
    }
}


/// Builder struct for Surfer
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SurferBuilder {
    schemas: HashMap<String, SurferSchema>,
    home: Option<String>,
}

impl fmt::Display for SurferBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let home = self.home.as_ref();
        let indexes = &self.schemas;
        for (name, schema) in indexes {
            let home = resolve_index_directory_path(name, home);
            let home = match home {
                Ok(h) => h.to_string_lossy().to_string(),
                Err(e) => format!("<PathError {}>", e.to_string())
            };
            let _ = write!(f, "Index: {} Location: {}\n", name, home);
            let _ = write!(f, "{}", schema);
        }
        write!(f, "\n")
    }
}

#[derive(Serialize)]
struct SingleValuedNamedFieldDocument<'a>(BTreeMap<&'a str, &'a SchemaValue>);

/// Default impl to get things going
impl Default for SurferBuilder {
    fn default() -> Self {
        let schemas = HashMap::new();
        let home = None;
        Self {
            schemas,
            home,
        }
    }
}


/// Provides access to Surfer
impl SurferBuilder {
    /// Surfer Schema
    pub fn resolve_schemas(&self) -> &HashMap<String, SurferSchema> {
        &self.schemas
    }
    /// Set home location - default is indexes
    pub fn set_home(&mut self, home: &str) {
        self.home = Some(home.to_string());
    }
    /// Add a schema
    pub fn add_schema(&mut self, name: String, schema: SurferSchema) {
        self.schemas.insert(name, schema);
    }
    /// Add serde value panics otherwise
    fn add_serde<T: Serialize>(&mut self, name: String, data: &T) {
        let (schema, mappings) = to_schema(data, None).unwrap();
        let schema = SurferSchema::new(schema, mappings, false, false);
        self.schemas.insert(name, schema);
    }
    /// Add a serializable rust struct panics otherwise
    pub fn add_struct<T: Serialize>(&mut self, name: String, data: &T) {
        self.add_serde::<T>(name, data);
    }
}

/// Surfer: Client API
pub struct Surfer {
    home: String,
    indexes: HashMap<String, Index>,
    fields: HashMap<String, Vec<Field>>,
    readers: HashMap<String, Option<IndexReader>>,
    writers: HashMap<String, Option<IndexWriter>>,
    schemas: HashMap<String, SurferSchema>,
}

impl Surfer {
    /// Access to Surfer Schema
    pub fn resolve_schema(&self, name: &str) -> Option<&SurferSchema> {
        self.schemas.get(name)
    }
    /// Location of home
    pub fn home(&self) -> &String {
        &self.home
    }
    /// Location of Index
    pub fn which_index(&self, name: &str) -> Option<String> {
        if !self.indexes.contains_key(name) {
            return None;
        }
        if name.starts_with(&self.home) {
            Some(name.to_string())
        } else {
            join(&self.home, name)
        }
    }
    /// Access to Index
    pub fn resolve_index(&self, name: &str) -> Option<&Index> {
        if !self.indexes.contains_key(name) {
            return None;
        }
        self.indexes.get(name)
    }
    /// Inserts a struct
    pub fn insert_struct<T: Serialize>(&mut self, name: &str, data: &T) -> Result<(), IndexError> {
        let data = serde_json::to_string(data)?;
        let writer = self.writers.get(name);
        if writer.is_none() {
            return Ok(());
        };

        let index = self.indexes.get(name).unwrap();
        let schema = &index.schema();

        let writer = writer.unwrap();
        if writer.is_none() {
            let writer = open_index_writer(index)?;
            self.writers.insert(name.to_string(), Some(writer));
        };

        let writer = self.writers.get_mut(name).unwrap().as_mut().unwrap();
        let document = schema.parse_document(&data)?;
        writer.add_document(document);
        writer.commit()?;
        Ok(())
    }
    /// Inserts a structs
    pub fn insert_structs<T: Serialize>(&mut self, name: &str, payload: &Vec<T>) -> Result<(), IndexError> {
        let writer = self.writers.get(name);
        if writer.is_none() {
            return Ok(());
        };

        let index = self.indexes.get(name).unwrap();
        let schema = &index.schema();

        let writer = writer.unwrap();
        if writer.is_none() {
            let writer = open_index_writer(index)?;
            self.writers.insert(name.to_string(), Some(writer));
        };

        let writer = self.writers.get_mut(name).unwrap().as_mut().unwrap();
        for data in payload {
            let data = serde_json::to_string(data)?;
            let document = schema.parse_document(&data)?;
            writer.add_document(document);
        }

        writer.commit()?;
        Ok(())
    }
    /// Massive hack look away ;)
    fn jsonify(&self, name: &str, document: &Document) -> Result<String, IndexError> {
        let schema = self.indexes.get(name).unwrap().schema();

        let mut field_map = BTreeMap::new();
        for (field, field_values) in document.get_sorted_field_values() {
            let field_name = schema.get_field_name(field);
            let fv = field_values.get(0);
            if fv.is_none() {
                let message = format!("Unable to jsonify: {}", name);
                let reason = format!("Field: {} does not have any value", field_name);
                let error = IndexError::new(message, reason);
                return Err(error);
            };
            let fv = fv.unwrap().value();
            field_map.insert(field_name, fv);
        };
        let payload = SingleValuedNamedFieldDocument(field_map);
        let result = serde_json::to_string(&payload)
            .map_err(|e| {
                let message = "Unable to serialize struct".to_string();
                let reason = e.to_string();
                IndexError::new(
                    message,
                    reason,
                )
            });
        result
    }
    fn _is_index_valid(&self, name: &str) -> bool {
        let index = self.indexes.get(name);
        if index.is_some() {
            true
        } else {
            false
        }
    }

    fn _prepare_index_reader(&mut self, name: &str) -> Result<(), IndexError> {
        let valid = self._is_index_valid(name);
        if !valid {
            let message = format!("Unable to prepare the reader");
            let reason = format!("Index was missing: {} ", name);
            return Err(IndexError::new(message, reason));
        };
        let index = self.indexes.get(name).unwrap();
        let reader = open_index_reader(index)?;
        let _ = self.readers.insert(name.to_string(), Some(reader));
        Ok(())
    }

    pub fn read_stucts_by_field<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, field_name: &str, query: &str, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<T>>, IndexError> {
        {
            let result = self._prepare_index_reader(index_name);
            if result.is_err() {
                return Ok(None);
            };
        }
        let reader = self.readers.get(index_name).unwrap().as_ref().unwrap();
        let schema = self.schemas.get(index_name);
        if schema.is_none() {
            return Ok(None);
        }
        let schema = schema.unwrap();
        let mappings = schema.resolve_mapping();

        let field_type = mappings.get(field_name);
        if field_type.is_none() {
            return Ok(None);
        };
        let field_type = field_type.unwrap();

        let field = schema.get_field(field_name);
        if field.is_none() {
            return Ok(None);
        };
        let field = field.unwrap();

       let term = match field_type {
            SurferFieldTypes::U64 => {
                let field_value = query.parse::<u64>().map_err(|e| {
                    let message = format!("Invalid search: {}", query);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_u64(field, field_value)
            }
            SurferFieldTypes::I64 => {
                let field_value = query.parse::<i64>().map_err(|e| {
                    let message = format!("Invalid search: {}", query);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_i64(field, field_value)
            }
            SurferFieldTypes::F64 => {
                let field_value = query.parse::<f64>().map_err(|e| {
                    let message = format!("Invalid search: {}", query);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_f64(field, field_value)
            }
            SurferFieldTypes::String => {
                Term::from_field_text(field, query)
            }
            SurferFieldTypes::Bytes => {
                let message = format!("Invalid search: {}", query);
                let reason = "Cant search on bytes".to_string();
                return Err(IndexError::new(message, reason));
            }
        };


        let query = TermQuery::new(
            term,
            IndexRecordOption::Basic,
        );
        let limit = match limit {
            Some(limit) => limit,
            None => 10
        };
        let searcher = reader.searcher();
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| {
                let message = "Error while term query".to_string();
                let reason = e.to_string();
                IndexError::new(message, reason)
            })?;


        let mut docs = Vec::with_capacity(top_docs.len());
        for (doc_score, doc_address) in top_docs {
            if score.is_some() && doc_score < score.unwrap() {
                continue;
            }
            let doc = searcher.doc(doc_address)?;
            let doc = self.jsonify(index_name, &doc)?;
            let doc = serde_json::from_str::<T>(&doc).unwrap();
            docs.push(doc);
        };
        Ok(Some(docs))
    }

    /// Reads as string
    pub fn read_string(&mut self, name: &str, query: &str, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<String>>, IndexError> {
        {
            let result = self._prepare_index_reader(name);
            if result.is_err() {
                return Ok(None);
            };
        }
        let index = self.indexes.get(name).unwrap();
        let reader = self.readers.get(name).unwrap().as_ref().unwrap();

        let default_fields = self.fields.get(name).unwrap().clone();
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&index, default_fields);
        let query = query_parser.parse_query(query)?;
        let limit = if limit.is_some() {
            limit.unwrap()
        } else {
            10
        };
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut docs = Vec::with_capacity(top_docs.len());
        for (doc_score, doc_address) in top_docs {
            if score.is_some() && doc_score < score.unwrap() {
                continue;
            }
            let doc = searcher.doc(doc_address)?;
            let doc = self.jsonify(name, &doc)?;
            docs.push(doc);
        };
        Ok(Some(docs))
    }
    /// Reads as struct
    pub fn read_structs<T: Serialize + DeserializeOwned>(&mut self, name: &str, query: &str, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<T>>, IndexError> {
        {
            let result = self._prepare_index_reader(name);
            if result.is_err() {
                return Ok(None);
            };
        }
        let index = self.indexes.get(name).unwrap();
        let reader = self.readers.get(name).unwrap().as_ref().unwrap();

        let default_fields = self.fields.get(name).unwrap().clone();
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&index, default_fields);
        let query = query_parser.parse_query(query)?;
        let limit = if limit.is_some() {
            limit.unwrap()
        } else {
            10
        };
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut docs = Vec::with_capacity(top_docs.len());
        for (doc_score, doc_address) in top_docs {
            if score.is_some() && doc_score < score.unwrap() {
                continue;
            }
            let doc = searcher.doc(doc_address)?;
            let doc = self.jsonify(name, &doc)?;
            let doc = serde_json::from_str::<T>(&doc).unwrap();
            docs.push(doc);
        };
        Ok(Some(docs))
    }
}

/// Panics if somethings goes wrong
impl Surfer {
    pub fn new(builder: SurferBuilder) -> Self {
        Surfer::try_from(builder).unwrap()
    }
}

/// Opens mmap dir
fn initialize_mmap(name: &str, home: &str, schema: &Schema) -> Result<Index, IndexError> {
    let path = resolve_index_directory_path(name, Some(home))?;
    if path.exists() {
        let dir = open_mmap_directory(path)?;
        open_index(dir, None)
    } else {
        let dir = open_mmap_directory(path)?;
        open_index(dir, Some(&schema))
    }
}

/// Get home location
fn extract_home(builder: &SurferBuilder) -> Result<String, IndexError> {
    let home = builder.home.as_ref();
    let home = resolve_home(home)?;
    Ok(home.to_str().unwrap().to_string())
}

/// Setup indexes
fn initialized_index(home: &str, builder: &SurferBuilder) -> Result<HashMap<String, Index>, IndexError> {
    let schemas = &builder.schemas;
    let mut indexes = HashMap::<String, Index>::with_capacity(schemas.len());
    for (name, schema) in schemas {
        let index = initialize_mmap(name, &home, &schema)?;
        indexes.insert(name.to_string(), index);
    };
    Ok(indexes)
}

/// Extract field information
fn extract_fields(builder: &SurferBuilder) -> HashMap<String, Vec<Field>> {
    let data = &builder.schemas;
    let mut fields = HashMap::<String, Vec<Field>>::with_capacity(data.len());
    for (data, schema) in data {
        let key = data.clone();
        let value: Vec<Field> = schema.fields().map(|(f, _)| f).collect();
        fields.insert(key, value);
    };
    fields
}


impl TryFrom<SurferBuilder> for Surfer {
    type Error = IndexError;
    fn try_from(builder: SurferBuilder) -> Result<Self, Self::Error> {
        let home = extract_home(&builder)?;
        let indexes = initialized_index(&home, &builder)?;
        let fields = extract_fields(&builder);

        let mut readers = HashMap::new();
        let mut writers = HashMap::new();
        for (name, _) in &builder.schemas {
            let reader: Option<IndexReader> = None;
            let writer: Option<IndexWriter> = None;
            writers.insert(name.to_string(), writer);
            readers.insert(name.to_string(), reader);
        };
        let schemas = builder.resolve_schemas().clone();
        Ok(Surfer {
            home,
            indexes,
            fields,
            readers,
            writers,
            schemas,
        })
    }
}

/// Container to pass through config to tantivy
pub enum Control {
    ControlTextOptions(TextOptions),
    ControlIntOptions(IntOptions),
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::utils;
    use serde::{Serialize, Deserialize};
    use std::fmt::Debug;
    use std::path::Path;
    use std::fs::remove_dir_all;


    #[derive(Clone, Serialize, Debug, Deserialize, PartialEq)]
    struct OldMan {
        title: String,
        body: String,
    }

    impl Default for OldMan {
        fn default() -> Self {
            let title = "".to_string();
            let body = "".to_string();
            Self {
                title,
                body,
            }
        }
    }

    #[test]
    fn validate_read_existing_documents_as_structs() {
        let name = random_string(None);
        let home = "tmp";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());

        let data = OldMan::default();

        let mut builder = SurferBuilder::default();
        builder.set_home(home);
        builder.add_struct(name.clone(), &data);

        {
            let title = "The Old Man and the Sea".to_string();
            let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
            let old_man_doc = OldMan {
                title,
                body,
            };

            let mut surfer = Surfer::new(builder.clone());
            let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        }

        let mut surfer = Surfer::new(builder.clone());
        let query = "sea whale";
        let result = surfer.read_structs::<OldMan>(&name, query, None, None);
        assert!(result.is_ok());
        assert!(path.exists());
        let _ = remove_dir_all(index_path);
    }

    #[test]
    fn validate_read_existing_documents_as_strings() {
        let title = "The Old Man and the Sea".to_string();
        let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
        let expected = OldMan {
            title,
            body,
        };


        let name = random_string(None);
        let mut builder = SurferBuilder::default();
        let data = OldMan::default();
        let home = "tmp";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());
        builder.set_home(home);
        builder.add_struct(name.to_string(), &data);

        {
            let title = "The Old Man and the Sea".to_string();
            let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
            let old_man_doc = OldMan {
                title,
                body,
            };

            let mut surfer = Surfer::new(builder.clone());
            let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        }

        let mut surfer = Surfer::new(builder.clone());
        let query = "sea whale";
        let result = surfer.read_string("Non-existent", query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_none());
        let result = surfer.read_string(&name, query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        let mut computed = Vec::new();
        for entry in result {
            let data: serde_json::Result<OldMan> = serde_json::from_str(&entry);
            let data = data.unwrap();
            computed.push(data);
        };
        assert_eq!(computed, vec![expected.clone()]);

        // Reading documents again
        let result = surfer.read_string(&name, query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        let mut computed = Vec::new();
        for entry in result {
            let data: serde_json::Result<OldMan> = serde_json::from_str(&entry);
            let data = data.unwrap();
            computed.push(data);
        };
        assert_eq!(computed, vec![expected.clone()]);

        let _ = remove_dir_all(&index_path);
    }

    #[test]
    fn validate_as_rust_structs() {
        let name = random_string(None);
        let home = "tmp".to_string();
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());

        let title = "The Old Man and the Sea".to_string();
        let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
        let old_man_doc = OldMan {
            title,
            body,
        };


        let mut builder = SurferBuilder::default();
        builder.set_home(home.as_str());
        builder.add_struct(name.to_string(), &old_man_doc);
        let mut surfer = Surfer::new(builder);

        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        let query = "sea whale";

        let result = surfer.read_structs::<OldMan>("non-existent", query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_none());

        let result = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
        for computed in result {
            assert_eq!(computed, old_man_doc);
        };
        assert!(path.exists());

        // Reading documents again

        let result = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
        for computed in result {
            assert_eq!(computed, old_man_doc);
        };


        let _ = remove_dir_all(index_path);
    }

    #[test]
    fn validate_initialize_mmap() {
        let home = "tmp/indexes";
        let index_name = "someindex";
        let path_to_index = "tmp/indexes/someindex";
        let path = Path::new(path_to_index);
        assert!(!path.exists());
        let oldman = OldMan::default();
        let (schema, mappings) = to_schema(&oldman, None).unwrap();
        let schema = SurferSchema::new(schema, mappings, false, false);
        let _ = initialize_mmap(index_name, home, &schema);
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path_to_index);
    }

    #[test]
    fn validate_read_existing_documents_as_structs_limit_one() {
        let name = random_string(None);
        let home = "tmp";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());

        let data = OldMan::default();

        let mut builder = SurferBuilder::default();
        builder.set_home(home);
        builder.add_struct(name.clone(), &data);

        let title = "The Old Man and the Sea".to_string();
        let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
        let old_man_doc = OldMan {
            title,
            body,
        };

        let mut surfer = Surfer::new(builder.clone());
        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();

        let query = "sea whale";
        let result = surfer.read_structs::<OldMan>(&name, query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap().unwrap();
        assert_eq!(result.len(), 5);

        let result = surfer.read_structs::<OldMan>(&name, query, Some(1), None);
        assert!(result.is_ok());
        let result = result.unwrap().unwrap();
        assert_eq!(result.len(), 1);

        assert!(path.exists());
        let _ = remove_dir_all(index_path);
    }

    #[test]
    fn validate_read_existing_documents_as_structs_default_ten() {
        let name = random_string(None);
        let home = "tmp";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());

        let data = OldMan::default();

        let mut builder = SurferBuilder::default();
        builder.set_home(home);
        builder.add_struct(name.clone(), &data);

        let title = "The Old Man and the Sea".to_string();
        let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
        let old_man_doc = OldMan {
            title,
            body,
        };

        let mut surfer = Surfer::new(builder.clone());
        for _ in 0..20 {
            let _ = surfer.insert_struct(&name, &old_man_doc).unwrap();
        }


        let query = "sea whale";
        let result = surfer.read_structs::<OldMan>(&name, query, None, None);
        assert!(result.is_ok());
        let result = result.unwrap().unwrap();
        assert_eq!(result.len(), 10);

        let result = surfer.read_structs::<OldMan>(&name, query, Some(20), None);
        assert!(result.is_ok());
        let result = result.unwrap().unwrap();
        assert_eq!(result.len(), 20);

        assert!(path.exists());
        let _ = remove_dir_all(index_path);
    }

    #[derive(Serialize)]
    struct Dummy {
        x: String,
        y: String,
        z: u64,
    }

    #[test]
    fn validate_surfer_schema() {
        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100u64,
        };

        let data = utils::as_value(&data).unwrap();
        let (schema, mappings) = utils::to_schema(&data, None).unwrap();
        let surf_schema = SurferSchema::new(schema, mappings, false, false);

        let mut computed1 = SurferBuilder::default();
        computed1.add_schema("dummy".to_string(), surf_schema.clone());

        let mut computed2 = SurferBuilder::default();
        computed2.add_struct("dummy".to_string(), &data);

        assert_eq!(computed1, computed2);

        assert_eq!(format!("{:?}", computed1.schemas), format!("{:?}", computed2.schemas))
    }
}