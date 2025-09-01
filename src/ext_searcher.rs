use tantivy::{collector::TopDocs, query::{BooleanQuery, Occur, Query, RangeQuery, TermQuery}, schema::IndexRecordOption, Index, IndexReader, Term};

use crate::{package_meta::ExtensionMetadata, serve::extensions::{GetExtensionUpdatesParams, GetExtensionVersionsParams, GetExtensionsParams}};

#[derive(Clone)]
pub struct ExtSearcher {
    index: Index,
    reader: IndexReader,
}

impl ExtSearcher {
    pub fn init(index: Index) -> anyhow::Result<Self> {
        let reader = index.reader()?;

        Ok(Self {
            index,
            reader
        })
    }

    pub fn get_extension_updates(&self, params: &GetExtensionUpdatesParams) -> anyhow::Result<Vec<ExtensionMetadata>> {
        let searcher = self.reader.searcher();

        let mut sub_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        
        self.add_query_from_ids(&mut sub_queries, &params.ids)?;

        self.add_query_from_schema_version_range(&mut sub_queries, Some(params.min_schema_version), params.max_schema_version)?;

        let top_docs = searcher.search(&(Box::new(BooleanQuery::new(sub_queries)) as Box<dyn Query>), &TopDocs::with_limit(1000))?;

        let mut data = Vec::new();
        
        for (_score, doc_address) in top_docs {
            let doc: ExtensionMetadata = searcher.doc(doc_address)?;

            data.push(doc);
        }

        Ok(data)
    }

    pub fn get_extension_versions(&self, params: &GetExtensionVersionsParams) -> anyhow::Result<Vec<ExtensionMetadata>> {
        let searcher = self.reader.searcher();

        let mut sub_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        
        self.add_query_from_ids(&mut sub_queries, &params.extension_id)?;

        let top_docs = searcher.search(&(Box::new(BooleanQuery::new(sub_queries)) as Box<dyn Query>), &TopDocs::with_limit(1000))?;

        let mut data = Vec::new();
        
        for (_score, doc_address) in top_docs {
            let doc: ExtensionMetadata = searcher.doc(doc_address)?;

            data.push(doc);
        }

        Ok(data)
    }

    pub fn get_extensions(&self, params: &GetExtensionsParams) -> anyhow::Result<Vec<ExtensionMetadata>> {
        let searcher = self.reader.searcher();

        let mut sub_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        
        if let Some(filter) = &params.filter {
            self.add_query_from_filter(&mut sub_queries, filter)?;
        }

        if let Some(provides_filter) = &params.provides {
            self.add_query_from_provides(&mut sub_queries, provides_filter)?;
        }

        self.add_query_from_schema_version_range(&mut sub_queries, None, params.max_schema_version)?;

        let top_docs = searcher.search(&(Box::new(BooleanQuery::new(sub_queries)) as Box<dyn Query>), &TopDocs::with_limit(1000))?;

        let mut data = Vec::new();
        
        for (_score, doc_address) in top_docs {
            let doc: ExtensionMetadata = searcher.doc(doc_address)?;

            data.push(doc);
        }

        Ok(data)
    }
    
    fn add_query_from_schema_version_range(&self, sub_queries: &mut Vec<(Occur, Box<dyn Query>)>, min_schema_version: Option<i32>, max_schema_version: i32) -> anyhow::Result<()> {
        let schema_version_field = self.index.schema().get_field("schema_version")?;

        let lower = min_schema_version
            .map(|v| std::ops::Bound::Included(Term::from_field_i64(schema_version_field, v as i64)))
            .unwrap_or(std::ops::Bound::Unbounded);

        let upper = std::ops::Bound::Included(Term::from_field_i64(schema_version_field, max_schema_version as i64));

        sub_queries.push((Occur::Must, Box::new(BooleanQuery::new(vec![
            (Occur::Must, Box::new(RangeQuery::new(lower, upper)))
        ]))));

        Ok(())
    }

    fn add_query_from_provides(&self, sub_queries: &mut Vec<(Occur, Box<dyn Query>)>, provides_filter: &str) -> anyhow::Result<()> {
        let provides_field = self.index.schema().get_field("provides")?;

        let provides_filter = provides_filter
            .split(',')
            .map(|v| (Occur::Should, Box::new(TermQuery::new(Term::from_field_text(provides_field, v.trim()), IndexRecordOption::Basic)) as Box<dyn Query>))
            .collect::<Vec<(Occur, Box<dyn Query>)>>();
        
        if !provides_filter.is_empty() {
            sub_queries.push((Occur::Must, Box::new(BooleanQuery::new(vec![
                (Occur::Must, Box::new(BooleanQuery::new(
                    provides_filter
                )))
            ]))))
        }
        
        Ok(())
    }

    fn add_query_from_ids(&self, sub_queries: &mut Vec<(Occur, Box<dyn Query>)>, ids: &str) -> anyhow::Result<()> {
        let id_field = self.index.schema().get_field("id")?;
        
        let id_filter = ids
            .split(',')
            .map(|v| (Occur::Should, Box::new(TermQuery::new(Term::from_field_text(id_field, v.trim()), IndexRecordOption::Basic)) as Box<dyn Query>))
            .collect::<Vec<(Occur, Box<dyn Query>)>>();

        sub_queries.push((Occur::Must, Box::new(BooleanQuery::new(vec![
            (Occur::Must, Box::new(BooleanQuery::new(id_filter)))
        ]))));

        Ok(())
    }

    fn add_query_from_filter(&self, sub_queries: &mut Vec<(Occur, Box<dyn Query>)>, filter: &str) -> anyhow::Result<()> {
        let id_field = self.index.schema().get_field("id")?;
        let name_field = self.index.schema().get_field("name")?;
        
        sub_queries.push((Occur::Must, Box::new(BooleanQuery::new(vec![
            (Occur::Must, Box::new(BooleanQuery::new(
                vec![
                    (Occur::Should, Box::new(TermQuery::new(Term::from_field_text(name_field, filter), IndexRecordOption::Basic))),
                    (Occur::Should, Box::new(TermQuery::new(Term::from_field_text(id_field, &filter.to_lowercase()), IndexRecordOption::Basic)))
                ]
            )))
        ]))));

        Ok(())
    }
}