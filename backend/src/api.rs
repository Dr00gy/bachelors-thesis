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

use crate::xmap::{XmapCache, hash_content, stream_matches};

pub async fn stream_xmap_matches(
    State(cache): State<Arc<XmapCache>>,
    mut multipart: Multipart,
) -> Result<Response<Body>, StatusCode> {
    let mut chm13_bytes: Option<Bytes> = None;
    let mut hg38_bytes: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        let bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

        if name.to_lowercase().contains("chm13") {
            chm13_bytes = Some(bytes);
        } else if name.to_lowercase().contains("hg38") {
            hg38_bytes = Some(bytes);
        }
    }

    let chm13_bytes = chm13_bytes.ok_or(StatusCode::BAD_REQUEST)?;
    let hg38_bytes = hg38_bytes.ok_or(StatusCode::BAD_REQUEST)?;

    let chm13_hash = hash_content(std::str::from_utf8(&chm13_bytes).map_err(|_| StatusCode::BAD_REQUEST)?);
    let hg38_hash = hash_content(std::str::from_utf8(&hg38_bytes).map_err(|_| StatusCode::BAD_REQUEST)?);
    
    let chm13_bytes = Arc::new(chm13_bytes);
    let hg38_bytes = Arc::new(hg38_bytes);

    let chm13_records = tokio::task::spawn_blocking({
        let cache = Arc::clone(&cache);
        let content = Arc::clone(&chm13_bytes);
        move || { // copy obsession
            let s: Cow<str> = Cow::Borrowed(std::str::from_utf8(&content).unwrap());
            cache.get_or_parse(chm13_hash, &s)
        }
    })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let hg38_records = tokio::task::spawn_blocking({
        let cache = Arc::clone(&cache);
        let content = Arc::clone(&hg38_bytes);
        move || {
            let s: Cow<str> = Cow::Borrowed(std::str::from_utf8(&content).unwrap());
            cache.get_or_parse(hg38_hash, &s)
        }
    })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let hg38_index = tokio::task::spawn_blocking({
        let cache = Arc::clone(&cache);
        let records = Arc::clone(&hg38_records);
        move || cache.get_or_build_index(hg38_hash, records)
    })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // streaming pipe
    let (mut writer, reader) = tokio::io::duplex(131072);

    tokio::spawn(async move {
        let rx = stream_matches(chm13_records, hg38_index);

        while let Ok(match_data) = rx.recv() {
            let match_arc = Arc::new(match_data.clone());
            cache.cache_match((chm13_hash, hg38_hash), match_arc);

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
