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

Please do check [tantivy](https://github.com/tantivy-search/tantivy) out. Absolute stunner!!! 

This crate is just a convenience crate over [tantivy](https://github.com/tantivy-search/tantivy). 

## TODO
* Add more examples
* Cleanup tmp folder left behind from test cases
* Remove any further copy
* Introduce more housekeeping API (If required)

## Features
* Full text search
* Serialize __**flat**__ JSON/Struct
* Easy write and read API
* Write multiple documents together
* Depends mostly on stable crates
* Support fuzzy word search (see examples)
* Requires no runtime
* No unsafe block
* Run on rust stable

## Motivation
* Allow your existing flat rust structs to be searched
* Encoded/Decoded byte streams can be stored along side too, as base64 encoded string
* This can just act as a container to actual data in databases, keeping indexes light
* Create time-aware containers which could possibly updated/deleted
* Create ephemeral storage for request/response
* This can integrate with any web-server to index and search near real-time
* This crate will focus mostly on user-workflow(s) related problem(s)
* Again, [tantivy](https://github.com/tantivy-search/tantivy) is a great library, do check it out

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

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;

/// Document to be indexed and searched
#[derive(Serialize, Debug, Deserialize, PartialEq, Clone)]
struct OldMan {
    title: String,
    body: String,
}

impl OldMan {
    pub fn new(title: String, body: String) -> Self {
        Self {
            title,
            body,
        }
    }
}

/// Convenience implementation for bootstraping the builder
impl Default for OldMan {
    fn default() -> Self {
        let title = "".to_string();
        let body = "".to_string();
        OldMan::new(title, body)
    }
}

/// Convenience method to keep indexes tucked under a directory
fn home_and_random_index_name() -> (String, String) {
    let home = ".store/examples".to_string();
    let name = random_string(None);
    (home, name)
}

fn main() {
    let (home, name) = home_and_random_index_name();

    // Mostly empty but can be a real flat struct
    let data = OldMan::default();

    // Prepare the builder instance
    let mut builder = SurferBuilder::default();
    // By default everything goes to directory indexes
    builder.set_home(&home);
    builder.add_struct(name.clone(), &data);

    // Make the Surfer
    let mut surfer = Surfer::try_from(builder).unwrap();

    // Prepare your data or get it from somewhere
    let title = "The Old Man and the Sea".to_string();
    let body = "He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.".to_string();
    let old_man = OldMan::new(title, body);

    // Insert the data so that store has only one document
    let _ = surfer.insert_struct(&name, &old_man).unwrap();
    println!("Inserting document: 1");

    // Give some time to indexing to complete
    block_thread(1);

    // Lets query our one document
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
    // Check one document
    println!("Total documents found: {}", computed.len());
    assert_eq!(computed, vec![old_man.clone()]);

    // Insert the data so that store has two document
    let _ = surfer.insert_struct(&name, &old_man).unwrap();
    println!("Inserting document: 1");

    // Give some time for indexing to complete
    block_thread(1);

    // Lets query again for two documents
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
    // Check two documents
    println!("Total documents found: {}", computed.len());
    assert_eq!(computed, vec![old_man.clone(), old_man.clone()]);

    // Lets add 50 more documents
    let mut i = 0;
    let mut documents = Vec::with_capacity(50);
    while i < 50 {
        documents.push(old_man.clone());
        i = i + 1;
    };
    let _ = surfer.insert_structs(&name, &documents).unwrap();
    println!("Inserting document: 50");

    // Give some time for indexing to complete
    block_thread(2);

    // Lets query again to get first 10 documents only (default settings)
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
    // Check 10 documents
    println!("Total documents found: {} due to default limit = 10", computed.len());
    assert_eq!(computed.len(), 10);

    // Lets query again to get all stored documents
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, Some(100), None).unwrap().unwrap();
    // Check all 52 documents
    println!("Total documents found: {} with limit = 100", computed.len());
    assert_eq!(computed.len(), 52);

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
}
```