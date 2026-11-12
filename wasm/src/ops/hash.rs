use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256 as Sha256Hasher, Sha512 as Sha512Hasher};

use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum DigestFormat {
    #[default]
    Hex,
    Base64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct HashConfig {
    /// How the digest is written out.
    pub output: DigestFormat,
}

fn digest<D: Digest>(config: &HashConfig, input: &[u8]) -> String {
    let digest = D::digest(input);
    match config.output {
        DigestFormat::Hex => hex_simd::encode_to_string(&digest, hex_simd::AsciiCase::Lower),
        DigestFormat::Base64 => base64_simd::STANDARD.encode_to_string(&digest),
    }
}

macro_rules! hash_op {
    ($name:ident, $hasher:ty, $id:literal, $label:literal, $link:literal) => {
        pub struct $name;

        impl Op for $name {
            const ID: &'static str = $id;
            const NAME: &'static str = $label;
            const DESCRIPTION: &'static str =
                concat!("Hashes the input with ", $label, "; the digest is written as hex or Base64");
            const LINK: Option<&'static str> = Some($link);
            const TAGS: &'static [Tag] = &[Tag::Hashing];

            type Config = HashConfig;
            type In<'a> = &'a [u8];
            type Out = String;

            fn params() -> Vec<ParamSpec> {
                let formats = ["hex", "base64"].map(String::from).to_vec();
                vec![ParamSpec::new("output", "Output", ParamKind::Choice { options: formats })]
            }

            fn run(config: &HashConfig, input: &[u8]) -> Result<String, OpError> {
                Ok(digest::<$hasher>(config, input))
            }
        }
    };
}

hash_op!(Md5Hash, Md5, "md5", "MD5", "https://en.wikipedia.org/wiki/MD5");
hash_op!(Sha1Hash, Sha1, "sha1", "SHA1", "https://en.wikipedia.org/wiki/SHA-1");
hash_op!(Sha256, Sha256Hasher, "sha256", "SHA256", "https://en.wikipedia.org/wiki/SHA-2");
hash_op!(Sha512, Sha512Hasher, "sha512", "SHA512", "https://en.wikipedia.org/wiki/SHA-2");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        assert_eq!(Md5Hash::run(&HashConfig::default(), b"abc").unwrap(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            Sha1Hash::run(&HashConfig::default(), b"abc").unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            Sha256::run(&HashConfig::default(), b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_digest_can_be_written_as_base64() {
        // `printf abc | openssl dgst -sha256 -binary | base64`
        let base64 = HashConfig { output: DigestFormat::Base64 };
        assert_eq!(Sha256::run(&base64, b"abc").unwrap(), "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
    }

    #[test]
    fn sha512_known_vector() {
        assert_eq!(
            Sha512::run(&HashConfig::default(), b"abc").unwrap(),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn multi_line_input_is_hashed_as_a_whole() {
        // `printf 'a\nb' | sha256sum`
        assert_eq!(
            Sha256::run(&HashConfig::default(), b"a\nb").unwrap(),
            "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78"
        );
    }
}
