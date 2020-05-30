//! JSON-Surf
//! ## Features
//! * Full text search
//! * Serialize __**flat**__ JSON/Struct
//! * Easy write and read API
//! * Write multiple documents together
//! * Requires no runtime
//! * No unsafe block
//! * Run on rust stable (Please check the Rust version, 1.39 does not work)
//! * Coming Soon: Bigram suggestion & TF-IDF support
//!
//! ## Motivation
//! * Allow your existing simples flat rust structs to be searched
//! * Encoded/Decoded byte streams can be stored along side too as base64 encoded string
//! * The crate will support arbitary byte stream once it is supported by tantivy (see [here](https://github.com/tantivy-search/tantivy/issues/832))
//! * This can just act as a container to actual data in databases, keeping indexes light
//! * Create time-aware containers which could possibly updated/deleted
//! * Create ephemeral storage for request/response
//! * This can integrate with any web-server to index and search near real-time
//! * This crate is just a convenience crate over [tantivy](https://github.com/tantivy-search/tantivy).
//! * This crate will focus mostly on user-workflow(s) related problem(s)
//!
//! ## Quickstart
//!
//! ### Prerequisite:
//!
//!  ```toml
//!   [dependencies]
//!   json-surf = "*"
//! ```
//!
//! ### Example
//! ```rust
//! use std::convert::TryFrom;
//! use std::fs::remove_dir_all;
//!
//! use serde::{Serialize, Deserialize};
//!
//! use json_surf::prelude::*;
//!
//! /// Document to be indexed and searched
//! #[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
//! struct OldMan {
//!     title: String,
//!     body: String,
//! }
//!
//! impl OldMan {
//!     pub fn new(title: String, body: String) -> Self {
//!         Self {
//!             title,
//!             body,
//!         }
//!     }
//! }
//!
//! /// Convenience implementation for bootstraping the builder
//! impl Default for OldMan {
//!     fn default() -> Self {
//!         let title = "".to_string();
//!         let body = "".to_string();
//!         OldMan::new(title, body)
//!     }
//! }
//!
//! /// Convenience method to keep indexes tucked under a directory
//! fn home_and_random_index_name() -> (String, String) {
//!     let home = ".store/examples".to_string();
//!     let name = random_string(None);
//!     (home, name)
//! }
//!
//! fn main() {
//!     let (home, name) = home_and_random_index_name();
//!
//!     // Mostly empty but can be a real flat struct
//!     let data = OldMan::default();
//!
//!     // Prepare the builder instance
//!     let mut builder = SurferBuilder::default();
//!     // By default everything goes to directory indexes
//!     builder.set_home(&home);
//!     builder.add_struct(name.clone(), &data);
//!
//!     // Make the Surfer
//!     let mut surfer = Surfer::try_from(builder).unwrap();
//!
//!     // Prepare your data or get it from somewhere
//!     let title = "The Old Man and the Sea".to_string();
//!     let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
//!     let old_man = OldMan::new(title, body);
//!
//!     // Insert the data so that store has only one document
//!     let _ = surfer.insert_struct(&name, &old_man).unwrap();
//!     println!("Inserting document: 1");
//!
//!     // Give some time to indexing to complete
//!     block_thread(1);
//!
//!     // Lets query our one document
//!     let query = "sea whale";
//!     let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
//!     // Check one document
//!     println!("Total documents found: {}", computed.len());
//!     assert_eq!(computed, vec![old_man.clone()]);
//!
//!     // Insert the data so that store has two document
//!     let _ = surfer.insert_struct(&name, &old_man).unwrap();
//!     println!("Inserting document: 1");
//!
//!     // Give some time for indexing to complete
//!     block_thread(1);
//!
//!     // Lets query again for two documents
//!     let query = "sea whale";
//!     let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
//!     // Check two documents
//!     println!("Total documents found: {}", computed.len());
//!     assert_eq!(computed, vec![old_man.clone(), old_man.clone()]);
//!
//!     // Lets add 50 more documents
//!     let mut i = 0;
//!     let mut documents = Vec::with_capacity(50);
//!     while i < 50 {
//!         documents.push(old_man.clone());
//!         i = i + 1;
//!     };
//!     let _ = surfer.insert_structs(&name, &documents).unwrap();
//!     println!("Inserting document: 50");
//!
//!     // Give some time for indexing to complete
//!     block_thread(2);
//!
//!     // Lets query again to get first 10 documents only (default settings)
//!     let query = "sea whale";
//!     let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
//!     // Check 10 documents
//!     println!("Total documents found: {} due to default limit = 10", computed.len());
//!     assert_eq!(computed.len(), 10);
//!
//!     // Lets query again to get all stored documents
//!     let query = "sea whale";
//!     let computed = surfer.read_structs::<OldMan>(&name, query, Some(100), None).unwrap().unwrap();
//!     // Check all 52 documents
//!     println!("Total documents found: {} with limit = 100", computed.len());
//!     assert_eq!(computed.len(), 52);
//!
//!     // Clean-up
//!     let path = surfer.which_index(&name).unwrap();
//!     let _ = remove_dir_all(&path);
//! }
//! ```

pub mod prelude;
pub mod seed;
pub mod errors;
pub mod utils;
pub mod registry;
pub mod fuzzy;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize};
    use serde_value;
    use std::collections::BTreeMap;
    use std::fmt;
    use tantivy::schema::{Schema, IntOptions, TEXT, STORED};
    use std::ops::{Deref, DerefMut};
    use std::fmt::Debug;

    #[derive(Serialize, Clone, PartialEq)]
    struct SchemaTest(Schema);

    impl Deref for SchemaTest {
        type Target = Schema;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for SchemaTest {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }

    impl Debug for SchemaTest {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let schema = &self.0;
            let x = schema.fields();
            let mut fields = Vec::new();
            for (field,_) in x {
                fields.push(field);
            }
            f.write_str(format!("{:?}", fields).as_str())
        }
    }

    impl SchemaTest {
        fn new(schema: Schema) -> Self {
            Self(schema)
        }
    }

    #[derive(Serialize)]
    struct Dummy {
        x: String,
        y: String,
        z: u64,
    }

    #[derive(Serialize)]
    struct Giant {
        a: String,
        b: bool,
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
        m: Vec<u8>,
    }


    #[test]
    fn validate_field_names() {
        let data = Dummy {
            x: "A".to_owned(),
            y: "B".to_owned(),
            z: 100,
        };

        let value = utils::as_value(&data).unwrap();
        let computed = utils::field_names(&value).unwrap();
        let expected = vec!["x", "y", "z"];
        assert_eq!(expected, computed);
    }

    #[test]
    fn validate_as_value() {
        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100,
        };

        let mut bmap = BTreeMap::new();
        bmap.insert(serde_value::Value::String("x".to_owned()), serde_value::Value::String("X".to_owned()));
        bmap.insert(serde_value::Value::String("y".to_owned()), serde_value::Value::String("Y".to_owned()));
        bmap.insert(serde_value::Value::String("z".to_owned()), serde_value::Value::U64(100));

        let expected = serde_value::Value::Map(bmap);
        let computed = utils::as_value(&data).unwrap();

        assert_eq!(expected, computed);
    }

    #[test]
    fn validate_basic_schema() {
        let data = Dummy {
            x: "X".to_owned(),
            y: "Y".to_owned(),
            z: 100u64,
        };

        let data = utils::as_value(&data).unwrap();

        let computed = utils::to_schema(&data, None).unwrap();
        let computed = SchemaTest::new(computed);

        let mut expected = Schema::builder();
        expected.add_text_field("x", TEXT | STORED);
        expected.add_text_field("y", TEXT | STORED);

        let options = IntOptions::default();
        let options = options.set_stored();
        let options = options.set_indexed();
        expected.add_u64_field("z", options);

        let expected = expected.build();
        let expected = SchemaTest::new(expected);

        assert_eq!(expected, computed);
    }

    #[test]
    fn validate_full_schema() {
        let a: String = "Empire Of The Clouds".to_string();
        let b: bool = true;
        let c: u64 = 1;
        let d: u32 = 1;
        let e: u16 = 1;
        let f: u8 = 1;
        let g: i64 = 1;
        let h: i32 = 1;
        let i: i16 = 1;
        let j: i8 = 1;
        let k: f64 = 1.0;
        let l: f32 = 1.0;
        let m: Vec<u8> = "The book of souls".as_bytes().to_vec();
        let data = Giant {
            a,
            b,
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
            m,
        };
        let data = utils::as_value(&data).unwrap();
        let computed = utils::to_schema(&data, None).unwrap();
        let computed = SchemaTest::new(computed);

        let mut expected = Schema::builder();
        expected.add_text_field("a", TEXT | STORED);
        expected.add_text_field("b", TEXT | STORED);

        let options = IntOptions::default();
        let options = options.set_stored();
        let options = options.set_indexed();
        expected.add_u64_field("c", options.clone());
        expected.add_u64_field("d", options.clone());
        expected.add_u64_field("e", options.clone());
        expected.add_u64_field("f", options.clone());

        expected.add_i64_field("g", options.clone());
        expected.add_i64_field("h", options.clone());
        expected.add_i64_field("i", options.clone());
        expected.add_i64_field("j", options.clone());
        expected.add_f64_field("k", options.clone());
        expected.add_f64_field("l", options.clone());
        expected.add_bytes_field("m");
        let expected = expected.build();
        let expected = SchemaTest::new(expected);
        assert_eq!(format!("{:?}", expected), format!("{:?}", computed));
        assert_eq!(expected, computed);
    }
}
