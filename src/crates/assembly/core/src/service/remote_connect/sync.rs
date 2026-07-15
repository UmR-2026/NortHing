//! Sub-domain: file sync (upload/download).
//! Spec step-3.7 — extracted from remote_connect/mod.rs (R55e refactor).

use super::*;
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// File metadata used for the incremental upload check.
#[derive(serde::Serialize)]
pub(super) struct FileManifestEntry {
    path: String,
    hash: String,
    size: u64,
}

/// Collected file data ready for upload.
pub(super) struct CollectedFile {
    rel_path: String,
    content: Vec<u8>,
    hash: String,
}

const MAX_UPLOAD_BATCH_BASE64_BYTES: usize = 256 * 1024;

pub(super) async fn upload_mobile_web(relay_url: &str, room_id: &str, web_dir: &str) -> Result<()> {
    let base = std::path::Path::new(web_dir);
    if !base.join("index.html").exists() {
        return Err(anyhow::anyhow!("mobile-web dir missing index.html: {}", web_dir));
    }

    let mut all_files: Vec<CollectedFile> = Vec::new();
    collect_files_with_hash(base, base, &mut all_files)?;

    info!(
        "Collected {} mobile-web files ({} bytes total) for room {room_id}",
        all_files.len(),
        all_files.iter().map(|f| f.content.len()).sum::<usize>()
    );

    let client = reqwest::Client::new();
    let relay_base = relay_url.trim_end_matches('/');

    // Step 1: try incremental check
    let manifest: Vec<FileManifestEntry> = all_files
        .iter()
        .map(|f| FileManifestEntry {
            path: f.rel_path.clone(),
            hash: f.hash.clone(),
            size: f.content.len() as u64,
        })
        .collect();

    let check_url = format!("{relay_base}/api/rooms/{room_id}/check-web-files");
    let check_result = client
        .post(&check_url)
        .json(&serde_json::json!({ "files": manifest }))
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await;

    match check_result {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| anyhow::anyhow!("parse check-web-files response: {e}"))?;
            let needed: Vec<String> = body["needed"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let existing = body["existing_count"].as_u64().unwrap_or(0);
            let total = body["total_count"].as_u64().unwrap_or(0);
            if needed.is_empty() {
                info!("All {total} files already exist on relay server, no upload needed");
                return Ok(());
            }

            info!(
                "Incremental upload: {existing}/{total} files already on server, uploading {} needed",
                needed.len()
            );

            upload_needed_files(&client, relay_base, room_id, &all_files, &needed).await
        }
        Ok(resp) if resp.status().as_u16() == 404 => {
            info!("Relay server does not support incremental upload, falling back to full upload");
            upload_all_files(&client, relay_base, room_id, &all_files).await
        }
        Ok(resp) => {
            let status = resp.status();
            info!("check-web-files returned HTTP {status}, falling back to full upload");
            upload_all_files(&client, relay_base, room_id, &all_files).await
        }
        Err(e) => {
            info!("check-web-files request failed ({e}), falling back to full upload");
            upload_all_files(&client, relay_base, room_id, &all_files).await
        }
    }
}

/// Upload only the files that the server said it needs.
async fn upload_needed_files(
    client: &reqwest::Client,
    relay_base: &str,
    room_id: &str,
    all_files: &[CollectedFile],
    needed: &[String],
) -> Result<()> {
    let needed_set: std::collections::HashSet<&str> = needed.iter().map(|s| s.as_str()).collect();

    let mut files_payload: Vec<(String, serde_json::Value, usize)> = Vec::new();
    for f in all_files {
        if needed_set.contains(f.rel_path.as_str()) {
            let encoded = B64.encode(&f.content);
            let encoded_len = encoded.len();
            files_payload.push((
                f.rel_path.clone(),
                serde_json::json!({
                    "content": encoded,
                    "hash": f.hash,
                }),
                encoded_len,
            ));
        }
    }

    let url = format!("{relay_base}/api/rooms/{room_id}/upload-web-files");
    let total_b64_bytes: usize = files_payload.iter().map(|(_, _, len)| *len).sum();

    info!(
        "Uploading {} needed files ({} bytes base64) to {url}",
        files_payload.len(),
        total_b64_bytes
    );

    let mut current_batch: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
    let mut current_batch_b64_bytes = 0usize;
    let mut batch_index = 0usize;
    for (path, entry, entry_len) in files_payload {
        let should_flush =
            !current_batch.is_empty() && current_batch_b64_bytes + entry_len > MAX_UPLOAD_BATCH_BASE64_BYTES;
        if should_flush {
            upload_web_files_batch(
                client,
                &url,
                room_id,
                batch_index,
                &current_batch,
                current_batch_b64_bytes,
            )
            .await?;
            batch_index += 1;
            current_batch = std::collections::HashMap::new();
            current_batch_b64_bytes = 0;
        }
        current_batch.insert(path, entry);
        current_batch_b64_bytes += entry_len;
    }

    if !current_batch.is_empty() {
        upload_web_files_batch(
            client,
            &url,
            room_id,
            batch_index,
            &current_batch,
            current_batch_b64_bytes,
        )
        .await?;
    }

    Ok(())
}

/// Fallback: upload all files using the legacy endpoint.
async fn upload_all_files(
    client: &reqwest::Client,
    relay_base: &str,
    room_id: &str,
    all_files: &[CollectedFile],
) -> Result<()> {
    let mut files: Vec<(String, String, usize)> = Vec::new();
    for f in all_files {
        let encoded = B64.encode(&f.content);
        let encoded_len = encoded.len();
        files.push((f.rel_path.clone(), encoded, encoded_len));
    }

    let url = format!("{relay_base}/api/rooms/{room_id}/upload-web");

    info!(
        "Full upload: {} files ({} bytes base64) to {url}",
        files.len(),
        files.iter().map(|(_, _, len)| *len).sum::<usize>()
    );

    let mut current_batch: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut current_batch_b64_bytes = 0usize;
    let mut batch_index = 0usize;
    for (path, encoded, encoded_len) in files {
        let should_flush =
            !current_batch.is_empty() && current_batch_b64_bytes + encoded_len > MAX_UPLOAD_BATCH_BASE64_BYTES;
        if should_flush {
            upload_web_legacy_batch(
                client,
                &url,
                room_id,
                batch_index,
                &current_batch,
                current_batch_b64_bytes,
            )
            .await?;
            batch_index += 1;
            current_batch = std::collections::HashMap::new();
            current_batch_b64_bytes = 0;
        }
        current_batch.insert(path, encoded);
        current_batch_b64_bytes += encoded_len;
    }

    if !current_batch.is_empty() {
        upload_web_legacy_batch(
            client,
            &url,
            room_id,
            batch_index,
            &current_batch,
            current_batch_b64_bytes,
        )
        .await?;
    }

    Ok(())
}

async fn upload_web_files_batch(
    client: &reqwest::Client,
    url: &str,
    _room_id: &str,
    batch_index: usize,
    files_payload: &std::collections::HashMap<String, serde_json::Value>,
    _total_b64_bytes: usize,
) -> Result<()> {
    let resp = client
        .post(url)
        .json(&serde_json::json!({ "files": files_payload }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("upload-web-files batch {batch_index}: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "upload-web-files batch {batch_index} failed: HTTP {status} — {body}"
        ));
    }
    Ok(())
}

async fn upload_web_legacy_batch(
    client: &reqwest::Client,
    url: &str,
    _room_id: &str,
    batch_index: usize,
    files_payload: &std::collections::HashMap<String, String>,
    _total_b64_bytes: usize,
) -> Result<()> {
    let resp = client
        .post(url)
        .json(&serde_json::json!({ "files": files_payload }))
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("upload mobile-web batch {batch_index}: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "upload mobile-web batch {batch_index} failed: HTTP {status} — {body}"
        ));
    }
    Ok(())
}

/// Recursively collect files with their SHA-256 hash.
fn collect_files_with_hash(base: &std::path::Path, dir: &std::path::Path, out: &mut Vec<CollectedFile>) -> Result<()> {
    use sha2::{Digest, Sha256};

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_hash(base, &path, out)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let content = std::fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());
            out.push(CollectedFile {
                rel_path: rel,
                content,
                hash,
            });
        }
    }
    Ok(())
}
