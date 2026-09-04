//! WebPush application-server crypto (RFC 8291 `aes128gcm` + RFC 8292 VAPID).
//!
//! Built entirely on `ring` primitives this crate already depends on:
//! ECDH P-256 (`agreement`), ES256 (ECDSA P-256 + SHA-256, `signature`),
//! AES-128-GCM (`aead`) and HKDF-SHA-256 (`hkdf`). No new third-party crates.
//!
//! Interop is pinned by the RFC 8291 Appendix A / §5 fixed-key vector — the
//! unit test reproduces the published ciphertext byte-for-byte, which locks
//! every HKDF info string, the record layout and the 86-byte frame header.

use ring::{
    aead, agreement, hkdf,
    rand::{SecureRandom, SystemRandom},
    signature::{self, EcdsaKeyPair, KeyPair as _},
};

const AES128GCM_CEK_INFO: &[u8] = b"Content-Encoding: aes128gcm\x00";
const AES128GCM_NONCE_INFO: &[u8] = b"Content-Encoding: nonce\x00";
const WEBPUSH_IKM_INFO_PREFIX: &[u8] = b"WebPush: info\x00";
const AES128GCM_RECORD_SIZE: u32 = 4096;
const PADDING_DELIMITER_LAST_RECORD: u8 = 0x02;
const UA_PUBLIC_KEY_LEN: usize = 65;
const AUTH_SECRET_LEN: usize = 16;
const TAG_LEN: usize = 16;

#[derive(Debug)]
pub struct WebPushCryptoError(String);

impl std::fmt::Display for WebPushCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn error(message: impl Into<String>) -> WebPushCryptoError {
    WebPushCryptoError(message.into())
}

// ---- base64url (RFC 4648 §5, unpadded — the encoding every WebPush field uses) ----

const B64URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn base64url_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64URL_ALPHABET[(triple >> 18) as usize & 63] as char);
        out.push(B64URL_ALPHABET[(triple >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(B64URL_ALPHABET[(triple >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL_ALPHABET[triple as usize & 63] as char);
        }
    }
    out
}

fn base64url_value(byte: u8) -> Result<u32, WebPushCryptoError> {
    match byte {
        b'A'..=b'Z' => Ok((byte - b'A') as u32),
        b'a'..=b'z' => Ok((byte - b'a') as u32 + 26),
        b'0'..=b'9' => Ok((byte - b'0') as u32 + 52),
        b'-' => Ok(62),
        b'_' => Ok(63),
        _ => Err(error("invalid base64url character")),
    }
}

pub fn base64url_decode(data: &str) -> Result<Vec<u8>, WebPushCryptoError> {
    let trimmed: Vec<u8> = data
        .bytes()
        .filter(|byte| !matches!(byte, b'=' | b'\n' | b'\r'))
        .collect();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(trimmed.len() * 3 / 4);
    for chunk in trimmed.chunks(4) {
        let mut values = [0u32; 4];
        for (index, byte) in chunk.iter().enumerate() {
            values[index] = base64url_value(*byte)?;
        }
        let triple = (values[0] << 18) | (values[1] << 12) | (values[2] << 6) | values[3];
        out.push((triple >> 16) as u8);
        if chunk.len() > 2 {
            out.push((triple >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(triple as u8);
        }
    }
    Ok(out)
}

// ---- VAPID (RFC 8292): ES256 compact JWS over {aud, exp, sub} ----

/// ES256 application-server key pair. The PKCS8 bytes are the serialized
/// private key — persist them 0600 like any secret.
pub struct VapidKeyPair {
    key_pair: EcdsaKeyPair,
}

impl VapidKeyPair {
    /// Generate a fresh P-256 signing key; returns the PKCS8 bytes to persist.
    pub fn generate() -> Result<(Self, Vec<u8>), WebPushCryptoError> {
        let rng = SystemRandom::new();
        let document =
            EcdsaKeyPair::generate_pkcs8(&signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
                .map_err(|_| error("vapid key generation failed"))?;
        let pkcs8 = document.as_ref().to_vec();
        let key_pair = Self::from_pkcs8(&pkcs8)?;
        Ok((key_pair, pkcs8))
    }

    /// Rebuild the key pair from persisted PKCS8 bytes.
    pub fn from_pkcs8(pkcs8: &[u8]) -> Result<Self, WebPushCryptoError> {
        let key_pair = EcdsaKeyPair::from_pkcs8(
            &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
            pkcs8,
            &SystemRandom::new(),
        )
        .map_err(|_| error("vapid private key unavailable"))?;
        Ok(Self { key_pair })
    }

    /// 65-byte uncompressed P-256 public key (starts with 0x04).
    pub fn public_key_raw(&self) -> Vec<u8> {
        self.key_pair.public_key().as_ref().to_vec()
    }

    /// The `k=` parameter value pushed services echo back (base64url).
    pub fn public_key_base64url(&self) -> String {
        base64url_encode(&self.public_key_raw())
    }

    /// Compact JWS (`header.claims.signature`) over the VAPID claims.
    pub fn sign_token(
        &self,
        audience_origin: &str,
        subject: &str,
        expires_at_secs: u64,
    ) -> Result<String, WebPushCryptoError> {
        use serde_json::json;
        let header = json!({"typ": "JWT", "alg": "ES256"}).to_string();
        let claims = json!({
            "aud": audience_origin,
            "exp": expires_at_secs,
            "sub": subject,
        })
        .to_string();
        let signing_input = format!(
            "{}.{}",
            base64url_encode(header.as_bytes()),
            base64url_encode(claims.as_bytes())
        );
        let signature = self
            .key_pair
            .sign(&SystemRandom::new(), signing_input.as_bytes())
            .map_err(|_| error("vapid signing failed"))?;
        Ok(format!(
            "{}.{}",
            signing_input,
            base64url_encode(signature.as_ref())
        ))
    }
}

// ---- RFC 8291 aes128gcm message encryption ----

fn hkdf_extract_expand(
    salt: &[u8],
    ikm: &[u8],
    info: &[&[u8]],
    length: usize,
) -> Result<Vec<u8>, WebPushCryptoError> {
    const HKDF_SHA256_LEN: usize = 32;
    if length > HKDF_SHA256_LEN {
        return Err(error("hkdf output length exceeds hash size"));
    }
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, salt);
    let prk = salt.extract(ikm);
    // RFC 5869: L ≤ HashLen truncates the first output block, so expanding to
    // the algorithm length and slicing is exact.
    let mut full = vec![0u8; HKDF_SHA256_LEN];
    prk.expand(info, hkdf::HKDF_SHA256)
        .map_err(|_| error("hkdf expand rejected info"))?
        .fill(&mut full)
        .map_err(|_| error("hkdf fill failed"))?;
    Ok(full[..length].to_vec())
}

/// RFC 8291 §4: IKM = HKDF(salt=auth_secret, ikm=ecdh_secret,
/// info="WebPush: info" || 0x00 || ua_public || as_public, L=32).
fn derive_ikm(
    ecdh_secret: &[u8],
    auth_secret: &[u8],
    ua_public: &[u8],
    as_public: &[u8],
) -> Result<Vec<u8>, WebPushCryptoError> {
    hkdf_extract_expand(
        auth_secret,
        ecdh_secret,
        &[WEBPUSH_IKM_INFO_PREFIX, ua_public, as_public],
        32,
    )
}

/// CEK (16B) and nonce (12B) from the random per-message salt.
fn derive_cek_and_nonce(
    ikm: &[u8],
    salt: &[u8],
) -> Result<(Vec<u8>, [u8; 12]), WebPushCryptoError> {
    let cek = hkdf_extract_expand(salt, ikm, &[AES128GCM_CEK_INFO], 16)?;
    let nonce_bytes = hkdf_extract_expand(salt, ikm, &[AES128GCM_NONCE_INFO], 12)?;
    let nonce: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| error("nonce length mismatch"))?;
    Ok((cek, nonce))
}

fn validate_ua_keys(ua_public: &[u8], auth_secret: &[u8]) -> Result<(), WebPushCryptoError> {
    if ua_public.len() != UA_PUBLIC_KEY_LEN || ua_public[0] != 0x04 {
        return Err(error("ua public key must be an uncompressed P-256 point"));
    }
    if auth_secret.len() != AUTH_SECRET_LEN {
        return Err(error("ua auth secret must be 16 bytes"));
    }
    Ok(())
}

/// Deterministic encryption core — the production path generates the ephemeral
/// key and salt; tests feed the RFC 8291 Appendix A fixed values and compare
/// the frame byte-for-byte with the published vector.
fn encrypt_with(
    ecdh_secret: &[u8],
    as_public: &[u8],
    ua_public: &[u8],
    auth_secret: &[u8],
    salt: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, WebPushCryptoError> {
    let ikm = derive_ikm(ecdh_secret, auth_secret, ua_public, as_public)?;
    let (cek, nonce) = derive_cek_and_nonce(&ikm, salt)?;

    let mut record = plaintext.to_vec();
    record.push(PADDING_DELIMITER_LAST_RECORD);
    let record_len = record.len() + TAG_LEN;
    if record_len > AES128GCM_RECORD_SIZE as usize {
        return Err(error("payload exceeds one aes128gcm record"));
    }
    let sealing_key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_128_GCM, &cek).map_err(|_| error("cek rejected"))?,
    );
    sealing_key
        .seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::empty(),
            &mut record,
        )
        .map_err(|_| error("payload sealing failed"))?;

    let mut frame = Vec::with_capacity(86 + record_len);
    frame.extend_from_slice(salt);
    frame.extend_from_slice(&AES128GCM_RECORD_SIZE.to_be_bytes());
    frame.push(as_public.len() as u8);
    frame.extend_from_slice(as_public);
    frame.extend_from_slice(&record);
    Ok(frame)
}

/// Encrypt one WebPush message for a browser subscription
/// (`keys.p256dh` / `keys.auth` in their base64url wire form are decoded by
/// the caller). Returns the complete `aes128gcm` request body.
pub fn encrypt_message(
    plaintext: &[u8],
    ua_public: &[u8],
    auth_secret: &[u8],
) -> Result<Vec<u8>, WebPushCryptoError> {
    let rng = SystemRandom::new();
    validate_ua_keys(ua_public, auth_secret)?;
    let ephemeral = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
        .map_err(|_| error("ephemeral key generation failed"))?;
    let as_public = ephemeral
        .compute_public_key()
        .map_err(|_| error("ephemeral public key derivation failed"))?;
    let ecdh_secret = agreement::agree_ephemeral(
        ephemeral,
        &agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, ua_public),
        |shared| shared.to_vec(),
    )
    .map_err(|_| error("ecdh with subscription key failed"))?;
    let mut salt = [0u8; 16];
    rng.fill(&mut salt)
        .map_err(|_| error("salt generation failed"))?;
    encrypt_with(
        &ecdh_secret,
        as_public.as_ref(),
        ua_public,
        auth_secret,
        &salt,
        plaintext,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(value: &str) -> Vec<u8> {
        base64url_decode(value).unwrap()
    }

    #[test]
    fn base64url_roundtrip_and_known_values() {
        // RFC 8291 §5 salt, 16 bytes
        assert_eq!(decode("DGv6ra1nlYgDCS1FRnbzlw").len(), 16);
        // RFC 8291 §5 UA public key, 65-byte uncompressed P-256 point
        let ua_public = decode(
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
        );
        assert_eq!(ua_public.len(), 65);
        assert_eq!(
            base64url_encode(&ua_public),
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"
        );
        assert!(base64url_decode("not*valid").is_err());
    }

    /// RFC 8291 Appendix A fixed-key vector, reproduced byte-for-byte.
    #[test]
    fn rfc8291_appendix_a_vector_matches_byte_for_byte() {
        let ua_public = decode(
            "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4",
        );
        let as_public = decode(
            "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8",
        );
        let auth_secret = decode("BTBZMqHH6r4Tts7J_aSIgg");
        let ecdh_secret = decode("kyrL1jIIOHEzg3sM2ZWRHDRB62YACZhhSlknJ672kSs");
        let salt = decode("DGv6ra1nlYgDCS1FRnbzlw");
        let plaintext = b"When I grow up, I want to be a watermelon";

        // Published intermediate values first, so failures point at the stage.
        let ikm = derive_ikm(&ecdh_secret, &auth_secret, &ua_public, &as_public).unwrap();
        assert_eq!(
            base64url_encode(&ikm),
            "S4lYMb_L0FxCeq0WhDx813KgSYqU26kOyzWUdsXYyrg"
        );
        let (cek, nonce) = derive_cek_and_nonce(&ikm, &salt).unwrap();
        assert_eq!(base64url_encode(&cek), "oIhVW04MRdy2XN9CiKLxTg");
        assert_eq!(base64url_encode(&nonce), "4h_95klXJ5E_qnoN");

        let frame = encrypt_with(
            &ecdh_secret,
            &as_public,
            &ua_public,
            &auth_secret,
            &salt,
            plaintext,
        )
        .unwrap();
        // RFC 8291 §5 的完整请求体（86 字节头 + 密文，整体 base64url 一次编码）
        let expected = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN";
        assert_eq!(base64url_encode(&frame), expected);
    }

    #[test]
    fn encrypt_message_roundtrips_through_browser_side_decryption() {
        let rng = SystemRandom::new();
        // 模拟浏览器：生成订阅密钥对（UA 侧私钥留在测试内用于解密）
        let ua_private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng).unwrap();
        let ua_public = ua_private.compute_public_key().unwrap().as_ref().to_vec();
        let mut auth_secret = [0u8; AUTH_SECRET_LEN];
        rng.fill(&mut auth_secret).unwrap();

        let plaintext = b"push payload from the dog & duck house";
        let frame = encrypt_message(plaintext, &ua_public, &auth_secret).unwrap();

        // 帧结构：salt(16) | rs(4 BE = 4096) | idhlen(1) | keyid(65)
        assert_eq!(&frame[16..20], &4096u32.to_be_bytes());
        assert_eq!(frame[20], UA_PUBLIC_KEY_LEN as u8);
        assert_eq!(frame[21], 0x04);
        let as_public = &frame[21..86];

        // 浏览器侧：ECDH(ua_priv, as_public) 必须还原同一共享秘密
        let ecdh_secret = agreement::agree_ephemeral(
            ua_private,
            &agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, as_public),
            |shared| shared.to_vec(),
        )
        .unwrap();
        let ikm = derive_ikm(&ecdh_secret, &auth_secret, &ua_public, as_public).unwrap();
        let (cek, nonce) = derive_cek_and_nonce(&ikm, &frame[0..16]).unwrap();
        let mut record = frame[86..].to_vec();
        let sealing_key =
            aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_128_GCM, &cek).unwrap());
        sealing_key
            .open_in_place(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::empty(),
                &mut record,
            )
            .unwrap();
        assert_eq!(&record[..plaintext.len()], plaintext);
        assert_eq!(record[plaintext.len()], PADDING_DELIMITER_LAST_RECORD);
    }

    #[test]
    fn encrypt_message_rejects_malformed_subscription_keys() {
        let rng = SystemRandom::new();
        let short_point = vec![0x04u8; 64];
        assert!(encrypt_message(b"x", &short_point, &[0u8; 16]).is_err());
        let mut point = vec![0x04u8; 65];
        assert!(encrypt_message(b"x", &point, &[0u8; 15]).is_err());
        point[0] = 0x03;
        assert!(encrypt_message(b"x", &point, &[0u8; 16]).is_err());
    }

    #[test]
    fn vapid_token_signs_and_verifies_with_structured_claims() {
        let (key_pair, pkcs8) = VapidKeyPair::generate().unwrap();
        let rebuilt = VapidKeyPair::from_pkcs8(&pkcs8).unwrap();
        assert_eq!(key_pair.public_key_raw(), rebuilt.public_key_raw());

        let token = rebuilt
            .sign_token(
                "https://fcm.googleapis.com",
                "mailto:steward@chat.ajw.cn",
                1_800_000_000,
            )
            .unwrap();
        let segments: Vec<&str> = token.split('.').collect();
        assert_eq!(segments.len(), 3);
        let header = String::from_utf8(base64url_decode(segments[0]).unwrap()).unwrap();
        assert!(header.contains("\"alg\":\"ES256\""));
        assert!(header.contains("\"typ\":\"JWT\""));
        let claims = String::from_utf8(base64url_decode(segments[1]).unwrap()).unwrap();
        assert!(claims.contains("\"aud\":\"https://fcm.googleapis.com\""));
        assert!(claims.contains("\"sub\":\"mailto:steward@chat.ajw.cn\""));
        assert!(claims.contains("\"exp\":1800000000"));

        // ES256 签名可用公钥对 signing input 校验（FIXED 64 字节 r||s）
        let signature = base64url_decode(segments[2]).unwrap();
        assert_eq!(signature.len(), 64);
        let signing_input = format!("{}.{}", segments[0], segments[1]);
        signature::UnparsedPublicKey::new(
            &signature::ECDSA_P256_SHA256_FIXED,
            rebuilt.public_key_raw(),
        )
        .verify(signing_input.as_bytes(), &signature)
        .unwrap();
    }
}
