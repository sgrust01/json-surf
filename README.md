<div align="center">
 <p><h1>JSON-Surf</h1> </p>
  <p><strong>Search/Analyze JSON and Rust Struct</strong> </p>
</div>

[![Build Status](https://travis-ci.org/sgrust01/json-surf.svg?branch=master)](https://travis-ci.org/sgrust01/json-surf)
[![codecov](https://codecov.io/gh/sgrust01/json-surf/branch/master/graph/badge.svg)](https://codecov.io/gh/sgrust01/json-surf)
[![Version](https://img.shields.io/badge/rustc-1.43.1+-blue.svg)](https://blog.rust-lang.org/2020/05/07/Rust.1.43.1.html) 
![RepoSize](https://img.shields.io/github/repo-size/sgrust01/json-surf)
![Crates.io](https://img.shields.io/crates/l/json-surf)
![Crates.io](https://img.shields.io/crates/v/json-surf)
![Crates.io](https://img.shields.io/crates/d/json-surf)
![Contributors](https://img.shields.io/github/contributors/sgrust01/json-surf)
[![Gitter](https://badges.gitter.im/json-surf/community.svg)](https://gitter.im/json-surf/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)


## Features
* Full text search
* Serialize __**flat**__ JSON/Struct
* Easy write and read API
* Write multiple documents together
* Support fuzzy word search (see examples)
* Support Term query
* Requires no runtime
* No unsafe block
* Run on rust stable
* Coming Soon: Bi-gram suggestion & TF-IDF support

## Motivation
* Allow your existing flat rust structs to be searched
* Encoded/Decoded byte streams can be stored along side too, as base64 encoded string
* The crate will support arbitary byte stream once it is supported by tantivy (see [here](https://github.com/tantivy-search/tantivy/issues/832))
* This can just act as a container to actual data in databases, keeping indexes light
* Create time-aware containers which could possibly updated/deleted
* Create ephemeral storage for request/response
* This can integrate with any web-server to index and search near real-time
* This crate is just a convenience crate over [tantivy](https://github.com/tantivy-search/tantivy). 
* This crate will focus mostly on user-workflow(s) related problem(s)

## TODO
* Add more examples
* Cleanup tmp folder left behind from test cases
* Remove any further copy
* Introduce more housekeeping API (If required)


## Quickstart

### Prerequisite:

 ```toml
  [dependencies]
  json-surf = "*"
```

### Example
```rust
use std::convert::TryFrom;
use std::fs::remove_dir_all;
use std::cmp::{Ord, Ordering, Eq};

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;

/// Main struct
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


fn main() {
    // Specify home location of indexes
    let home = ".store".to_string();
    let name = "users".to_string();

    // Prepare builder
    let mut builder = SurferBuilder::default();
    builder.set_home(&home);

    let data = UserInfo::default();
    builder.add_struct(name.clone(), &data);

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
    let _ = surfer.insert_struct(&name, &john_doe).unwrap();
    let _ = surfer.insert_struct(&name, &jane_doe).unwrap();

    // Option 2: Write all structs together
    let users = vec![jonny_doe.clone(), jinny_doe.clone()];
    let _ = surfer.insert_structs(&name, &users).unwrap();

    // Reading structs

    // Option 1: Full text search
    let expected = vec![john_doe.clone()];
    let computed = surfer.read_structs::<UserInfo>(&name, "John", None, None).unwrap().unwrap();
    assert_eq!(expected, computed);

    let mut expected = vec![john_doe.clone(), jane_doe.clone(), jonny_doe.clone(), jinny_doe.clone()];
    expected.sort();
    let mut computed = surfer.read_structs::<UserInfo>(&name, "doe", None, None).unwrap().unwrap();
    computed.sort();
    assert_eq!(expected, computed);

    // Option 2: Term search
    let mut expected = vec![jonny_doe.clone(), jinny_doe.clone()];
    expected.sort();
    let mut computed = surfer.read_stucts_by_field::<UserInfo>(&name, "age", "10", None, None).unwrap().unwrap();
    computed.sort();
    assert_eq!(expected, computed);

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
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
```