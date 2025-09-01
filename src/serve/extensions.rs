use axum::{body::Body, extract::{Path, Query, State}, response::IntoResponse, routing::get, Json, Router};
use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};
use tokio_util::io::ReaderStream;

use crate::{package_meta::ExtensionMetadata, serve::AppState};

pub fn get_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/extensions", get(get_extensions))
        .route("/extensions/updates", get(get_extension_updates))
        .route("/extensions/:extension_id", get(get_extension_versions))
        .route(
            "/extensions/:extension_id/download",
            get(download_latest_extension),
        )
        .route(
            "/extensions/:extension_id/:version/download",
            get(download_extension),
        )
        .with_state(state)
}

#[derive(Serialize)]
pub struct GetExtensionsResult {
    data: Vec<ExtensionMetadata>
}

#[derive(Debug, Deserialize)]
pub struct GetExtensionsParams {
    pub filter: Option<String>,
    #[serde(default)]
    pub provides: Option<String>,
    #[serde(default)]
    pub max_schema_version: i32,
}

async fn get_extensions(State(state): State<AppState>, Query(params): Query<GetExtensionsParams>) -> Result<Json<GetExtensionsResult>, StatusCode> {
    let data = match state.searcher.get_extensions(&params) {
        Ok(v) => v,
        Err(e) => {
            crate::log(format!("WARN {e}"));
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };
    
    Ok(Json(GetExtensionsResult { data }))
}

#[derive(Debug, Deserialize)]
pub struct GetExtensionUpdatesParams {
    pub ids: String,
    pub min_schema_version: i32,
    pub max_schema_version: i32,
    //pub min_wasm_api_version: String,
    //pub max_wasm_api_version: String,
}//

async fn get_extension_updates(State(state): State<AppState>, Query(params): Query<GetExtensionUpdatesParams>) -> Result<Json<GetExtensionsResult>, StatusCode> {
    let data = match state.searcher.get_extension_updates(&params) {
        Ok(v) => v,
        Err(e) => {
            crate::log(format!("WARN {e}"));
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };
    
    Ok(Json(GetExtensionsResult { data }))
}

#[derive(Debug, Deserialize)]
pub struct GetExtensionVersionsParams {
    pub extension_id: String,
}

async fn get_extension_versions(State(state): State<AppState>, Path(params): Path<GetExtensionVersionsParams>) -> Result<Json<GetExtensionsResult>, StatusCode> {
    let data = match state.searcher.get_extension_versions(&params) {
        Ok(v) => v,
        Err(e) => {
            crate::log(format!("WARN {e}"));
            return Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };
    
    Ok(Json(GetExtensionsResult { data }))
}

#[derive(Debug, Deserialize)]
struct DownloadLatestExtensionPathParams {
    extension_id: String,
}

/* 
#[derive(Debug, Deserialize)]
struct DownloadLatestExtensionQueryParams {
    min_schema_version: Option<i32>,
    max_schema_version: Option<i32>,
    min_wasm_api_version: Option<String>,
    max_wasm_api_version: Option<String>,
}*/

async fn download_latest_extension(State(state): State<AppState>, Path(params): Path<DownloadLatestExtensionPathParams>) -> Result<impl IntoResponse, StatusCode> {
    let file_path = format!("{}/extensions/{}/archive.tar.gz", state.output, params.extension_id);

    let Ok(file) = tokio::fs::File::open(file_path).await else {
        return Err(StatusCode::NOT_FOUND)
    };

    let header = [
        (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
        (header::CONTENT_DISPOSITION, "attachment; filename=archive.tar.gz".to_owned())
    ];

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((header, body))
}

#[derive(Debug, Deserialize)]
struct DownloadExtensionParams {
    extension_id: String,
    version: String,
}

async fn download_extension(State(state): State<AppState>, Path(params): Path<DownloadExtensionParams>) -> Result<impl IntoResponse, StatusCode> {
    let file_path = format!("{}/extensions/{}/{}/archive.tar.gz", state.output, params.extension_id, params.version);

    let Ok(file) = tokio::fs::File::open(file_path).await else {
        return Err(StatusCode::NOT_FOUND)
    };
    
    let header = [
        (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
        (header::CONTENT_DISPOSITION, "attachment; filename=archive.tar.gz".to_owned())
    ];

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Ok((header, body))
}