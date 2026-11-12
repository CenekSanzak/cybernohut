//! Every operation, and the one list that registers them.

pub mod aes;
pub mod analysis;
pub mod base64;
pub mod base58;
pub mod base64_custom;
pub mod caesar;
pub mod compare;
pub mod compress;
pub mod encoding;
pub mod frequency;
pub mod hash;
pub mod json;
pub mod key;
pub mod mac;
pub mod misc;
pub mod number_base;
pub mod regex;
pub mod rsa;
pub mod stego;
pub mod text;
pub mod time;
pub mod url;
pub mod xor;

use std::sync::OnceLock;

use wasm_bindgen::prelude::*;

use crate::core::{entry, NoConfig, Op, OpEntry};
use base64::{Base64StandardDecode, Base64StandardEncode, Base64UrlDecode, Base64UrlEncode};
use aes::{AesConfig, AesDecrypt, AesEncrypt};
use analysis::{DetectFileType, Entropy};
use compress::{Gunzip, Gzip, RawDeflate, RawInflate, ZlibDeflate, ZlibInflate};
use json::{JsonFormat, JsonFormatConfig, JsonMinify, JwtDecode};
use mac::{Crc32, HmacConfig, HmacOp};
use stego::{ImageMetadata, LsbConfig, LsbEmbed, LsbExtract, TrailingData};
use time::{FromUnixTimestamp, TimestampConfig, ToUnixTimestamp};
use caesar::{Atbash, CaesarCipher, CaesarConfig, Rot13, Vigenere, VigenereConfig};
use base64_custom::{Base64CustomConfig, Base64CustomDecode, Base64CustomEncode};
use compare::Compare;
use frequency::{CharacterFrequency, DetectLanguage, FrequencyConfig};
use rsa::{
    BytesToInteger, IntegerConfig, IntegerToBytes, RsaAnalyze, RsaAnalyzeConfig, RsaDecrypt,
    RsaDecryptConfig, RsaEncrypt, RsaEncryptConfig, RsaParseKey,
};
use base58::{Base58Decode, Base58Encode};
use hash::{HashConfig, Keccak256, Md5Hash, Sha1Hash, Sha256, Sha3_256, Sha3_512, Sha512};
use misc::{
    EscapeText, FromHexdump, FromMorse, HtmlEntityDecode, HtmlEntityEncode, Rot47, SortLines,
    SortLinesConfig, Strings, StringsConfig, ToHexdump, ToMorse, UnescapeText, UniqueLines,
};
use number_base::{FromBinary, FromDecimal, FromOctal, ToBinary, ToDecimal, ToOctal};
use self::regex::{
    ExtractPattern, ExtractPatternConfig, RegexExtract, RegexExtractConfig, RegexReplace,
    RegexReplaceConfig,
};
use url::{UrlDecode, UrlDecodeConfig, UrlEncode};
use xor::{Xor, XorConfig};
use encoding::{
    Base16Decode, Base16Encode, Base32Decode, Base32Encode, Base85Decode, Base85Encode,
    HexEncodeConfig,
};
use text::{
    CamelCase, ConcatenateConfig, ConcatenateText, CountWords, DelimiterConfig, JoinText,
    KebabCase, Lowercase, RemoveSpaces, Reverse, SliceConfig, SliceText, SnakeCase, SplitText,
    TextStats, TitleCase, Uppercase, Utf8Decode, Utf8DecodeConfig,
};

/// Registers operations and emits the TypeScript `OpConfigMap` that ties each id to its
/// config type. The id has to be repeated as a literal because `concat!` cannot read an
/// associated const; a test checks the two never disagree, and the `same_config` bound
/// makes a wrong config type a compile error.
macro_rules! ops {
    ($($id:literal => $op:ty : $config:ident),* $(,)?) => {
        pub fn registry() -> &'static [OpEntry] {
            static REGISTRY: OnceLock<Vec<OpEntry>> = OnceLock::new();
            REGISTRY.get_or_init(|| vec![$(entry::<$op>()),*])
        }

        #[cfg(test)]
        const DECLARED_IDS: &[(&str, &str)] = &[$(($id, <$op as Op>::ID)),*];

        const _: fn() = || {
            fn same_config<T: Op<Config = C>, C>() {}
            $(same_config::<$op, $config>();)*
        };

        #[wasm_bindgen(typescript_custom_section)]
        const OP_CONFIG_MAP: &'static str = concat!(
            "export interface OpConfigMap {\n",
            $("    \"", $id, "\": ", stringify!($config), ";\n",)*
            "}\n\nexport type OpId = keyof OpConfigMap;\n",
        );
    };
}

ops! {
    "base64-standard-encode" => Base64StandardEncode: NoConfig,
    "base64-standard-decode" => Base64StandardDecode: NoConfig,
    "base64-url-encode" => Base64UrlEncode: NoConfig,
    "base64-url-decode" => Base64UrlDecode: NoConfig,
    "md5" => Md5Hash: HashConfig,
    "sha1" => Sha1Hash: HashConfig,
    "sha256" => Sha256: HashConfig,
    "caesar-cipher" => CaesarCipher: CaesarConfig,
    "rot13" => Rot13: NoConfig,
    "utf8-decode" => Utf8Decode: Utf8DecodeConfig,
    "base16-encode" => Base16Encode: HexEncodeConfig,
    "base16-decode" => Base16Decode: NoConfig,
    "base32-encode" => Base32Encode: NoConfig,
    "base32-decode" => Base32Decode: NoConfig,
    "base85-encode" => Base85Encode: NoConfig,
    "base85-decode" => Base85Decode: NoConfig,
    "reverse" => Reverse: NoConfig,
    "to-uppercase" => Uppercase: NoConfig,
    "to-lowercase" => Lowercase: NoConfig,
    "remove-spaces" => RemoveSpaces: NoConfig,
    "slice-text" => SliceText: SliceConfig,
    "count-words" => CountWords: NoConfig,
    "text-stats" => TextStats: NoConfig,
    "split-text" => SplitText: DelimiterConfig,
    "join-text" => JoinText: DelimiterConfig,
    "concatenate-text" => ConcatenateText: ConcatenateConfig,
    "regex-extract" => RegexExtract: RegexExtractConfig,
    "regex-replace" => RegexReplace: RegexReplaceConfig,
    "sha512" => Sha512: HashConfig,
    "xor" => Xor: XorConfig,
    "atbash" => Atbash: NoConfig,
    "vigenere" => Vigenere: VigenereConfig,
    "url-encode" => UrlEncode: NoConfig,
    "url-decode" => UrlDecode: UrlDecodeConfig,
    "to-binary" => ToBinary: NoConfig,
    "from-binary" => FromBinary: NoConfig,
    "to-octal" => ToOctal: NoConfig,
    "from-octal" => FromOctal: NoConfig,
    "to-decimal" => ToDecimal: NoConfig,
    "from-decimal" => FromDecimal: NoConfig,
    "detect-file-type" => DetectFileType: NoConfig,
    "entropy" => Entropy: NoConfig,
    "gzip" => Gzip: NoConfig,
    "gunzip" => Gunzip: NoConfig,
    "zlib-deflate" => ZlibDeflate: NoConfig,
    "zlib-inflate" => ZlibInflate: NoConfig,
    "raw-deflate" => RawDeflate: NoConfig,
    "raw-inflate" => RawInflate: NoConfig,
    "aes-encrypt" => AesEncrypt: AesConfig,
    "aes-decrypt" => AesDecrypt: AesConfig,
    "hmac" => HmacOp: HmacConfig,
    "crc32" => Crc32: NoConfig,
    "json-format" => JsonFormat: JsonFormatConfig,
    "json-minify" => JsonMinify: NoConfig,
    "jwt-decode" => JwtDecode: NoConfig,
    "from-unix-timestamp" => FromUnixTimestamp: TimestampConfig,
    "to-unix-timestamp" => ToUnixTimestamp: TimestampConfig,
    "base64-custom-encode" => Base64CustomEncode: Base64CustomConfig,
    "base64-custom-decode" => Base64CustomDecode: Base64CustomConfig,
    "compare" => Compare: NoConfig,
    "to-title-case" => TitleCase: NoConfig,
    "to-camel-case" => CamelCase: NoConfig,
    "to-snake-case" => SnakeCase: NoConfig,
    "to-kebab-case" => KebabCase: NoConfig,
    "extract-pattern" => ExtractPattern: ExtractPatternConfig,
    "character-frequency" => CharacterFrequency: FrequencyConfig,
    "detect-language" => DetectLanguage: NoConfig,
    "bytes-to-integer" => BytesToInteger: IntegerConfig,
    "integer-to-bytes" => IntegerToBytes: IntegerConfig,
    "rsa-encrypt" => RsaEncrypt: RsaEncryptConfig,
    "rsa-decrypt" => RsaDecrypt: RsaDecryptConfig,
    "rsa-analyze-modulus" => RsaAnalyze: RsaAnalyzeConfig,
    "rsa-parse-key" => RsaParseKey: NoConfig,
    "extract-trailing-data" => TrailingData: NoConfig,
    "image-metadata" => ImageMetadata: NoConfig,
    "lsb-extract" => LsbExtract: LsbConfig,
    "lsb-embed" => LsbEmbed: LsbConfig,
    "strings" => Strings: StringsConfig,
    "to-hexdump" => ToHexdump: NoConfig,
    "from-hexdump" => FromHexdump: NoConfig,
    "to-morse" => ToMorse: NoConfig,
    "from-morse" => FromMorse: NoConfig,
    "html-entity-decode" => HtmlEntityDecode: NoConfig,
    "html-entity-encode" => HtmlEntityEncode: NoConfig,
    "rot47" => Rot47: NoConfig,
    "unescape-text" => UnescapeText: NoConfig,
    "escape-text" => EscapeText: NoConfig,
    "sort-lines" => SortLines: SortLinesConfig,
    "unique-lines" => UniqueLines: NoConfig,
    "base58-encode" => Base58Encode: NoConfig,
    "base58-decode" => Base58Decode: NoConfig,
    "sha3-256" => Sha3_256: HashConfig,
    "sha3-512" => Sha3_512: HashConfig,
    "keccak-256" => Keccak256: HashConfig,
}

pub fn find(id: &str) -> Option<&'static OpEntry> {
    registry().iter().find(|entry| entry.id == id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::core::{ErrorCode, Value, ValueType};

    #[test]
    fn declared_ids_match_the_operations() {
        for (declared, actual) in DECLARED_IDS {
            assert_eq!(declared, actual);
        }
    }

    #[test]
    fn ids_are_unique() {
        let ids: HashSet<_> = registry().iter().map(|entry| entry.id).collect();
        assert_eq!(ids.len(), registry().len());
    }

    #[test]
    fn params_and_config_fields_agree() {
        for entry in registry() {
            let meta = (entry.meta)();
            let fields: HashSet<String> = meta
                .default_config
                .as_object()
                .expect("config serializes to an object")
                .keys()
                .cloned()
                .collect();
            let params: HashSet<String> = meta.params.iter().map(|p| p.key.clone()).collect();
            assert_eq!(fields, params, "{}: config fields and params differ", meta.id);
        }
    }

    #[test]
    fn text_is_accepted_where_bytes_are_expected() {
        let input = Value::Text("abc".into());
        let output = (find("sha256").unwrap().run)(&serde_json::Value::Null, &[&input]).unwrap();
        assert_eq!(
            output,
            vec![Value::Text(
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into()
            )]
        );
    }

    #[test]
    fn bytes_are_rejected_where_text_is_expected() {
        let input = Value::Bytes(vec![1, 2, 3]);
        let error = (find("caesar-cipher").unwrap().run)(&serde_json::Value::Null, &[&input])
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::TypeMismatch);
    }

    #[test]
    fn partial_and_invalid_configs() {
        let run = find("caesar-cipher").unwrap().run;
        let input = Value::Text("abc".into());
        let partial = serde_json::json!({ "shift": 1 });
        assert_eq!(run(&partial, &[&input]).unwrap(), vec![Value::Text("bcd".into())]);
        let invalid = serde_json::json!({ "shift": "one" });
        assert_eq!(run(&invalid, &[&input]).unwrap_err().code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn port_types_come_from_the_signature() {
        let meta = (find("base64-standard-decode").unwrap().meta)();
        assert_eq!(meta.inputs[0].ty, ValueType::Text);
        assert_eq!(meta.outputs[0].ty, ValueType::Bytes);
    }
}
