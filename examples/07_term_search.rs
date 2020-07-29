use std::convert::TryFrom;
use std::fs::remove_dir_all;
use std::cmp::{Ord, Ordering, Eq};

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;

#[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
struct User {
    first: String,
    last: String,
    age: String,
}

impl User {
    pub fn new(first: String, last: String, age: String) -> Self {
        Self {
            first,
            last,
            age
        }
    }
}

impl Default for User {
    fn default() -> Self {
        let first = "".to_string();
        let last = "".to_string();
        let age = "Not specified".to_string();
        User::new(first, last, age)
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
    let age = "20".to_string();
    let john_doe = User::new(first, last, age);

    let first = "Jane".to_string();
    let last = "Doe".to_string();
    let age = "18".to_string();
    let jane_doe = User::new(first, last, age);

    let users = vec![john_doe.clone(), jane_doe.clone()];
    let _ = surfer.insert_structs(&name, &users).unwrap();
    println!("===========================");
    println!("Insert: John & Jane Doe");
    println!("---------------------------");
    println!("{:#?}", users);
    println!("---------------------------");


    let query = "20";
    let mut computed = surfer.read_stucts_by_field::<User>(&name, "age", query, Some(100), None).unwrap().unwrap();
    computed.sort();
    let mut expected = vec![john_doe];
    expected.sort();
    assert_eq!(computed, expected);

    let query = "18";
    let mut computed = surfer.read_stucts_by_field::<User>(&name, "age", query, Some(100), None).unwrap().unwrap();
    computed.sort();
    let mut expected = vec![jane_doe];
    expected.sort();
    assert_eq!(computed, expected);


    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
}