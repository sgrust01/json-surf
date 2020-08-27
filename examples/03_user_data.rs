use std::convert::TryFrom;
use std::fs::remove_dir_all;
use std::cmp::{Ord, Ordering, Eq};

use serde::{Serialize, Deserialize};

use base64::encode;

use json_surf::prelude::*;

#[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
struct User {
    full_name: String,
    user_id: String,
    buffer: String,
}

impl User {
    pub fn new(full_name: String, user_id: String, buffer: String) -> Self {
        Self {
            full_name,
            user_id,
            buffer,
        }
    }
}

impl Default for User {
    fn default() -> Self {
        let full_name = "".to_string();
        let user_id = "".to_string();
        let data = encode("".as_bytes());
        User::new(full_name, user_id, data)
    }
}

impl Eq for User {}


impl Ord for User {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.full_name == other.full_name {
            return Ordering::Equal;
        };
        if self.full_name > other.full_name {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    }
}



fn main() {
    let home = ".store".to_string();
    let name = "userdata".to_string();

    let mut builder = SurferBuilder::default();
    builder.set_home(&home);

    let data = User::default();
    builder.add_struct(name.clone(), &data);

    let mut surfer = Surfer::try_from(builder).unwrap();

    let john_doe_full_name = "John Doe".to_string();
    let john_doe_user_id = "john.doe.1".to_string();
    let john_doe_data = encode("Live some-where".as_bytes());
    let user_john_doe = User {
        full_name: john_doe_full_name,
        user_id: john_doe_user_id,
        buffer: john_doe_data,
    };

    let jane_doe_full_name = "Jane Doe".to_string();
    let jane_doe_user_id = "jane.doe.1".to_string();
    let jane_doe_data = encode("Live some-where else".as_bytes());

    let user_jane_doe = User {
        full_name: jane_doe_full_name,
        user_id: jane_doe_user_id,
        buffer: jane_doe_data,
    };

    let payload = vec![user_john_doe.clone(), user_jane_doe.clone()];
    let _ = surfer.insert_structs(&name, &payload).unwrap();
    println!("===========================");
    println!("Insert: John & Jane Doe");
    println!("---------------------------");
    println!("{:#?}", payload);
    println!("---------------------------");

    let query = "john";
    let users = surfer.read_structs::<User>(&name, query, None, None).unwrap();
    let users = users.unwrap();

    assert_eq!(users.len(), 1);
    let user = users.get(0).unwrap();
    assert_eq!(*user, user_john_doe);
    println!("================================");
    println!("Query: '{}' Select: John Doe",query);
    println!("--------------- -----------------");
    println!("{:#?}", user);
    println!("--------------------------------");

    // Clean-up
    let path = surfer.which_index(&name).unwrap();
    let _ = remove_dir_all(&path);
    let _ = remove_dir_all(home);
}