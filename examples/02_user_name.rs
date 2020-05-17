use std::convert::TryFrom;
use std::fs::remove_dir_all;
use std::cmp::{Ord, Ordering, Eq};

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;

#[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
struct User {
    first: String,
    last: String,
}

impl User {
    pub fn new(first: String, last: String) -> Self {
        Self {
            first,
            last,
        }
    }
}

impl Default for User {
    fn default() -> Self {
        let first = "".to_string();
        let last = "".to_string();
        User::new(first, last)
    }
}

impl Eq for User {}


impl Ord for User {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.first == other.first && self.last == other.last {
            return Ordering::Equal;
        };
        if self.first == other.first {
            if self.last > self.last {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            if self.first > self.first {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
    }
}

fn home_and_random_index_name() -> (String, String) {
    let home = ".store/examples".to_string();
    let name = random_string(None);
    (home, name)
}

fn main() {
    let (home, name) = home_and_random_index_name();

    let mut builder = SurferBuilder::default();
    builder.set_home(&home);

    let data = User::default();
    builder.add_struct(name.clone(), &data);

    let mut surfer = Surfer::try_from(builder).unwrap();

    let first = "John".to_string();
    let last = "Doe".to_string();
    let john_doe = User::new(first, last);

    let first = "Jane".to_string();
    let last = "Doe".to_string();
    let jane_doe = User::new(first, last);

    let users = vec![john_doe.clone(), jane_doe.clone()];
    let _ = surfer.insert_structs(&name, &users).unwrap();
    println!("===========================");
    println!("Insert: John & Jane Doe");
    println!("---------------------------");
    println!("{:#?}", users);
    println!("---------------------------");

    block_thread(1);

    let query = "john";
    let computed = surfer.read_structs::<User>(&name, query, Some(100), None).unwrap().unwrap();
    assert_eq!(computed, [john_doe.clone()]);
    println!("================================");
    println!("Query: '{}' Select: John Doe",query);
    println!("--------------------------------");
    println!("{:#?}", computed);
    println!("--------------------------------");


    let query = "jane";
    let computed = surfer.read_structs::<User>(&name, query, Some(100), None).unwrap().unwrap();
    assert_eq!(computed, [jane_doe.clone()]);

    println!("================================");
    println!("Query: '{}' Select: Jane Doe", query);
    println!("--------------------------------");
    println!("{:#?}", computed);
    println!("--------------------------------");

    let query = "doe";
    let mut computed = surfer.read_structs::<User>(&name, query, Some(100), None).unwrap().unwrap();
    computed.sort();
    let mut expected = vec![john_doe.clone(), jane_doe.clone()];
    expected.sort();
    assert_eq!(computed, expected);

    println!("====================================");
    println!("Query: '{}' Select: Jane & John Doe", query);
    println!("------------------------------------");
    println!("{:#?}", computed);
    println!("------------------------------------");

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
}