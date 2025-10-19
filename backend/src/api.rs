use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    response::Response,
};
use axum_extra::extract::Multipart;
use tokio_util::io::ReaderStream;
use tokio::io::AsyncWriteExt;
use std::sync::Arc;
use std::borrow::Cow;
use bytes::Bytes;

use crate::xmap::{XmapCache, XmapFileSet, hash_content, stream_matches_multi};

/// Streams XMAP matches for uploaded files
///
/// # Arguments
/// * `cache` - Shared XmapCache instance
/// * `multipart` - Multipart form data containing XMAP files
///
/// # Returns
/// * `Result<Response<Body>, StatusCode>` - Streaming response or error status
///
/// # Process
/// 1. Extracts 2-3 XMAP files from multipart form
/// 2. Parses files and builds indices
/// 3. Streams matches via duplex channel
/// 4. Caches results for future requests
pub async fn stream_xmap_matches(
    State(cache): State<Arc<XmapCache>>,
    mut multipart: Multipart,
) -> Result<Response<Body>, StatusCode> {
    let mut files: Vec<(String, Bytes)> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        files.push((name, bytes));
    }

    if files.is_empty() || files.len() > 3 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if files.len() == 1 {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/octet-stream")
            .body(Body::empty())
            .unwrap());
    }

    let mut file_hashes = Vec::with_capacity(files.len());
    let mut file_records = Vec::with_capacity(files.len());

    for (name, bytes) in files {
        let content_str = std::str::from_utf8(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
        let hash = hash_content(content_str);
        file_hashes.push(hash);

        let bytes_arc = Arc::new(bytes);
        let records = tokio::task::spawn_blocking({
            let cache = Arc::clone(&cache);
            let content = Arc::clone(&bytes_arc);
            move || {
                let s: Cow<str> = Cow::Borrowed(std::str::from_utf8(&content).unwrap());
                cache.get_or_parse(hash, &s)
            }
        })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        file_records.push(records);
    }

    let mut all_records_with_indices = Vec::with_capacity(file_records.len());

    for (idx, records) in file_records.into_iter().enumerate() {
        if idx == 0 {
            all_records_with_indices.push(records);
        } else {
            let hash = file_hashes[idx];
            tokio::task::spawn_blocking({
                let cache = Arc::clone(&cache);
                let records_clone = Arc::clone(&records);
                move || cache.get_or_build_index(hash, records_clone)
            })
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            all_records_with_indices.push(records);
        }
    }

    let fileset = Arc::new(XmapFileSet::new(
        all_records_with_indices.into_boxed_slice()
    ));

    let (mut writer, reader) = tokio::io::duplex(131072);

    let cache_key = file_hashes.into_boxed_slice();
    tokio::spawn(async move {
        let rx = stream_matches_multi(fileset);

        while let Ok(match_data) = rx.recv() {
            let match_arc = Arc::new(match_data.clone());
            cache.cache_match(cache_key.clone(), match_arc);

            if let Ok(bytes) = bincode::serialize(&match_data) {
                let len = (bytes.len() as u32).to_le_bytes();
                if writer.write_all(&len).await.is_err() { break; }
                if writer.write_all(&bytes).await.is_err() { break; }
            } else {
                break;
            }
        }
    });

    let stream = ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    Ok(Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header("Cache-Control", "no-cache")
        .header("X-Content-Type-Options", "nosniff")
        .body(body)
        .unwrap())
}