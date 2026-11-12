//! Every operation, and the one list that registers them.

pub mod analysis;
pub mod base64;
pub mod caesar;
pub mod encoding;
pub mod hash;
pub mod number_base;
pub mod regex;
pub mod text;
pub mod url;
pub mod xor;

use std::sync::OnceLock;

use wasm_bindgen::prelude::*;

use crate::core::{entry, NoConfig, Op, OpEntry};
use base64::{Base64StandardDecode, Base64StandardEncode, Base64UrlDecode, Base64UrlEncode};
use analysis::{DetectFileType, Entropy};
use caesar::{Atbash, CaesarCipher, CaesarConfig, Rot13, Vigenere, VigenereConfig};
use hash::{Md5Hash, Sha1Hash, Sha256, Sha512};
use number_base::{FromBinary, FromDecimal, FromOctal, ToBinary, ToDecimal, ToOctal};
use self::regex::{RegexExtract, RegexExtractConfig, RegexReplace, RegexReplaceConfig};
use url::{UrlDecode, UrlDecodeConfig, UrlEncode};
use xor::{Xor, XorConfig};
use encoding::{
    Base16Decode, Base16Encode, Base32Decode, Base32Encode, Base85Decode, Base85Encode,
    HexEncodeConfig,
};
use text::{
    ConcatenateConfig, ConcatenateText, CountWords, DelimiterConfig, JoinText, Lowercase,
    RemoveSpaces, Reverse, SliceConfig, SliceText, SplitText, TextStats, Uppercase, Utf8Decode,
    Utf8DecodeConfig,
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
    "md5" => Md5Hash: NoConfig,
    "sha1" => Sha1Hash: NoConfig,
    "sha256" => Sha256: NoConfig,
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
    "sha512" => Sha512: NoConfig,
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
