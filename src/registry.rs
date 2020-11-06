use std::collections::{HashMap, BTreeMap, HashSet};
use std::convert::TryFrom;
use std::ops::{Deref, DerefMut};
use std::fmt;
use std::hash::{Hash, Hasher};

use tantivy::schema::{Schema, Field, TextOptions, IntOptions, IndexRecordOption};
use tantivy::{Index, IndexReader, IndexWriter, Document, Term, DocAddress};
use tantivy::query::{QueryParser, TermQuery};
use tantivy::collector::{TopDocs};
use tantivy::schema::Value as SchemaValue;


use crate::prelude::*;
use crate::prelude::join;

use serde::{Serialize};
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};


#[derive(Clone, Eq, PartialEq)]
pub enum SurferFieldTypes {
    U64,
    I64,
    F64,
    String,
    Bytes,
}

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AndCondition {
    field_name: String,
    field_value: String,
}

impl AndCondition {
    pub fn new(field_name: String, field_value: String) -> Self {
        Self {
            field_name,
            field_value,
        }
    }
    pub fn update_field_value(&mut self, field_value: String) {
        self.field_value = field_value;
    }
    pub fn resolve_field_name(&self) -> &String {
        &self.field_name
    }
    pub fn resolve_field_value(&self) -> &String {
        &self.field_value
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OrCondition {
    conditions: Vec<AndCondition>,
}

impl Display for OrCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let mut fragments = Vec::<String>::new();
        for (index, condition) in self.conditions.iter().enumerate() {
            if index == 0 {
                let fragment = format!("{} = {}", condition.resolve_field_name(), condition.resolve_field_value());
                fragments.push(fragment);
            }
        }
        let x = fragments.join::<&str>(" AND ");
        write!(f, "{}", x)
    }
}

impl OrCondition {
    pub fn new(conditions: Vec<AndCondition>) -> Self {
        Self {
            conditions
        }
    }
    pub fn resolve_conditions_for_edit(&mut self) -> &mut Vec<AndCondition> {
        &mut self.conditions
    }
    pub fn resolve_conditions(&self) -> &Vec<AndCondition> {
        &self.conditions
    }
}


impl From<(String, String)> for OrCondition {
    fn from((field_name, field_value): (String, String)) -> Self {
        let conditions = vec![AndCondition::new(field_name, field_value)];
        Self::new(conditions)
    }
}

/// Surfer: Client API
pub struct Surf {
    surfer: Surfer
}

impl Surf {
    pub fn new(surfer: Surfer) -> Self {
        Self {
            surfer,
        }
    }
    pub fn apply<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, conditions: &Vec<OrCondition>, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<T>>, IndexError> {
        self.surfer.multiple_structs_by_field(index_name, conditions, limit, score)
    }
    pub fn select<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, conditions: &Vec<OrCondition>) -> Result<Option<Vec<T>>, IndexError> {
        let limit = Some(100usize);
        let score = Some(0f32);
        self.apply(index_name, conditions, limit, score)
    }
}

impl Deref for Surf {
    type Target = Surfer;
    fn deref(&self) -> &Self::Target {
        &self.surfer
    }
}

impl DerefMut for Surf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.surfer
    }
}

impl From<Surfer> for Surf {
    fn from(surfer: Surfer) -> Self {
        Self::new(surfer)
    }
}

impl TryFrom<SurferBuilder> for Surf {
    type Error = IndexError;
    fn try_from(builder: SurferBuilder) -> Result<Self, Self::Error> {
        let surfer = Surfer::try_from(builder)?;
        Ok(Surf::from(surfer))
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
        {
            let result = self._prepare_index_writer(name);
            if result.is_err() {
                return Ok(());
            };
        }

        let writer = self.writers.get_mut(name).unwrap().as_mut().unwrap();

        let index = self.indexes.get(name).unwrap();
        let schema = &index.schema();
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
    fn _is_reader_valid(&self, name: &str) -> bool {
        if !self.readers.contains_key(name) {
            return false;
        }
        let reader = self.readers.get(name).unwrap();
        if reader.is_some() {
            true
        } else {
            false
        }
    }
    fn _is_writer_valid(&self, name: &str) -> bool {
        if !self.writers.contains_key(name) {
            return false;
        }
        let writer = self.writers.get(name).unwrap();
        if writer.is_some() {
            true
        } else {
            false
        }
    }
    fn _prepare_index_writer(&mut self, index_name: &str) -> Result<(), IndexError> {
        if !self._is_index_valid(index_name) {
            let message = format!("Unable to prepare the writer");
            let reason = format!("Index was missing: {} ", index_name);
            return Err(IndexError::new(message, reason));
        };
        if self._is_writer_valid(&index_name) {
            return Ok(());
        };
        let index = self.indexes.get(index_name).unwrap();
        let writer = open_index_writer(index)?;
        let _ = self.writers.insert(index_name.to_string(), Some(writer));
        Ok(())
    }

    fn _prepare_index_reader(&mut self, index_name: &str) -> Result<(), IndexError> {
        if !self._is_index_valid(index_name) {
            let message = format!("Unable to prepare the reader");
            let reason = format!("Index was missing: {} ", index_name);
            return Err(IndexError::new(message, reason));
        };
        if self._is_reader_valid(&index_name) {
            return Ok(());
        };
        let index = self.indexes.get(index_name).unwrap();
        let reader = open_index_reader(index)?;
        let _ = self.readers.insert(index_name.to_string(), Some(reader));
        Ok(())
    }
    fn _build_terms(&self, schema: &SurferSchema, field_value: &str) -> Result<Vec<Term>, IndexError> {
        let mut field_names = Vec::<&String>::with_capacity(schema.mappings.len());
        for (field_name, field_type) in schema.mappings.iter() {
            match field_type {
                SurferFieldTypes::String => field_names.push(field_name),
                _ => {}
            };
        }
        let mut terms = Vec::<Term>::with_capacity(schema.mappings.len());
        if field_names.is_empty() {
            return Ok(terms);
        };
        for field_name in field_names {
            let term = self._build_term(schema, field_name, field_value)?;
            terms.push(term);
        }
        Ok(terms)
    }
    fn _build_term(&self, schema: &SurferSchema, field_name: &str, field_value: &str) -> Result<Term, IndexError> {
        let mappings = schema.resolve_mapping();

        let field_type = mappings.get(field_name);
        if field_type.is_none() {
            let message = format!("Unable to perform search");
            let reason = format!("Missing field: {}", field_name);
            return Err(IndexError::new(message, reason));
        };
        let field_type = field_type.unwrap();

        let field = schema.get_field(field_name);
        if field.is_none() {
            let message = format!("Unable to perform search");
            let reason = format!("Missing field: {}", field_name);
            return Err(IndexError::new(message, reason));
        };
        let field = field.unwrap();

        let term = match field_type {
            SurferFieldTypes::U64 => {
                let field_value = field_value.parse::<u64>().map_err(|e| {
                    let message = format!("Invalid search: {}", field_value);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_u64(field, field_value)
            }
            SurferFieldTypes::I64 => {
                let field_value = field_value.parse::<i64>().map_err(|e| {
                    let message = format!("Invalid search: {}", field_value);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_i64(field, field_value)
            }
            SurferFieldTypes::F64 => {
                let field_value = field_value.parse::<f64>().map_err(|e| {
                    let message = format!("Invalid search: {}", field_value);
                    let reason = e.to_string();
                    IndexError::new(message, reason)
                })?;
                Term::from_field_f64(field, field_value)
            }
            SurferFieldTypes::String => {
                Term::from_field_text(field, field_value)
            }
            SurferFieldTypes::Bytes => {
                let message = format!("Invalid search: {}", field_value);
                let reason = "Cant search on bytes".to_string();
                return Err(IndexError::new(message, reason));
            }
        };

        Ok(term)
    }

    fn _build_term_query(&self, term: Term, segment_postings_options: Option<IndexRecordOption>) -> Result<TermQuery, IndexError> {
        let segment_postings_options = match segment_postings_options {
            Some(option) => option,
            None => IndexRecordOption::Basic,
        };
        Ok(TermQuery::new(term, segment_postings_options))
    }

    fn _resolve_surfer_schema(&self, index_name: &str) -> Result<&SurferSchema, IndexError> {
        let schema = self.schemas.get(index_name);
        if schema.is_none() {
            let message = format!("Invalid index operation for {}", index_name);
            let reason = format!("No schema found for index: {}", index_name);
            return Err(IndexError::new(message, reason));
        };
        let schema = schema.unwrap();
        Ok(schema)
    }

    fn _resolve_limit(&self, limit: Option<usize>) -> usize {
        match limit {
            Some(limit) => limit,
            None => 10
        }
    }

    fn _resolve_score(&self, score: Option<f32>) -> f32 {
        match score {
            Some(score) => score,
            None => 90f32
        }
    }

    /// Uses term search
    pub fn delete_structs_by_field(&mut self, index_name: &str, field_name: &str, field_value: &str) -> Result<(), IndexError> {
        let schema = self._resolve_surfer_schema(index_name)?;
        let term = self._build_term(&schema, field_name, field_value)?;
        let _ = self._prepare_index_writer(index_name)?;
        let writer = self.writers.get_mut(index_name).unwrap().as_mut().unwrap();
        let _ = writer.delete_term(term);
        let _ = writer.commit()?;
        Ok(())
    }

    /// Uses full text serach
    pub fn delete_structs(&mut self, index_name: &str, field_value: &str) -> Result<(), IndexError> {
        let schema = self._resolve_surfer_schema(index_name)?;
        let terms = self._build_terms(&schema, field_value)?;
        let _ = self._prepare_index_writer(index_name)?;
        let writer = self.writers.get_mut(index_name).unwrap().as_mut().unwrap();
        for i in 0..terms.len() {
            let term = terms.get(i).unwrap().to_owned();
            let _ = writer.delete_term(term);
        }
        let _ = writer.commit()?;
        Ok(())
    }

    /// Uses term search
    pub fn read_all_structs_by_field<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, field_name: &str, field_value: &str) -> Result<Option<Vec<T>>, IndexError> {
        self.read_structs_by_field::<T>(index_name, field_name, field_value, None, Some(0f32))
    }
    /// Uses term search
    pub fn read_structs_by_field<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, field_name: &str, field_value: &str, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<T>>, IndexError> {
        let conditions = vec![OrCondition::from((field_name.to_string(), field_value.to_string()))];
        self.multiple_structs_by_field::<T>(index_name, &conditions, limit, score)
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
    pub fn read_all_structs<T: Serialize + DeserializeOwned>(&mut self, name: &str, query: &str) -> Result<Option<Vec<T>>, IndexError> {
        self.read_structs(name, query, None, None)
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

        let surfer_schema = self.schemas.get(name).unwrap();
        let mappings = surfer_schema.resolve_mapping();

        let mut fields = Vec::<Field>::with_capacity(mappings.len());
        for (f, fe) in surfer_schema.schema.fields() {
            let name = fe.name();
            if !mappings.contains_key(name) {
                continue;
            };
            let ft = mappings.get(name).unwrap();
            match ft {
                SurferFieldTypes::String => fields.push(f),
                _ => {}
            }
        };

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&index, fields);
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
    /// Uses term search
    fn multiple_structs_by_field<T: Serialize + DeserializeOwned>(&mut self, index_name: &str, conditions: &Vec<OrCondition>, limit: Option<usize>, score: Option<f32>) -> Result<Option<Vec<T>>, IndexError> {
        let _ = self._prepare_index_reader(index_name)?;
        let reader = self.readers.get(index_name).unwrap().as_ref().unwrap();
        let searcher = reader.searcher();
        let limit = self._resolve_limit(limit);
        let cutoff = self._resolve_score(score);
        let schema = self._resolve_surfer_schema(index_name)?;
        let mut all_docs = HashSet::<SurferDocAddress>::new();

        for condition in conditions {
            let and = condition.resolve_conditions();
            let mut docs = HashSet::new();
            for (i, c) in and.iter().enumerate() {
                let field_name = c.resolve_field_name();
                let field_value = c.resolve_field_value();
                let term = self._build_term(schema, field_name, field_value)?;
                let query = self._build_term_query(term, None)?;
                let mut tmp = HashSet::new();
                let top_docs = searcher
                    .search(&query, &TopDocs::with_limit(limit))
                    .map_err(|e| {
                        let message = "Error while term query".to_string();
                        let reason = e.to_string();
                        IndexError::new(message, reason)
                    })?;


                for (score, address) in top_docs.to_owned() {
                    if score < cutoff {
                        continue;
                    };

                    let address = SurferDocAddress::from(address);
                    // println!("Found: {}", address);
                    if i == 0 {
                        tmp.insert(address);
                        continue;
                    }

                    if docs.contains(&address) {
                        tmp.insert(address);
                    }
                }

                if tmp.is_empty() {
                    docs.clear();
                    break;
                } else {
                    docs.extend(tmp);
                }
            }
            all_docs.extend(docs);
        };

        let mut docs = Vec::with_capacity(all_docs.len());
        for doc_address in all_docs {
            let doc = searcher.doc(doc_address.0)?;
            let doc = self.jsonify(index_name, &doc)?;
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

struct SurferDocAddress(DocAddress);

impl SurferDocAddress {
    fn new(address: DocAddress) -> Self {
        Self(address)
    }
}

impl From<DocAddress> for SurferDocAddress {
    fn from(address: DocAddress) -> Self {
        Self::new(address)
    }
}


impl Deref for SurferDocAddress {
    type Target = DocAddress;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SurferDocAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for SurferDocAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let address = &self.0;
        write!(f, "SegmentLocalId = {}, DocId {})", address.0, address.1)
    }
}

impl Debug for SurferDocAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let address = &self.0;
        write!(f, "SegmentLocalId = {}, DocId {})", address.0, address.1)
    }
}


impl PartialEq for SurferDocAddress {
    fn eq(&self, other: &SurferDocAddress) -> bool {
        let doc_address = &self.0;
        let other_doc_address = other.0;
        doc_address.0 == other_doc_address.0 && doc_address.1 == other_doc_address.1
    }
}

impl Hash for SurferDocAddress {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let address = self.0;
        state.write_u32(address.0);
        state.write_u32(address.1);
        state.finish();
    }
}

impl Eq for SurferDocAddress {}

/// Container to pass through config to tantivy
pub enum Control {
    ControlTextOptions(TextOptions),
    ControlIntOptions(IntOptions),
}


#[cfg(test)]
mod library_tests {
    use super::*;
    use super::super::utils;
    use serde::{Serialize, Deserialize};
    use std::fmt::Debug;
    use std::path::Path;
    use std::fs::remove_dir_all;
    use std::cmp::{Ord, Ordering, Eq};
    use std::collections::HashSet;
    use std::iter::FromIterator;
    use std::hash::{Hash, Hasher};


    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Giant {
        a: String,
        c: u64,
        d: u32,
        e: u16,
        f: u8,
        g: i64,
        h: i32,
        i: i16,
        j: i8,
        k: f64,
        l: f32,

    }

    impl Default for Giant {
        fn default() -> Self {
            let a: String = "tag1".to_string();
            let c: u64 = 10000;
            let d: u32 = 1000;
            let e: u16 = 100;
            let f: u8 = 10;
            let g: i64 = 20000;
            let h: i32 = 2000;
            let i: i16 = 200;
            let j: i8 = 20;
            let k: f64 = 10.0;
            let l: f32 = 1.0;
            Giant {
                a,
                c,
                d,
                e,
                f,
                g,
                h,
                i,
                j,
                k,
                l,

            }
        }
    }

    #[test]
    fn validate_giant() {
        let name = random_string(None);
        let home = ".validate_giant";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());

        let giant = Giant::default();

        let mut builder = SurferBuilder::default();
        builder.set_home(home);
        builder.add_struct("giant".to_string(), &giant);
        let mut surfer = Surfer::try_from(builder).unwrap();
        let data = vec![Giant::default()];
        let computed = surfer.insert_structs("giant", &data);
        assert!(computed.is_ok());
        let a: String = "tag1".to_string();
        let c: u64 = 10000;
        let d: u32 = 1000;
        let e: u16 = 100;
        let f: u8 = 10;
        let g: i64 = 20000;
        let h: i32 = 2000;
        let i: i16 = 200;
        let j: i8 = 20;
        let k: f64 = 10.0;
        let l: f32 = 1.0;

        let conditions = vec![
            AndCondition::new("a".to_string(), format!("{}", a)),
            AndCondition::new("c".to_string(), format!("{}", c)),
            AndCondition::new("d".to_string(), format!("{}", d)),
            AndCondition::new("e".to_string(), format!("{}", e)),
            AndCondition::new("g".to_string(), format!("{}", g)),
            AndCondition::new("f".to_string(), format!("{}", f)),
            AndCondition::new("h".to_string(), format!("{}", h)),
            AndCondition::new("i".to_string(), format!("{}", i)),
            AndCondition::new("j".to_string(), format!("{}", j)),
            AndCondition::new("k".to_string(), format!("{}", k)),
            AndCondition::new("l".to_string(), format!("{}", l)),
        ];
        let conditions = vec![OrCondition::new(conditions)];
        let computed = surfer.multiple_structs_by_field::<Giant>("giant", &conditions, None, Some(0.0));
        assert!(computed.is_ok());
        let computed = computed.unwrap();
        assert!(computed.is_some());
        let computed = computed.unwrap();
        assert_eq!(computed, vec![Giant::default()]);

        let computed = surfer.multiple_structs_by_field::<Giant>("giant", &conditions, None, None);
        assert!(computed.is_ok());


        let computed = surfer.delete_structs_by_field("giant", "c", "crap");
        assert!(computed.is_err());

        let computed = surfer.delete_structs_by_field("giant", "c", "crap");
        assert!(computed.is_err());

        let computed = surfer.delete_structs_by_field("giant", "g", "crap");
        assert!(computed.is_err());

        let computed = surfer.delete_structs_by_field("giant", "k", "crap");
        assert!(computed.is_err());

        let computed = surfer.delete_structs_by_field("giant", "crap", "crap");
        assert!(computed.is_err());

        let computed = surfer.delete_structs_by_field("crap", "crap", "crap");
        assert!(computed.is_err());

        let _ = remove_dir_all(index_path);
        let _ = remove_dir_all(home);
    }


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
        let home = ".validate_read_existing_documents_as_structs";
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
        let _ = remove_dir_all(home);
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
        let home = ".validate_read_existing_documents_as_strings";
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
        let _ = remove_dir_all(home);
    }

    #[test]
    fn validate_as_rust_structs() {
        let name = random_string(None);
        let home = ".validate_as_rust_structs".to_string();
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
        let _ = remove_dir_all(home);
    }

    #[test]
    fn validate_initialize_mmap() {
        let home = ".validate_initialize_mmap";
        let index_name = "someindex";
        let path_to_index = ".validate_initialize_mmap/someindex";
        let path = Path::new(path_to_index);
        assert!(!path.exists());
        let oldman = OldMan::default();
        let (schema, mappings) = to_schema(&oldman, None).unwrap();
        let schema = SurferSchema::new(schema, mappings, false, false);
        let _ = initialize_mmap(index_name, home, &schema);
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path_to_index);

        let _ = remove_dir_all(path_to_index);
        let _ = remove_dir_all(index_name);
        let _ = remove_dir_all(home);
    }

    #[test]
    fn validate_read_existing_documents_as_structs_limit_one() {
        let name = random_string(None);
        let home = ".validate_read_existing_documents_as_structs_limit_one";
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
        let _ = remove_dir_all(home);
    }

    #[test]
    fn validate_read_existing_documents_as_structs_default_ten() {
        let name = random_string(None);
        let home = ".validate_read_existing_documents_as_structs_default_ten";
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
        let _ = remove_dir_all(home);
    }

    #[derive(Serialize, Deserialize)]
    struct Dummy {
        x: String,
        y: String,
        z: u64,
    }

    #[test]
    fn check_invalid_index_name() {
        let home = random_string(None);
        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100u64,
        };
        let mut builder = SurferBuilder::default();
        builder.set_home(&home);
        builder.add_struct("dummy".to_string(), &data);
        let surfer = Surfer::try_from(builder).unwrap();
        let computed = surfer.which_index("crap");
        assert!(computed.is_none());
        let home = Path::new(&home);
        assert!(home.exists());
        let index_path = surfer.which_index("dummy").unwrap();
        let _ = remove_dir_all(&index_path);
        let _ = remove_dir_all(&home);
    }

    #[test]
    fn check_invalid_index_insert() {
        let home = random_string(None);
        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100u64,
        };
        let mut builder = SurferBuilder::default();
        builder.set_home(&home);
        builder.add_struct("dummy".to_string(), &data);
        let mut surfer = Surfer::try_from(builder).unwrap();
        let data = vec![data];
        let computed = surfer.insert_structs("crap", &data);
        assert!(computed.is_ok());
        let index_path = surfer.which_index("dummy").unwrap();
        let _ = remove_dir_all(&index_path);
        let _ = remove_dir_all(&home);
    }

    #[test]
    fn check_invalid_index_lookup() {
        let name = random_string(None);
        let home = ".check_invalid_index_lookup";
        let index_path = format!("{}/{}", home, name);
        let path = Path::new(&index_path);
        assert!(!path.exists());


        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100u64,
        };
        let mut builder = SurferBuilder::default();
        builder.set_home(&home);
        let mut surfer = Surfer::try_from(builder).unwrap();
        let _ = surfer.insert_struct("dummy", &data).unwrap();
        let computed = surfer.read_structs::<Dummy>("crap", "X", None, None);
        assert!(computed.is_ok());
        let computed = computed.unwrap();
        assert!(computed.is_none());
        let _ = remove_dir_all(&path);
        let _ = remove_dir_all(&home);
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

    // Main struct
    #[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
    struct UserInfo {
        first: String,
        last: String,
        age: u8,
    }

    impl UserInfo {
        pub fn new(first: String, last: String, age: u8) -> Self {
            Self {
                first,
                last,
                age,
            }
        }
    }

    impl Default for UserInfo {
        fn default() -> Self {
            let first = "".to_string();
            let last = "".to_string();
            let age = 0u8;
            UserInfo::new(first, last, age)
        }
    }

    #[test]
    fn test_user_info() {
        // Specify home location for indexes
        let home = ".test_user_info".to_string();
        // Specify index name
        let index_name = "store".to_string();

        // Prepare builder
        let mut builder = SurferBuilder::default();
        builder.set_home(&home);

        let data = UserInfo::default();
        builder.add_struct(index_name.clone(), &data);

        // Prepare Surfer
        let mut surfer = Surfer::try_from(builder).unwrap();

        // Prepare data to insert & search

        // User 1: John Doe
        let first = "John".to_string();
        let last = "Doe".to_string();
        let age = 20u8;
        let john_doe = UserInfo::new(first, last, age);

        // User 2: Jane Doe
        let first = "Jane".to_string();
        let last = "Doe".to_string();
        let age = 18u8;
        let jane_doe = UserInfo::new(first, last, age);

        // User 3: Jonny Doe
        let first = "Jonny".to_string();
        let last = "Doe".to_string();
        let age = 10u8;
        let jonny_doe = UserInfo::new(first, last, age);

        // User 4: Jinny Doe
        let first = "Jinny".to_string();
        let last = "Doe".to_string();
        let age = 10u8;
        let jinny_doe = UserInfo::new(first, last, age);

        // Writing structs

        // Option 1: One struct at a time
        let _ = surfer.insert_struct(&index_name, &john_doe).unwrap();
        let _ = surfer.insert_struct(&index_name, &jane_doe).unwrap();

        // Option 2: Write all structs together
        let users = vec![jonny_doe.clone(), jinny_doe.clone()];
        let _ = surfer.insert_structs(&index_name, &users).unwrap();

        block_thread(1);

        // Reading structs

        // Option 1: Full text search
        let expected = vec![john_doe.clone()];
        let computed = surfer.read_all_structs::<UserInfo>(&index_name, "John").unwrap().unwrap();
        assert_eq!(expected, computed);

        let mut expected = vec![john_doe.clone(), jane_doe.clone(), jonny_doe.clone(), jinny_doe.clone()];
        expected.sort();
        let mut computed = surfer.read_all_structs::<UserInfo>(&index_name, "doe").unwrap().unwrap();
        computed.sort();
        assert_eq!(expected, computed);

        // Option 2: Term search
        let mut expected = vec![jonny_doe.clone(), jinny_doe.clone()];
        expected.sort();
        let mut computed = surfer.read_all_structs_by_field::<UserInfo>(&index_name, "age", "10").unwrap().unwrap();
        computed.sort();
        assert_eq!(expected, computed);

        // Delete structs

        // Option 1: Delete based on all text fields
        // Before delete
        let before = surfer.read_all_structs::<UserInfo>(&index_name, "doe").unwrap().unwrap();
        let before: HashSet<UserInfo> = HashSet::from_iter(before.into_iter());

        // Delete any occurrence of John (Actual call to delete)
        surfer.delete_structs(&index_name, "john").unwrap();

        // After delete
        let after = surfer.read_all_structs::<UserInfo>(&index_name, "doe").unwrap().unwrap();
        let after: HashSet<UserInfo> = HashSet::from_iter(after.into_iter());
        // Check difference
        let computed: Vec<UserInfo> = before.difference(&after).map(|e| e.clone()).collect();
        // Only John should be deleted
        let expected = vec![john_doe];
        assert_eq!(expected, computed);

        // Option 2: Delete based on a specific field
        // Before delete
        let before = surfer.read_all_structs_by_field::<UserInfo>(&index_name, "age", "10").unwrap().unwrap();
        let before: HashSet<UserInfo> = HashSet::from_iter(before.into_iter());

        // Delete any occurrence where age = 10 (Actual call to delete)
        surfer.delete_structs_by_field(&index_name, "age", "10").unwrap();

        // After delete
        let after = surfer.read_all_structs_by_field::<UserInfo>(&index_name, "age", "10").unwrap().unwrap();
        let after: HashSet<UserInfo> = HashSet::from_iter(after.into_iter());
        // Check difference
        let mut computed: Vec<UserInfo> = before.difference(&after).map(|e| e.clone()).collect();
        computed.sort();
        // Both Jonny & Jinny should be deleted
        let mut expected = vec![jonny_doe, jinny_doe];
        expected.sort();
        assert_eq!(expected, computed);


        // Clean-up
        let path = surfer.which_index(&index_name).unwrap();
        let _ = remove_dir_all(&path);
        let _ = remove_dir_all(&home);
    }

    #[test]
    fn test_where_clause() {
        // Specify home location for indexes
        let home = ".test_where_clause".to_string();
        // Specify index name
        let index_name = "store".to_string();

        // Prepare builder
        let mut builder = SurferBuilder::default();
        builder.set_home(&home);

        let data = UserInfo::default();
        builder.add_struct(index_name.clone(), &data);

        // Prepare Surfer
        let surfer = Surfer::try_from(builder).unwrap();
        let mut surf = Surf::from(surfer);
        let surfer = &mut surf;

        // Prepare data to insert & search

        // User 1: John Doe
        let first = "John".to_string();
        let last = "Doe".to_string();
        let age = 20u8;
        let john_doe = UserInfo::new(first, last, age);

        // User 2: Jane Doe
        let first = "Jane".to_string();
        let last = "Doe".to_string();
        let age = 18u8;
        let jane_doe = UserInfo::new(first, last, age);

        // User 3: Jonny Doe
        let first = "Jonny".to_string();
        let last = "Doe".to_string();
        let age = 10u8;
        let jonny_doe = UserInfo::new(first, last, age);

        // User 4: Jinny Doe
        let first = "Jinny".to_string();
        let last = "Doe".to_string();
        let age = 10u8;
        let jinny_doe = UserInfo::new(first, last, age);

        // Writing structs

        let users = vec![john_doe.clone(), jane_doe.clone(), jonny_doe.clone(), jinny_doe.clone()];
        let _ = surfer.insert_structs(&index_name, &users).unwrap();
        block_thread(1);

        let conditions = vec![OrCondition::from(("age".to_string(), "10".to_string()))];
        let mut expected = vec![jonny_doe.clone(), jinny_doe.clone()];
        let mut computed = surfer.select::<UserInfo>(&index_name, &conditions).unwrap().unwrap();
        expected.sort();
        computed.sort();
        assert_eq!(expected, computed);

        let conditions = vec![OrCondition::from(("age".to_string(), "10".to_string())),
                              OrCondition::from(("first".to_string(), "john".to_string()))];
        let mut expected = vec![john_doe.clone(), jonny_doe.clone(), jinny_doe.clone()];
        let mut computed = surfer.select::<UserInfo>(&index_name, &conditions).unwrap().unwrap();
        expected.sort();
        computed.sort();
        assert_eq!(expected, computed);


        let name_condition = AndCondition::new("first".to_string(), "jinny".to_string());
        let age_condition = AndCondition::new("age".to_string(), "10".to_string());
        let and_conditions = vec![name_condition, age_condition];
        let child_condition = OrCondition::new(and_conditions);
        let parent_condition = OrCondition::from(("first".to_string(), "john".to_string()));
        let conditions = vec![child_condition, parent_condition];

        let mut expected = vec![john_doe.clone(), jinny_doe.clone()];
        let mut computed = surfer.select::<UserInfo>(&index_name, &conditions).unwrap().unwrap();
        expected.sort();
        computed.sort();
        assert_eq!(expected, computed);

        // Clean-up
        let path = surfer.which_index(&index_name).unwrap();
        let _ = remove_dir_all(&path);
        let _ = remove_dir_all(&home);
    }

    /// Convenience method for sorting & likely not required in user code
    impl Ord for UserInfo {
        fn cmp(&self, other: &Self) -> Ordering {
            if self.first == other.first && self.last == other.last {
                return Ordering::Equal;
            };
            if self.first == other.first {
                if self.last > other.last {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            } else {
                if self.first > other.first {
                    Ordering::Greater
                } else {
                    Ordering::Less
                }
            }
        }
    }

    /// Convenience method for sorting & likely not required in user code
    impl Eq for UserInfo {}

    /// Convenience method for sorting & likely not required in user code
    impl Hash for UserInfo {
        fn hash<H: Hasher>(&self, state: &mut H) {
            for i in self.first.as_bytes() {
                state.write_u8(*i);
            }
            for i in self.last.as_bytes() {
                state.write_u8(*i);
            }
            state.write_u8(self.age);
            state.finish();
        }
    }
}