use serde_value;
use serde::{Serialize, Deserialize};
use tantivy::schema::Schema;
use tantivy::schema::FieldEntry;
use tantivy::schema::TextOptions;


#[derive(Serialize, Debug, Deserialize, PartialEq, PartialOrd, Clone)]
struct Container {
    buffer: Vec<u8>,
    labels: String,
}

impl Default for Container {
    fn default() -> Self {
        let buffer = "HelloWorld".to_string().into_bytes();
        let labels = "Tantivy Rocks".to_string();
        Self{
            buffer,
            labels,
        }
    }
}

fn main() {
    println!("This example is here to demonstrate that Tantivy does not support stored bytes yet");
    // Build Container
    let data = Container::default();
    let value = serde_value::to_value(data).expect("[Cough]");
    let json_doc = serde_json::to_string(&value).expect("[Cough Again]");
    println!("Json: {}", json_doc);

    // Build Schema
    let mut builder = Schema::builder();
    // Is there any alternate way or just not supported?
    // src/schema/field_type.rs:32 - FieldType maps to Type which say Vec<u8>
    let field_name = "buffer".to_string();
    let entry = FieldEntry::new_bytes(field_name);
    let _ = builder.add_field(entry);

    let text_options = TextOptions::default();
    let field_name = "labels".to_string();
    let entry = FieldEntry::new_text(field_name, text_options);
    let _ = builder.add_field(entry);

    let schema = builder.build();
    let document = schema.parse_document(&json_doc).expect("[Dies Coughing]");
    println!("Document: {:#?}", document);
}