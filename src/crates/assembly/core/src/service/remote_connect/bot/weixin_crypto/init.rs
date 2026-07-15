//! AES-128-ECB encrypt/decrypt + CDN AES key parsing for the Weixin iLink pipeline.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

/// Encrypt `plaintext` with AES-128-ECB + PKCS#7 padding.
pub(crate) fn encrypt_aes_128_ecb_pkcs7(plaintext: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new_from_slice(key).expect("AES-128 key len");
    let pad_len = 16 - (plaintext.len() % 16);
    let pad_len = if pad_len == 0 { 16 } else { pad_len };
    let mut buf = plaintext.to_vec();
    buf.extend(std::iter::repeat_n(pad_len as u8, pad_len));
    let mut out = Vec::with_capacity(buf.len());
    for chunk in buf.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    out
}

/// Decrypt `ciphertext` with AES-128-ECB + PKCS#7 unpadding.
pub(crate) fn decrypt_aes_128_ecb_pkcs7(ciphertext: &[u8], key: &[u8; 16]) -> Result<Vec<u8>> {
    if ciphertext.is_empty() || !ciphertext.len().is_multiple_of(16) {
        return Err(anyhow!("invalid ciphertext length {}", ciphertext.len()));
    }
    let cipher = Aes128::new_from_slice(key).expect("AES-128 key len");
    let mut out = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks_exact(16) {
        let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        out.extend_from_slice(&block);
    }
    let Some(&pad_byte) = out.last() else {
        return Err(anyhow!("empty after decrypt"));
    };
    let pad = pad_byte as usize;
    if pad == 0 || pad > 16 || pad > out.len() {
        return Err(anyhow!("invalid PKCS#7 padding (pad={pad})"));
    }
    if !out[out.len() - pad..].iter().all(|&b| b == pad_byte) {
        return Err(anyhow!("invalid PKCS#7 padding bytes"));
    }
    out.truncate(out.len() - pad);
    Ok(out)
}

/// `CDNMedia.aes_key`: base64(raw 16 bytes) or base64(32-char hex) — OpenClaw `parseAesKey`.
pub(crate) fn parse_weixin_cdn_aes_key(aes_key_base64: &str) -> Result<[u8; 16]> {
    let decoded = B64
        .decode(aes_key_base64.trim())
        .map_err(|e| anyhow!("aes_key base64: {e}"))?;
    if decoded.len() == 16 {
        let mut k = [0u8; 16];
        k.copy_from_slice(&decoded);
        return Ok(k);
    }
    if decoded.len() == 32 {
        let s = std::str::from_utf8(&decoded).map_err(|_| anyhow!("aes_key: expected utf8 hex"))?;
        if s.len() == 32 && s.chars().all(|c| c.is_ascii_hexdigit()) {
            let bytes = hex::decode(s).map_err(|e| anyhow!("aes_key inner hex: {e}"))?;
            if bytes.len() == 16 {
                let mut k = [0u8; 16];
                k.copy_from_slice(&bytes);
                return Ok(k);
            }
        }
    }
    Err(anyhow!(
        "aes_key: unsupported encoding (decoded {} bytes)",
        decoded.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_ecb_roundtrip() {
        let key = [9u8; 16];
        let plain = b"hello weixin cdn";
        let ct = encrypt_aes_128_ecb_pkcs7(plain, &key);
        let back = decrypt_aes_128_ecb_pkcs7(&ct, &key).unwrap();
        assert_eq!(back.as_slice(), plain.as_slice());
    }

    #[test]
    fn parse_aes_key_raw16_base64() {
        let raw = [1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let b64 = B64.encode(raw);
        let k = parse_weixin_cdn_aes_key(&b64).unwrap();
        assert_eq!(k, raw);
    }

    #[test]
    fn parse_aes_key_hex_wrapped_base64() {
        let raw = [0xabu8; 16];
        let hex_str = hex::encode(raw);
        let b64 = B64.encode(hex_str.as_bytes());
        let k = parse_weixin_cdn_aes_key(&b64).unwrap();
        assert_eq!(k, raw);
    }
}
