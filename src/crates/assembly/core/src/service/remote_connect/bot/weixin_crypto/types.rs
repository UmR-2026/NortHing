//! Constants + size/padding helpers for the Weixin iLink crypto pipeline.

/// Weixin CDN host for encrypted upload (same as `@tencent-weixin/openclaw-weixin`).
pub(crate) const DEFAULT_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";
/// Same cap as Feishu bot file send.
pub(crate) const MAX_WEIXIN_FILE_BYTES: u64 = 30 * 1024 * 1024;
pub(crate) const CDN_UPLOAD_MAX_RETRIES: u32 = 3;

/// PKCS#7-padded ciphertext length for a given plaintext length (AES-128 block = 16).
pub(crate) fn aes_ecb_ciphertext_len(plaintext_len: usize) -> usize {
    let pad = 16 - (plaintext_len % 16);
    let pad = if pad == 0 { 16 } else { pad };
    plaintext_len + pad
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ciphertext_len_pads_to_block_boundary() {
        assert_eq!(aes_ecb_ciphertext_len(0), 16);
        assert_eq!(aes_ecb_ciphertext_len(15), 16);
        assert_eq!(aes_ecb_ciphertext_len(16), 32);
        assert_eq!(aes_ecb_ciphertext_len(17), 32);
    }
}
