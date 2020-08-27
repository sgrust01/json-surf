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

fn main() {
    let home = ".store".to_string();
    let name = "tantivy".to_string();

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

    // Insert the data so that store as only one document
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

    // Insert the data so that store as two document
    let _ = surfer.insert_struct(&name, &old_man).unwrap();
    println!("Inserting document: 1");

    // Give some time to indexing to complete
    block_thread(1);

    // Lets query again for two documents
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
    // Check two documents
    println!("Total documents found: {}", computed.len());
    assert_eq!(computed, vec![old_man.clone(), old_man.clone()]);

    // Lets add 100 more documents
    let mut i = 0;
    let mut documents = Vec::with_capacity(50);
    while i < 50 {
        documents.push(old_man.clone());
        i = i + 1;
    };
    let _ = surfer.insert_structs(&name, &documents).unwrap();
    println!("Inserting document: 50");

    // Give some time to indexing to complete
    block_thread(2);

    // Lets query again for to get first 10 only
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, None, None).unwrap().unwrap();
    // Check 10 documents
    println!("Total documents found: {} due to default limit = 10", computed.len());
    assert_eq!(computed.len(), 10);

    // Lets query again for to get first 10 only
    let query = "sea whale";
    let computed = surfer.read_structs::<OldMan>(&name, query, Some(100), None).unwrap().unwrap();
    // Check 10 documents
    println!("Total documents found: {} with limit = 100", computed.len());
    assert_eq!(computed.len(), 52);

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
    let _ = remove_dir_all(&home);
}