use std::convert::TryFrom;
use std::hash::{Hash, Hasher};
use std::cmp::{Ord, Ordering, Eq};


use std::fs::remove_dir_all;

use serde::{Serialize, Deserialize};

use json_surf::prelude::*;


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

fn main() {
    // Specify home location for indexes
    let home = ".store".to_string();
    // Specify index name
    let index_name = "usage".to_string();

    // Prepare builder
    let mut builder = SurferBuilder::default();
    builder.set_home(&home);

    let data = UserInfo::default();
    builder.add_struct(index_name.clone(), &data);

    // Prepare Surf
    let mut surf = Surf::try_from(builder).unwrap();

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

    // See examples for more options
    let users = vec![john_doe.clone(), jane_doe.clone()];
    let _ = surf.insert(&index_name, &users).unwrap();

    block_thread(1);

    // See examples for more options
    // Similar to SELECT * FROM users WHERE (age = 20 AND last = "Doe") OR (first = "Jane")
    let conditions = vec![
        // (age = 20 AND last = "Doe")
        OrCondition::new(
            vec![
                AndCondition::new("age".to_string(), "20".to_string()),
                AndCondition::new("last".to_string(), "doe".to_string())
            ]),
        // (first = "Jane")
        OrCondition::new(
            vec![
                AndCondition::new("first".to_string(), "jane".to_string())
            ])
    ];

    // Validate John and Jane Doe
    let mut computed = surf.select::<UserInfo>(&index_name, &conditions).unwrap().unwrap();
    let mut expected = vec![john_doe.clone(), jane_doe.clone(), ];
    expected.sort();
    computed.sort();
    assert_eq!(expected, computed);

    // Validated John's record - Alternate shortcut for query using one field only
    let computed = surf.read_all_structs_by_field(&index_name, "age", "20");
    let computed: Vec<UserInfo> = computed.unwrap().unwrap();
    assert_eq!(vec![john_doe], computed);

    // Delete John's record
    let result = surf.delete(&index_name, "age", "20");
    assert!(result.is_ok());

    // John's was removed
    let computed = surf.read_all_structs_by_field(&index_name, "age", "20");
    let computed: Vec<UserInfo> = computed.unwrap().unwrap();
    assert!(computed.is_empty());


    // Clean-up
    let path = surf.which_index(&index_name).unwrap();
    let _ = remove_dir_all(&path);
    let _ = remove_dir_all(&home);
}

/// Ignore all of this
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

/// Ignore all of this
/// Convenience method for sorting & likely not required in user code
impl Eq for UserInfo {}

/// Ignore all of this
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