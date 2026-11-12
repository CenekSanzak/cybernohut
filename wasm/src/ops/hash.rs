use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256 as Sha256Hasher, Sha512 as Sha512Hasher};

use crate::core::{NoConfig, Op, OpError, Tag};

fn hex_digest<D: Digest>(input: &[u8]) -> String {
    D::digest(input).iter().map(|byte| format!("{byte:02x}")).collect()
}

macro_rules! hash_op {
    ($name:ident, $hasher:ty, $id:literal, $label:literal, $link:literal) => {
        pub struct $name;

        impl Op for $name {
            const ID: &'static str = $id;
            const NAME: &'static str = $label;
            const DESCRIPTION: &'static str =
                concat!("Hashes the input with ", $label, " and outputs the digest as hex");
            const LINK: Option<&'static str> = Some($link);
            const TAGS: &'static [Tag] = &[Tag::Hashing];

            type Config = NoConfig;
            type In<'a> = &'a [u8];
            type Out = String;

            fn run(_: &NoConfig, input: &[u8]) -> Result<String, OpError> {
                Ok(hex_digest::<$hasher>(input))
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
        assert_eq!(Md5Hash::run(&NoConfig {}, b"abc").unwrap(), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            Sha1Hash::run(&NoConfig {}, b"abc").unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            Sha256::run(&NoConfig {}, b"abc").unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha512_known_vector() {
        assert_eq!(
            Sha512::run(&NoConfig {}, b"abc").unwrap(),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn multi_line_input_is_hashed_as_a_whole() {
        // `printf 'a\nb' | sha256sum`
        assert_eq!(
            Sha256::run(&NoConfig {}, b"a\nb").unwrap(),
            "7e18f737311b2dc3b2f269dd78396b0351f14fb66efa879f768cb23181883c78"
        );
    }
}
