//! Weixin QR login flow + DTOs + session state (`qr_sessions`).
//!
//! Split out from the original `weixin.rs` god-file in R39a so each sibling
//! stays under 800 canonical lines.  The actual iLink bot driver lives in
//! the sibling `weixin_bot.rs` (struct + core lifecycle + HTTP/auth helpers);
//! CDN/messaging helpers live in `weixin_bot_media.rs`; inbound parsing,
//! pairing and message loop live in `weixin_bot_inbound.rs`.
//!
//! Constants and helpers exported `pub(super)` here are needed across
//! siblings: `now_ms` / `ensure_trailing_slash` / `random_wechat_uin_header`
//! are used by the iLink authenticated client (`weixin_bot.rs`); the
//! `*_sync_buf` helpers are used by `wait_for_pairing` / `run_message_loop`
//! in `weixin_bot_inbound.rs`.
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::warn;

pub(super) const DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
pub(super) const DEFAULT_ILINK_BOT_TYPE: &str = "3";
pub(super) const QR_POLL_TIMEOUT_SECS: u64 = 36;
pub(super) const MAX_QR_REFRESH: u32 = 3;

/// Cross-sibling timestamp helper: used by QR freshness checks, by
/// `is_session_paused` / `pause_session` (`weixin_bot.rs`), and by the QR
/// session store below.
pub(super) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Cross-sibling URL helper used both by QR call URLs and by `base_url`
/// in `weixin_bot.rs`.
pub(super) fn ensure_trailing_slash(url: &str) -> String {
    if url.ends_with('/') {
        url.to_string()
    } else {
        format!("{url}/")
    }
}

/// Random base64-encoded 32-bit value used in `build_auth_headers`
/// (`weixin_bot.rs`) as the `x-wechat-uin` request header.
pub(super) fn random_wechat_uin_header() -> String {
    let n: u32 = rand::thread_rng().gen();
    B64.encode(n.to_string().as_bytes())
}

fn normalize_weixin_account_id(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Path of the persisted `get_updates_buf` for this bot account — used by
/// QR-confirmed bots to resume long-poll after restart.
pub(super) fn sync_buf_path(bot_account_id: &str) -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    base.join(".northhing")
        .join("weixin")
        .join(format!("{bot_account_id}_get_updates_buf.txt"))
}

/// Load the long-poll sync buffer (empty string if missing).
pub(super) fn load_sync_buf(bot_account_id: &str) -> String {
    let p = sync_buf_path(bot_account_id);
    std::fs::read_to_string(&p).unwrap_or_default().trim().to_string()
}

/// Persist the long-poll sync buffer (best-effort; logs warnings).
pub(super) fn save_sync_buf(bot_account_id: &str, buf: &str) {
    let p = sync_buf_path(bot_account_id);
    if let Some(parent) = p.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&p, buf) {
        warn!("weixin: failed to save sync buf {}: {e}", p.display());
    }
}

// ── QR login session store (in-memory, same role as OpenClaw installer) ─────

#[derive(Debug, Clone)]
struct QrLoginSession {
    qrcode: String,
    qr_image_url: String,
    started_at_ms: i64,
    refresh_count: u32,
}

enum QrSessionLookup {
    Missing,
    TimedOut,
    Found(QrLoginSession),
}

fn qr_sessions() -> &'static Mutex<HashMap<String, QrLoginSession>> {
    static CELL: OnceLock<Mutex<HashMap<String, QrLoginSession>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(HashMap::new()))
}

// ── Public QR API (used from Tauri) ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct WeixinQrStartResponse {
    pub session_key: String,
    pub qr_image_url: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WeixinQrPollStatus {
    Wait,
    Scanned,
    Confirmed,
    Expired,
    Error,
}

#[derive(Debug, Serialize)]
pub struct WeixinQrPollResponse {
    pub status: WeixinQrPollStatus,
    pub message: String,
    /// Present when a new QR was issued after expiry (client should refresh image).
    pub qr_image_url: Option<String>,
    pub ilink_token: Option<String>,
    pub bot_account_id: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QrCodeApiResponse {
    qrcode: Option<String>,
    qrcode_img_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QrStatusApiResponse {
    status: Option<String>,
    bot_token: Option<String>,
    ilink_bot_id: Option<String>,
    baseurl: Option<String>,
}

/// Start Weixin QR login: fetch QR from iLink and register a session.
pub async fn weixin_qr_start(base_url_override: Option<String>) -> Result<WeixinQrStartResponse> {
    let base = ensure_trailing_slash(
        base_url_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_BASE_URL),
    );
    let url = format!(
        "{}ilink/bot/get_bot_qrcode?bot_type={}",
        base,
        urlencoding::encode(DEFAULT_ILINK_BOT_TYPE)
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(super::weixin_bot::API_TIMEOUT_SECS))
        .build()?;

    let resp = client.get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!("get_bot_qrcode HTTP {status}: {body}"));
    }
    let parsed: QrCodeApiResponse = resp.json().await?;
    let qrcode = parsed
        .qrcode
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("get_bot_qrcode: missing qrcode"))?;
    let qr_image_url = parsed
        .qrcode_img_content
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("get_bot_qrcode: missing qrcode_img_content"))?;

    let session_key = uuid::Uuid::new_v4().to_string();
    let session = QrLoginSession {
        qrcode,
        qr_image_url: qr_image_url.clone(),
        started_at_ms: now_ms(),
        refresh_count: 0,
    };
    qr_sessions()
        .lock()
        .map_err(|e| anyhow!("qr session lock: {e}"))?
        .insert(session_key.clone(), session);

    Ok(WeixinQrStartResponse {
        session_key,
        qr_image_url,
        message: "Scan the QR code with WeChat.".to_string(),
    })
}

/// Poll QR login status (long-poll once). Call repeatedly from the UI until `confirmed` or `error`.
pub async fn weixin_qr_poll(session_key: &str, base_url_override: Option<String>) -> Result<WeixinQrPollResponse> {
    let base = ensure_trailing_slash(
        base_url_override
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_BASE_URL),
    );

    let lookup = {
        let mut map = qr_sessions().lock().map_err(|e| anyhow!("qr session lock: {e}"))?;
        match map.get(session_key) {
            None => QrSessionLookup::Missing,
            Some(s) => {
                if now_ms() - s.started_at_ms > 5 * 60_000 {
                    map.remove(session_key);
                    QrSessionLookup::TimedOut
                } else {
                    QrSessionLookup::Found(s.clone())
                }
            }
        }
    };

    match lookup {
        QrSessionLookup::Missing => Ok(WeixinQrPollResponse {
            status: WeixinQrPollStatus::Error,
            message: "No active QR session. Start login again.".to_string(),
            qr_image_url: None,
            ilink_token: None,
            bot_account_id: None,
            base_url: None,
        }),
        QrSessionLookup::TimedOut => Ok(WeixinQrPollResponse {
            status: WeixinQrPollStatus::Error,
            message: "QR session expired. Start again.".to_string(),
            qr_image_url: None,
            ilink_token: None,
            bot_account_id: None,
            base_url: None,
        }),
        QrSessionLookup::Found(session) => {
            let qrcode_enc = urlencoding::encode(&session.qrcode);
            let url = format!("{}ilink/bot/get_qrcode_status?qrcode={}", base, qrcode_enc);

            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(QR_POLL_TIMEOUT_SECS))
                .build()?;

            let resp = client.get(&url).header("iLink-App-ClientVersion", "1").send().await;

            let resp = match resp {
                Ok(r) => r,
                Err(e) => {
                    if e.is_timeout() {
                        return Ok(WeixinQrPollResponse {
                            status: WeixinQrPollStatus::Wait,
                            message: "waiting".to_string(),
                            qr_image_url: None,
                            ilink_token: None,
                            bot_account_id: None,
                            base_url: None,
                        });
                    }
                    qr_sessions()
                        .lock()
                        .map_err(|e| anyhow!("qr session lock: {e}"))?
                        .remove(session_key);
                    return Err(anyhow!("get_qrcode_status: {e}"));
                }
            };

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                qr_sessions()
                    .lock()
                    .map_err(|e| anyhow!("qr session lock: {e}"))?
                    .remove(session_key);
                return Ok(WeixinQrPollResponse {
                    status: WeixinQrPollStatus::Error,
                    message: format!("HTTP {status}: {body}"),
                    qr_image_url: None,
                    ilink_token: None,
                    bot_account_id: None,
                    base_url: None,
                });
            }

            let status_json: QrStatusApiResponse = resp.json().await?;
            let st = status_json.status.as_deref().unwrap_or("wait");

            match st {
                "wait" => Ok(WeixinQrPollResponse {
                    status: WeixinQrPollStatus::Wait,
                    message: "waiting".to_string(),
                    qr_image_url: None,
                    ilink_token: None,
                    bot_account_id: None,
                    base_url: None,
                }),
                "scaned" => Ok(WeixinQrPollResponse {
                    status: WeixinQrPollStatus::Scanned,
                    message: "Scanned; confirm on your phone.".to_string(),
                    qr_image_url: None,
                    ilink_token: None,
                    bot_account_id: None,
                    base_url: None,
                }),
                "confirmed" => {
                    let token = status_json
                        .bot_token
                        .clone()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| anyhow!("confirmed but bot_token missing"))?;
                    let raw_id = status_json
                        .ilink_bot_id
                        .clone()
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| anyhow!("confirmed but ilink_bot_id missing"))?;
                    let normalized = normalize_weixin_account_id(&raw_id);
                    let baseurl = status_json
                        .baseurl
                        .clone()
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| base.trim_end_matches('/').to_string());

                    qr_sessions()
                        .lock()
                        .map_err(|e| anyhow!("qr session lock: {e}"))?
                        .remove(session_key);

                    Ok(WeixinQrPollResponse {
                        status: WeixinQrPollStatus::Confirmed,
                        message: "WeChat linked.".to_string(),
                        qr_image_url: None,
                        ilink_token: Some(token),
                        bot_account_id: Some(normalized),
                        base_url: Some(baseurl),
                    })
                }
                "expired" => {
                    let over_limit = {
                        let mut map = qr_sessions().lock().map_err(|e| anyhow!("qr session lock: {e}"))?;
                        let Some(s) = map.get_mut(session_key) else {
                            return Ok(WeixinQrPollResponse {
                                status: WeixinQrPollStatus::Error,
                                message: "Session lost. Start again.".to_string(),
                                qr_image_url: None,
                                ilink_token: None,
                                bot_account_id: None,
                                base_url: None,
                            });
                        };
                        s.refresh_count += 1;
                        if s.refresh_count > MAX_QR_REFRESH {
                            map.remove(session_key);
                            true
                        } else {
                            false
                        }
                    };

                    if over_limit {
                        return Ok(WeixinQrPollResponse {
                            status: WeixinQrPollStatus::Error,
                            message: "QR expired too many times; start again.".to_string(),
                            qr_image_url: None,
                            ilink_token: None,
                            bot_account_id: None,
                            base_url: None,
                        });
                    }

                    let refresh_url = format!(
                        "{}ilink/bot/get_bot_qrcode?bot_type={}",
                        base,
                        urlencoding::encode(DEFAULT_ILINK_BOT_TYPE)
                    );
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(super::weixin_bot::API_TIMEOUT_SECS))
                        .build()?;
                    let refresh = client.get(&refresh_url).send().await?;
                    if !refresh.status().is_success() {
                        qr_sessions()
                            .lock()
                            .map_err(|e| anyhow!("qr session lock: {e}"))?
                            .remove(session_key);
                        return Ok(WeixinQrPollResponse {
                            status: WeixinQrPollStatus::Error,
                            message: "Failed to refresh QR.".to_string(),
                            qr_image_url: None,
                            ilink_token: None,
                            bot_account_id: None,
                            base_url: None,
                        });
                    }
                    let parsed: QrCodeApiResponse = refresh.json().await?;
                    let qrcode = parsed
                        .qrcode
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| anyhow!("refresh: missing qrcode"))?;
                    let qr_image_url = parsed
                        .qrcode_img_content
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| anyhow!("refresh: missing qrcode_img_content"))?;

                    {
                        let mut m = qr_sessions().lock().map_err(|e| anyhow!("qr session lock: {e}"))?;
                        if let Some(s) = m.get_mut(session_key) {
                            s.qrcode = qrcode;
                            s.qr_image_url = qr_image_url.clone();
                            s.started_at_ms = now_ms();
                        }
                    }

                    Ok(WeixinQrPollResponse {
                        status: WeixinQrPollStatus::Expired,
                        message: "QR refreshed.".to_string(),
                        qr_image_url: Some(qr_image_url),
                        ilink_token: None,
                        bot_account_id: None,
                        base_url: None,
                    })
                }
                _ => Ok(WeixinQrPollResponse {
                    status: WeixinQrPollStatus::Wait,
                    message: st.to_string(),
                    qr_image_url: None,
                    ilink_token: None,
                    bot_account_id: None,
                    base_url: None,
                }),
            }
        }
    }
}
