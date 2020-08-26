use std::convert::TryFrom;
use std::fs::remove_dir_all;
use std::cmp::{Ord, Ordering, Eq};

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;

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
            age
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

impl Eq for UserInfo {}


impl Ord for UserInfo {
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

    let data = UserInfo::default();
    builder.add_struct(name.clone(), &data);

    let mut surfer = Surfer::try_from(builder).unwrap();

    let first = "John".to_string();
    let last = "Doe".to_string();
    let age = 20u8;
    let john_doe = UserInfo::new(first, last, age);

    let first = "Jane".to_string();
    let last = "Doe".to_string();
    let age = 18u8;
    let jane_doe = UserInfo::new(first, last, age);

    let users = vec![john_doe.clone(), jane_doe.clone()];
    let _ = surfer.insert_structs(&name, &users).unwrap();
    println!("===========================");
    println!("Insert: John & Jane Doe");
    println!("---------------------------");
    println!("{:#?}", users);
    println!("---------------------------");


    println!("===========================");
    println!("Search users with Age = 20");
    println!("---------------------------");
    let query = "20";
    let mut computed = surfer.read_stucts_by_field::<UserInfo>(&name, "age", query, Some(100), None).unwrap().unwrap();
    computed.sort();
    let mut expected = vec![john_doe];
    expected.sort();
    assert_eq!(computed, expected);
    println!("{:#?}", computed);
    println!("---------------------------");


    println!("===========================");
    println!("Search users with Age = 18");
    println!("---------------------------");
    let query = "18";
    let mut computed = surfer.read_stucts_by_field::<UserInfo>(&name, "age", query, Some(100), None).unwrap().unwrap();
    computed.sort();
    let mut expected = vec![jane_doe];
    expected.sort();
    assert_eq!(computed, expected);
    println!("{:#?}", computed);
    println!("---------------------------");

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
}