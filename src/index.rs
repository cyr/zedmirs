use std::path::PathBuf;

use tantivy::{schema::{Schema, SchemaBuilder, INDEXED, STORED, STRING, TEXT}, Index, IndexWriter, TantivyDocument};
use tokio::fs::create_dir_all;

use crate::{package_meta::ExtensionListData, progress::Progress};


pub struct Indexer {
    index: Index
}

impl Indexer {
    pub async fn init(output: &str) -> anyhow::Result<Self> {
        let schema = schema();

        let idx_path = PathBuf::from(format!("{output}/.tmp/idx"));

        create_dir_all(idx_path).await?;

        let index = Index::create_in_dir(format!("{output}/.tmp/idx"), schema.clone())?;

        Ok(Self {
            index
        })
    }

    pub fn index(&self, data: ExtensionListData, progress: Progress) -> anyhow::Result<()> {
        let mut index_writer: IndexWriter = self.index.writer(15_000_000)?;

        let schema = self.index.schema();

        for mut package_meta in data.data {
            if package_meta.get("wasm_api_version").map(|v| v.is_null()).unwrap_or(false) {
                package_meta.remove("wasm_api_version");
            }

            let doc = TantivyDocument::from_json_object(&schema, package_meta)?;
                

            index_writer.add_document(doc)?;

            progress.files.inc_success(1);
        }

        index_writer.commit()?;

        Ok(())
    }
}

fn schema() -> Schema {
    let mut builder = SchemaBuilder::new();

    builder.add_text_field("id", STORED | STRING);
    builder.add_text_field("name", STORED | TEXT);
    builder.add_text_field("version", STORED | STRING);
    builder.add_text_field("description", STORED | STRING);
    builder.add_text_field("authors", STORED | TEXT);
    builder.add_text_field("repository", STORED | TEXT);
    builder.add_i64_field("schema_version", STORED | INDEXED);
    builder.add_text_field("wasm_api_version", STORED | TEXT);
    builder.add_text_field("provides", STORED | STRING);
    builder.add_text_field("published_at", STORED | STRING);
    builder.add_u64_field("download_count", STORED | INDEXED);

    builder.build()
}