use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message as WsMessage;

// =====================================================================
// Type aliases
// =====================================================================

type FeishuWsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
pub type FeishuWsWrite = futures::stream::SplitSink<FeishuWsStream, WsMessage>;
pub type SharedFeishuWsWrite = Arc<RwLock<FeishuWsWrite>>;

// =====================================================================
// Constants
// =====================================================================

/// Feishu IM file-upload hard limit (30 MB).
pub const MAX_FEISHU_FILE_BYTES: u64 = 30 * 1024 * 1024;

// =====================================================================
// Minimal protobuf codec for Feishu WebSocket binary protocol
// =====================================================================

pub mod pb {
    //! Protobuf codec matching Feishu SDK's pbbp2.proto.
    //! Field numbers from pbbp2.pb.go (proto2 with required fields):
    //!   1: SeqID (uint64)
    //!   2: LogID (uint64)
    //!   3: Service (int32)
    //!   4: Method (int32)       — 0 = control, 1 = data
    //!   5: Headers (repeated Header)
    //!   6: PayloadEncoding (string)
    //!   7: PayloadType (string)
    //!   8: Payload (bytes)
    //!   9: LogIDNew (string)

    #[derive(Debug, Default, Clone)]
    pub struct Frame {
        pub seq_id: u64,
        pub log_id: u64,
        pub service: i32,
        pub method: i32,
        pub headers: Vec<(String, String)>,
        pub payload_encoding: String,
        pub payload_type: String,
        pub payload: Vec<u8>,
        pub log_id_new: String,
    }

    pub const FRAME_TYPE_CONTROL: i32 = 0;
    pub const FRAME_TYPE_DATA: i32 = 1;

    fn decode_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
        let mut result: u64 = 0;
        let mut shift = 0u32;
        loop {
            if *pos >= data.len() {
                return None;
            }
            let byte = data[*pos];
            *pos += 1;
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
    }

    fn encode_varint(mut val: u64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(10);
        loop {
            let mut byte = (val & 0x7F) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            buf.push(byte);
            if val == 0 {
                break;
            }
        }
        buf
    }

    fn read_len<'a>(data: &'a [u8], pos: &mut usize) -> Option<&'a [u8]> {
        let len = decode_varint(data, pos)? as usize;
        if *pos + len > data.len() {
            return None;
        }
        let slice = &data[*pos..*pos + len];
        *pos += len;
        Some(slice)
    }

    fn decode_header(data: &[u8]) -> Option<(String, String)> {
        let mut pos = 0;
        let (mut key, mut val) = (String::new(), String::new());
        while pos < data.len() {
            let tag = decode_varint(data, &mut pos)? as u32;
            match (tag >> 3, tag & 7) {
                (1, 2) => key = String::from_utf8_lossy(read_len(data, &mut pos)?).into(),
                (2, 2) => val = String::from_utf8_lossy(read_len(data, &mut pos)?).into(),
                (_, 0) => {
                    decode_varint(data, &mut pos)?;
                }
                (_, 2) => {
                    read_len(data, &mut pos)?;
                }
                _ => return None,
            }
        }
        Some((key, val))
    }

    pub fn decode_frame(data: &[u8]) -> Option<Frame> {
        let mut pos = 0;
        let mut f = Frame::default();
        while pos < data.len() {
            let tag = decode_varint(data, &mut pos)? as u32;
            match (tag >> 3, tag & 7) {
                (1, 0) => f.seq_id = decode_varint(data, &mut pos)?,
                (2, 0) => f.log_id = decode_varint(data, &mut pos)?,
                (3, 0) => f.service = decode_varint(data, &mut pos)? as i32,
                (4, 0) => f.method = decode_varint(data, &mut pos)? as i32,
                (5, 2) => {
                    if let Some(h) = decode_header(read_len(data, &mut pos)?) {
                        f.headers.push(h);
                    }
                }
                (6, 2) => f.payload_encoding = String::from_utf8_lossy(read_len(data, &mut pos)?).into(),
                (7, 2) => f.payload_type = String::from_utf8_lossy(read_len(data, &mut pos)?).into(),
                (8, 2) => f.payload = read_len(data, &mut pos)?.to_vec(),
                (9, 2) => f.log_id_new = String::from_utf8_lossy(read_len(data, &mut pos)?).into(),
                (_, 0) => {
                    decode_varint(data, &mut pos)?;
                }
                (_, 2) => {
                    read_len(data, &mut pos)?;
                }
                (_, 5) => {
                    pos += 4;
                } // fixed32
                (_, 1) => {
                    pos += 8;
                } // fixed64
                _ => return None,
            }
        }
        Some(f)
    }

    fn write_varint(buf: &mut Vec<u8>, field: u32, val: u64) {
        buf.extend(encode_varint((field << 3) as u64));
        buf.extend(encode_varint(val));
    }

    fn write_bytes(buf: &mut Vec<u8>, field: u32, data: &[u8]) {
        buf.extend(encode_varint(((field << 3) | 2) as u64));
        buf.extend(encode_varint(data.len() as u64));
        buf.extend(data);
    }

    fn encode_header(key: &str, value: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        write_bytes(&mut buf, 1, key.as_bytes());
        write_bytes(&mut buf, 2, value.as_bytes());
        buf
    }

    pub fn encode_frame(frame: &Frame) -> Vec<u8> {
        let mut buf = Vec::new();
        write_varint(&mut buf, 1, frame.seq_id);
        write_varint(&mut buf, 2, frame.log_id);
        write_varint(&mut buf, 3, frame.service as u64);
        write_varint(&mut buf, 4, frame.method as u64);
        for (k, v) in &frame.headers {
            let hdr = encode_header(k, v);
            write_bytes(&mut buf, 5, &hdr);
        }
        if !frame.payload_encoding.is_empty() {
            write_bytes(&mut buf, 6, frame.payload_encoding.as_bytes());
        }
        if !frame.payload_type.is_empty() {
            write_bytes(&mut buf, 7, frame.payload_type.as_bytes());
        }
        if !frame.payload.is_empty() {
            write_bytes(&mut buf, 8, &frame.payload);
        }
        if !frame.log_id_new.is_empty() {
            write_bytes(&mut buf, 9, frame.log_id_new.as_bytes());
        }
        buf
    }

    impl Frame {
        pub fn get_header(&self, key: &str) -> Option<&str> {
            self.headers.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
        }

        pub fn new_ping(service_id: i32) -> Self {
            Frame {
                method: FRAME_TYPE_CONTROL,
                service: service_id,
                headers: vec![("type".into(), "ping".into())],
                ..Default::default()
            }
        }

        pub fn new_response(original: &Frame, status_code: u16) -> Self {
            let mut headers = original.headers.clone();
            headers.push(("biz_rt".into(), "0".into()));
            Frame {
                seq_id: original.seq_id,
                log_id: original.log_id,
                service: original.service,
                method: original.method,
                headers,
                payload: serde_json::to_vec(&serde_json::json!({"code": status_code})).unwrap_or_default(),
                log_id_new: original.log_id_new.clone(),
                ..Default::default()
            }
        }
    }
}

// =====================================================================
// Configuration and state types
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FeishuToken {
    pub(crate) access_token: String,
    pub(crate) expires_at: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingPairing {
    pub(crate) created_at: i64,
}

pub(crate) struct ParsedMessage {
    pub(crate) chat_id: String,
    pub(crate) message_id: String,
    pub(crate) text: String,
    pub(crate) image_keys: Vec<String>,
}
