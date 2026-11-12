//! Keyed hashes and checksums.

use hmac::{Hmac, Mac};
use md5::Md5;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use tsify::Tsify;

use super::key::{parse_key, KeyFormat};
use crate::core::{NoConfig, Op, OpError, ParamKind, ParamSpec, Tag};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum HmacHash {
    Md5,
    Sha1,
    #[default]
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default, rename_all = "camelCase")]
pub struct HmacConfig {
    pub hash: HmacHash,
    pub key: String,
    pub key_format: KeyFormat,
}

fn hex(bytes: &[u8]) -> String {
    hex_simd::encode_to_string(bytes, hex_simd::AsciiCase::Lower)
}

pub struct HmacOp;

impl Op for HmacOp {
    const ID: &'static str = "hmac";
    const NAME: &'static str = "HMAC";
    const DESCRIPTION: &'static str =
        "Keyed hash of the input, as hex: proves it came from someone who has the key";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/HMAC");
    const TAGS: &'static [Tag] = &[Tag::Hashing];

    type Config = HmacConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        let hashes = ["md5", "sha1", "sha256", "sha512"].map(String::from).to_vec();
        vec![
            ParamSpec::new("hash", "Hash", ParamKind::Choice { options: hashes }),
            ParamSpec::new("key", "Key", ParamKind::Text),
            ParamSpec::new(
                "keyFormat",
                "Key format",
                ParamKind::Choice { options: KeyFormat::options() },
            ),
        ]
    }

    fn run(config: &HmacConfig, input: &[u8]) -> Result<String, OpError> {
        let key = parse_key(&config.key, config.key_format)?;
        macro_rules! mac {
            ($hash:ty) => {{
                // HMAC accepts a key of any length.
                let mut mac = Hmac::<$hash>::new_from_slice(&key).expect("any key length");
                mac.update(input);
                hex(&mac.finalize().into_bytes())
            }};
        }
        Ok(match config.hash {
            HmacHash::Md5 => mac!(Md5),
            HmacHash::Sha1 => mac!(Sha1),
            HmacHash::Sha256 => mac!(Sha256),
            HmacHash::Sha512 => mac!(Sha512),
        })
    }
}

pub struct Crc32;

impl Op for Crc32 {
    const ID: &'static str = "crc32";
    const NAME: &'static str = "CRC32";
    const DESCRIPTION: &'static str =
        "The CRC-32 checksum used by zip, gzip and PNG, as eight hex digits";
    const LINK: Option<&'static str> =
        Some("https://en.wikipedia.org/wiki/Cyclic_redundancy_check");
    const TAGS: &'static [Tag] = &[Tag::Hashing];

    type Config = NoConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(format!("{:08x}", crc32fast::hash(input)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    fn hmac(hash: HmacHash, key: &str, key_format: KeyFormat, input: &[u8]) -> String {
        HmacOp::run(&HmacConfig { hash, key: key.into(), key_format }, input).unwrap()
    }

    #[test]
    fn rfc_vectors() {
        // RFC 4231 test case 2 and RFC 2202 test cases 2.
        let (key, data) = ("Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hmac(HmacHash::Sha256, key, KeyFormat::Text, data),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hmac(HmacHash::Sha512, key, KeyFormat::Text, data),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea250554\
             9758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
        assert_eq!(hmac(HmacHash::Md5, key, KeyFormat::Text, data), "750c783e6ab0b503eaa86e310a5db738");
        assert_eq!(
            hmac(HmacHash::Sha1, key, KeyFormat::Text, data),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
        // RFC 4231 test case 1: a key given as bytes.
        assert_eq!(
            hmac(HmacHash::Sha256, &"0b".repeat(20), KeyFormat::Hex, b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn an_empty_key_is_a_config_error() {
        let error = HmacOp::run(&HmacConfig::default(), b"x").unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn crc32_check_value() {
        assert_eq!(Crc32::run(&NoConfig {}, b"123456789").unwrap(), "cbf43926");
        assert_eq!(Crc32::run(&NoConfig {}, b"").unwrap(), "00000000");
    }
}
