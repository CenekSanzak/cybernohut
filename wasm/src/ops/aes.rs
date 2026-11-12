//! AES in the modes that turn up in practice. The key length picks AES-128, -192 or
//! -256.

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit, StreamCipher};
use aes::{Aes128, Aes192, Aes256};
use aes_gcm::aead::consts::U12;
use aes_gcm::aead::Aead;
use aes_gcm::{AesGcm, Nonce};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use super::key::{parse_hex, parse_key, KeyFormat};
use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

const BLOCK_BYTES: usize = 16;
const GCM_NONCE_BYTES: usize = 12;
const GCM_TAG_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum AesMode {
    #[default]
    Cbc,
    Ctr,
    Gcm,
    Ecb,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct AesConfig {
    pub mode: AesMode,
    pub key: String,
    pub key_format: KeyFormat,
    /// Hexadecimal. 16 bytes for CBC and CTR, 12 for GCM, unused for ECB.
    pub iv: String,
}

fn params() -> Vec<ParamSpec> {
    let modes = ["cbc", "ctr", "gcm", "ecb"].map(String::from).to_vec();
    vec![
        ParamSpec::new("mode", "Mode", ParamKind::Choice { options: modes }),
        ParamSpec::new("key", "Key", ParamKind::Text),
        ParamSpec::new("keyFormat", "Key format", ParamKind::Choice { options: KeyFormat::options() }),
        ParamSpec::new("iv", "IV / nonce (hex)", ParamKind::Text),
    ]
}

struct Material {
    key: Vec<u8>,
    iv: Vec<u8>,
}

fn material(config: &AesConfig) -> Result<Material, OpError> {
    let key = parse_key(&config.key, config.key_format)?;
    if ![16, 24, 32].contains(&key.len()) {
        return Err(OpError::invalid_config(format!(
            "The key is {} bytes; AES needs 16, 24 or 32",
            key.len()
        )));
    }
    let expected = match config.mode {
        AesMode::Ecb => return Ok(Material { key, iv: Vec::new() }),
        AesMode::Gcm => GCM_NONCE_BYTES,
        AesMode::Cbc | AesMode::Ctr => BLOCK_BYTES,
    };
    let iv = parse_hex(&config.iv, "IV")?;
    if iv.len() != expected {
        return Err(OpError::invalid_config(format!(
            "The IV is {} bytes; this mode needs {expected}",
            iv.len()
        )));
    }
    Ok(Material { key, iv })
}

/// Runs `$body` with `$cipher` bound to the AES variant the key length selects.
/// [`material`] has already rejected every other length.
macro_rules! with_cipher {
    ($key:expr, $cipher:ident => $body:expr) => {
        match $key.len() {
            16 => {
                type $cipher = Aes128;
                $body
            }
            24 => {
                type $cipher = Aes192;
                $body
            }
            _ => {
                type $cipher = Aes256;
                $body
            }
        }
    };
}

fn ctr(key: &[u8], iv: &[u8], input: &[u8]) -> Vec<u8> {
    let mut output = input.to_vec();
    with_cipher!(key, Cipher => {
        ctr::Ctr128BE::<Cipher>::new_from_slices(key, iv)
            .expect("lengths are checked")
            .apply_keystream(&mut output)
    });
    output
}

fn encrypt(config: &AesConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
    let Material { key, iv } = material(config)?;
    let (key, iv) = (key.as_slice(), iv.as_slice());
    Ok(match config.mode {
        AesMode::Cbc => with_cipher!(key, Cipher => {
            cbc::Encryptor::<Cipher>::new_from_slices(key, iv)
                .expect("lengths are checked")
                .encrypt_padded_vec_mut::<Pkcs7>(input)
        }),
        AesMode::Ecb => with_cipher!(key, Cipher => {
            ecb::Encryptor::<Cipher>::new_from_slice(key)
                .expect("length is checked")
                .encrypt_padded_vec_mut::<Pkcs7>(input)
        }),
        AesMode::Ctr => ctr(key, iv, input),
        AesMode::Gcm => with_cipher!(key, Cipher => {
            AesGcm::<Cipher, U12>::new_from_slice(key)
                .expect("length is checked")
                .encrypt(Nonce::from_slice(iv), input)
                .map_err(|_| OpError::invalid_input("The input is too large for AES-GCM"))?
        }),
    })
}

fn decrypt(config: &AesConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
    let Material { key, iv } = material(config)?;
    let (key, iv) = (key.as_slice(), iv.as_slice());
    let padding = || {
        OpError::invalid_input(
            "Decryption did not end in valid padding: wrong key, IV or mode, or damaged data",
        )
    };
    if matches!(config.mode, AesMode::Cbc | AesMode::Ecb)
        && (input.is_empty() || input.len() % BLOCK_BYTES != 0)
    {
        return Err(OpError::invalid_input(format!(
            "The input is {} bytes; this mode needs a multiple of 16",
            input.len()
        )));
    }
    Ok(match config.mode {
        AesMode::Cbc => with_cipher!(key, Cipher => {
            cbc::Decryptor::<Cipher>::new_from_slices(key, iv)
                .expect("lengths are checked")
                .decrypt_padded_vec_mut::<Pkcs7>(input)
                .map_err(|_| padding())?
        }),
        AesMode::Ecb => with_cipher!(key, Cipher => {
            ecb::Decryptor::<Cipher>::new_from_slice(key)
                .expect("length is checked")
                .decrypt_padded_vec_mut::<Pkcs7>(input)
                .map_err(|_| padding())?
        }),
        AesMode::Ctr => ctr(key, iv, input),
        AesMode::Gcm => {
            if input.len() < GCM_TAG_BYTES {
                return Err(OpError::invalid_input(
                    "The input is shorter than the 16-byte authentication tag",
                ));
            }
            with_cipher!(key, Cipher => {
                AesGcm::<Cipher, U12>::new_from_slice(key)
                    .expect("length is checked")
                    .decrypt(Nonce::from_slice(iv), input)
                    .map_err(|_| OpError::invalid_input(
                        "Authentication failed: wrong key or nonce, or the data was modified",
                    ))?
            })
        }
    })
}

const LINK: &str = "https://en.wikipedia.org/wiki/Advanced_Encryption_Standard";

pub struct AesEncrypt;

impl Op for AesEncrypt {
    const ID: &'static str = "aes-encrypt";
    const NAME: &'static str = "AES Encrypt";
    const DESCRIPTION: &'static str =
        "Encrypts with AES. CBC and ECB pad with PKCS#7; GCM appends the 16-byte tag";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encryption];

    type Config = AesConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &AesConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        encrypt(config, input)
    }
}

pub struct AesDecrypt;

impl Op for AesDecrypt {
    const ID: &'static str = "aes-decrypt";
    const NAME: &'static str = "AES Decrypt";
    const DESCRIPTION: &'static str =
        "Decrypts AES. CBC and ECB expect PKCS#7 padding; GCM expects the tag at the end";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encryption];

    type Config = AesConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        params()
    }

    fn run(config: &AesConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        decrypt(config, input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn hex(bytes: &[u8]) -> String {
        hex_simd::encode_to_string(bytes, hex_simd::AsciiCase::Lower)
    }

    fn unhex(text: &str) -> Vec<u8> {
        hex_simd::decode_to_vec(text).unwrap()
    }

    fn config(mode: AesMode, key: &str, iv: &str) -> AesConfig {
        AesConfig { mode, key: key.into(), key_format: KeyFormat::Hex, iv: iv.into() }
    }

    const KEY_128: &str = "2b7e151628aed2a6abf7158809cf4f3c";
    const BLOCK: &str = "6bc1bee22e409f96e93d7e117393172a";

    #[test]
    fn nist_sp800_38a_first_blocks() {
        let plain = unhex(BLOCK);
        let cbc = config(AesMode::Cbc, KEY_128, "000102030405060708090a0b0c0d0e0f");
        assert_eq!(hex(&encrypt(&cbc, &plain).unwrap()[..16]), "7649abac8119b246cee98e9b12e9197d");
        let ecb = config(AesMode::Ecb, KEY_128, "");
        assert_eq!(hex(&encrypt(&ecb, &plain).unwrap()[..16]), "3ad77bb40d7a3660a89ecaf32466ef97");
        let ctr = config(AesMode::Ctr, KEY_128, "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff");
        assert_eq!(hex(&encrypt(&ctr, &plain).unwrap()), "874d6191b620e3261bef6864990db6ce");
        let key_256 = "603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4";
        let ecb_256 = config(AesMode::Ecb, key_256, "");
        assert_eq!(hex(&encrypt(&ecb_256, &plain).unwrap()[..16]), "f3eed1bdb5d2a03c064b5a7e3db181f8");
    }

    #[test]
    fn gcm_matches_a_published_vector() {
        // McGrew & Viega, test case 2: zero key, zero nonce, one zero block.
        let gcm = config(AesMode::Gcm, &"00".repeat(16), &"00".repeat(12));
        let output = encrypt(&gcm, &[0u8; 16]).unwrap();
        assert_eq!(
            hex(&output),
            "0388dace60b6a392f328c2b971b2fe78ab6e47d42cec13bdf53a67b21257bddf"
        );
        assert_eq!(decrypt(&gcm, &output).unwrap(), [0u8; 16]);
    }

    #[test]
    fn every_mode_and_key_size_round_trips() {
        let message = b"nohut: not a multiple of sixteen bytes";
        for key_bytes in [16, 24, 32] {
            for mode in [AesMode::Cbc, AesMode::Ctr, AesMode::Gcm, AesMode::Ecb] {
                let iv = if mode == AesMode::Gcm { "0a".repeat(12) } else { "0a".repeat(16) };
                let config = config(mode, &"1f".repeat(key_bytes), &iv);
                let encrypted = encrypt(&config, message).unwrap();
                assert_ne!(encrypted, message);
                assert_eq!(decrypt(&config, &encrypted).unwrap(), message, "{mode:?} {key_bytes}");
            }
        }
    }

    #[test]
    fn a_text_key_is_used_byte_for_byte() {
        let text = AesConfig {
            mode: AesMode::Ecb,
            key: "YELLOW SUBMARINE".into(),
            key_format: KeyFormat::Text,
            iv: String::new(),
        };
        let hex_key = config(AesMode::Ecb, &hex(b"YELLOW SUBMARINE"), "");
        assert_eq!(encrypt(&text, b"x").unwrap(), encrypt(&hex_key, b"x").unwrap());
    }

    #[test]
    fn bad_keys_and_ivs_are_config_errors() {
        let cases = [
            config(AesMode::Cbc, "", &"00".repeat(16)),
            config(AesMode::Cbc, "0011", &"00".repeat(16)),
            config(AesMode::Cbc, KEY_128, ""),
            config(AesMode::Cbc, KEY_128, "zz"),
            config(AesMode::Gcm, KEY_128, &"00".repeat(16)),
        ];
        for case in cases {
            assert_eq!(encrypt(&case, b"x").unwrap_err().code, ErrorCode::InvalidConfig, "{case:?}");
        }
    }

    #[test]
    fn wrong_keys_and_damaged_data_are_input_errors() {
        let gcm = config(AesMode::Gcm, KEY_128, &"00".repeat(12));
        let mut encrypted = encrypt(&gcm, b"authenticated").unwrap();
        encrypted[0] ^= 1;
        assert_eq!(decrypt(&gcm, &encrypted).unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(decrypt(&gcm, b"short").unwrap_err().code, ErrorCode::InvalidInput);

        let cbc = config(AesMode::Cbc, KEY_128, &"00".repeat(16));
        assert_eq!(decrypt(&cbc, b"seventeen bytes!!").unwrap_err().code, ErrorCode::InvalidInput);
        assert_eq!(decrypt(&cbc, b"").unwrap_err().code, ErrorCode::InvalidInput);
    }
}
