use serde::{Deserialize, Serialize};
use serde_json::Map;
use tantivy::schema::{document::{DeserializeError, DocumentDeserialize, DocumentDeserializer}, OwnedValue};

#[derive(Deserialize)]
pub struct ExtensionListData {
    pub data: Vec<Map<String, serde_json::Value>>
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct ExtensionMetadata {
    pub id: String,
    pub published_at: String,
    pub download_count: u64,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub repository: String,
    pub schema_version: Option<i32>,
    pub wasm_api_version: Option<String>,
    pub provides: Vec<String>,
}



impl DocumentDeserialize for ExtensionMetadata {
    fn deserialize<'de, D>(mut deserializer: D) -> Result<Self, DeserializeError>
    where D: DocumentDeserializer<'de> {
        let mut doc = ExtensionMetadata::default();

        // TODO: Deserializing into OwnedValue is wasteful. The deserializer should be able to work
        // on slices and referenced data.
        while let Some((field, value)) = deserializer.next_field::<OwnedValue>()? {
            match field.field_id() {
                //id
                0 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("id is not str")))
                    };

                    doc.id = s;
                }
                //name
                1 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("name is not str")))
                    };

                    doc.name = s;
                }
                //version
                2 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("version is not str")))
                    };

                    doc.version = s;
                }
                //description
                3 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("description is not str")))
                    };

                    doc.description = Some(s);
                }
                //authors
                4 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("authors is not str")))
                    };

                    doc.authors.push(s);
                }
                //repository
                5 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("repository is not str")))
                    };

                    doc.repository = s;
                }
                //schema_version
                6 => {
                    let OwnedValue::I64(s) = value else {
                        return Err(DeserializeError::Custom(String::from("schema_version is not i64")))
                    };

                    doc.schema_version = Some(s as i32);
                }
                //wasm_api_version
                7 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("wasm_api_version is not str")))
                    };

                    doc.wasm_api_version = Some(s);
                }
                //provides
                8 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("provides is not str")))
                    };

                    doc.provides.push(s);
                }
                //published_at
                9 => {
                    let OwnedValue::Str(s) = value else {
                        return Err(DeserializeError::Custom(String::from("published_at is not str")))
                    };

                    doc.published_at = s;
                }
                //download_count
                10 => {
                    let OwnedValue::U64(s) = value else {
                        return Err(DeserializeError::Custom(String::from("schema_version is not i64")))
                    };

                    doc.download_count = s;
                }
                x => return Err(DeserializeError::Custom(format!("unexpected field_id {x}")))
            }
        }
        Ok(doc)
    }
}