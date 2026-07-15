//! Pure helpers for the Weixin iLink crypto pipeline.
//!
//! No crypto state, no network — just deterministic URL/MIME utilities.

/// MD5 digest as a lowercase hex string.
pub(crate) fn md5_hex_lower(data: &[u8]) -> String {
    format!("{:x}", md5::compute(data))
}

/// Build the outbound CDN upload URL from components.
pub(crate) fn build_cdn_upload_url(cdn_base: &str, upload_param: &str, filekey: &str) -> String {
    let base = cdn_base.trim_end_matches('/');
    format!(
        "{}/upload?encrypted_query_param={}&filekey={}",
        base,
        urlencoding::encode(upload_param),
        urlencoding::encode(filekey)
    )
}

/// CDN download URL (same as `@tencent-weixin/openclaw-weixin` `buildCdnDownloadUrl`).
pub(crate) fn build_cdn_download_url(cdn_base: &str, encrypted_query_param: &str) -> String {
    let base = cdn_base.trim_end_matches('/');
    format!(
        "{}/download?encrypted_query_param={}",
        base,
        urlencoding::encode(encrypted_query_param)
    )
}

/// Best-effort MIME type from magic bytes.
pub(crate) fn sniff_image_mime(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
        return "image/jpeg";
    }
    if bytes.len() >= 8 && bytes[..8] == [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a] {
        return "image/png";
    }
    if bytes.len() >= 6 && (&bytes[..6] == b"GIF87a".as_slice() || &bytes[..6] == b"GIF89a".as_slice()) {
        return "image/gif";
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp";
    }
    "image/jpeg"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_upload_url_formats_query() {
        let url = build_cdn_upload_url("https://cdn.example.com/", "param=1", "filekey_abc");
        assert!(url.starts_with("https://cdn.example.com/upload?"));
        assert!(url.contains("encrypted_query_param=param%3D1"));
        assert!(url.contains("filekey=filekey_abc"));
    }

    #[test]
    fn build_download_url_formats_query() {
        let url = build_cdn_download_url("https://cdn.example.com", "token");
        assert_eq!(url, "https://cdn.example.com/download?encrypted_query_param=token");
    }

    #[test]
    fn sniff_jpeg_magic() {
        assert_eq!(sniff_image_mime(&[0xff, 0xd8, 0xff]), "image/jpeg");
    }

    #[test]
    fn sniff_png_magic() {
        assert_eq!(
            sniff_image_mime(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
            "image/png"
        );
    }

    #[test]
    fn sniff_unknown_falls_back_to_jpeg() {
        assert_eq!(sniff_image_mime(b"UNKNOWN"), "image/jpeg");
    }
}
