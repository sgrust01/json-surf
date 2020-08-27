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
//! use std::cmp::{Ord, Ordering, Eq};
//!
//! use serde::{Serialize, Deserialize};
//!
//! use json_surf::prelude::*;
//!
//! /// Main struct
//! #[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
//! struct UserInfo {
//!     first: String,
//!     last: String,
//!     age: u8,
//! }
//!
//! impl UserInfo {
//!     pub fn new(first: String, last: String, age: u8) -> Self {
//!         Self {
//!             first,
//!             last,
//!             age,
//!         }
//!     }
//! }
//!
//! impl Default for UserInfo {
//!     fn default() -> Self {
//!         let first = "".to_string();
//!         let last = "".to_string();
//!         let age = 0u8;
//!         UserInfo::new(first, last, age)
//!     }
//! }
//!
//!
//! fn main() {
//!     // Specify home location of indexes
//!     let home = ".store".to_string();
//!     let name = "users".to_string();
//!
//!     // Prepare builder
//!     let mut builder = SurferBuilder::default();
//!     builder.set_home(&home);
//!
//!     let data = UserInfo::default();
//!     builder.add_struct(name.clone(), &data);
//!
//!     // Prepare Surfer
//!     let mut surfer = Surfer::try_from(builder).unwrap();
//!
//!     // Prepare data to insert & search
//!
//!     // User 1: John Doe
//!     let first = "John".to_string();
//!     let last = "Doe".to_string();
//!     let age = 20u8;
//!     let john_doe = UserInfo::new(first, last, age);
//!
//!     // User 2: Jane Doe
//!     let first = "Jane".to_string();
//!     let last = "Doe".to_string();
//!     let age = 18u8;
//!     let jane_doe = UserInfo::new(first, last, age);
//!
//!     // User 3: Jonny Doe
//!     let first = "Jonny".to_string();
//!     let last = "Doe".to_string();
//!     let age = 10u8;
//!     let jonny_doe = UserInfo::new(first, last, age);
//!
//!     // User 4: Jinny Doe
//!     let first = "Jinny".to_string();
//!     let last = "Doe".to_string();
//!     let age = 10u8;
//!     let jinny_doe = UserInfo::new(first, last, age);
//!
//!     // Writing structs
//!
//!     // Option 1: One struct at a time
//!     let _ = surfer.insert_struct(&name, &john_doe).unwrap();
//!     let _ = surfer.insert_struct(&name, &jane_doe).unwrap();
//!
//!     // Option 2: Write all structs together
//!     let users = vec![jonny_doe.clone(), jinny_doe.clone()];
//!     let _ = surfer.insert_structs(&name, &users).unwrap();
//!
//!     // Reading structs
//!
//!     // Option 1: Full text search
//!     let expected = vec![john_doe.clone()];
//!     let computed = surfer.read_structs::<UserInfo>(&name, "John", None, None).unwrap().unwrap();
//!     assert_eq!(expected, computed);
//!
//!     let mut expected = vec![john_doe.clone(), jane_doe.clone(), jonny_doe.clone(), jinny_doe.clone()];
//!     expected.sort();
//!     let mut computed = surfer.read_structs::<UserInfo>(&name, "doe", None, None).unwrap().unwrap();
//!     computed.sort();
//!     assert_eq!(expected, computed);
//!
//!     // Option 2: Term search
//!     let mut expected = vec![jonny_doe.clone(), jinny_doe.clone()];
//!     expected.sort();
//!     let mut computed = surfer.read_stucts_by_field::<UserInfo>(&name, "age", "10", None, None).unwrap().unwrap();
//!     computed.sort();
//!     assert_eq!(expected, computed);
//!
//!     // Clean-up
//!     let path = surfer.which_index(&name).unwrap();
//!     let _ = remove_dir_all(&path);
//!     let _ = remove_dir_all(&home);
//! }
//!
//! /// Convenience method for sorting & likely not required in user code
//! impl Ord for UserInfo {
//!     fn cmp(&self, other: &Self) -> Ordering {
//!         if self.first == other.first && self.last == other.last {
//!             return Ordering::Equal;
//!         };
//!         if self.first == other.first {
//!             if self.last > self.last {
//!                 Ordering::Greater
//!             } else {
//!                 Ordering::Less
//!             }
//!         } else {
//!             if self.first > self.first {
//!                 Ordering::Greater
//!             } else {
//!                 Ordering::Less
//!             }
//!         }
//!     }
//! }
//!
//! /// Convenience method for sorting & likely not required in user code
//! impl Eq for UserInfo {}
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

        let (computed, _) = utils::to_schema(&data, None).unwrap();
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
        let (computed, _) = utils::to_schema(&data, None).unwrap();
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
