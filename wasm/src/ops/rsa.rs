//! Big integers, and RSA as the arithmetic it is.
//!
//! This is "textbook" RSA: modular exponentiation with the numbers you give it. It is
//! for understanding RSA and for challenges built on its weaknesses, not for protecting
//! anything.

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

use crate::core::{Op, OpError, ParamKind, ParamSpec, Tag};

const LINK: &str = "https://en.wikipedia.org/wiki/RSA_(cryptosystem)";

/// A whole number in decimal, or in hexadecimal with a `0x` prefix. Spaces, line
/// breaks, underscores and colons between digits are ignored, since long numbers are
/// pasted in many shapes.
fn parse_number(text: &str) -> Option<BigUint> {
    let compact: String =
        text.chars().filter(|c| !c.is_whitespace() && !matches!(c, '_' | ':' | ',')).collect();
    match compact.strip_prefix("0x").or_else(|| compact.strip_prefix("0X")) {
        Some(hex) => BigUint::parse_bytes(hex.as_bytes(), 16),
        None => BigUint::parse_bytes(compact.as_bytes(), 10),
    }
}

fn setting(text: &str, name: &str) -> Result<Option<BigUint>, OpError> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    parse_number(text).map(Some).ok_or_else(|| {
        OpError::invalid_config(format!("{name} is not a whole number (decimal, or hex with 0x)"))
    })
}

fn required(text: &str, name: &str) -> Result<BigUint, OpError> {
    setting(text, name)?.ok_or_else(|| OpError::invalid_config(format!("Enter {name}")))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum Endian {
    #[default]
    Big,
    Little,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct IntegerConfig {
    /// Which end of the bytes holds the most significant digit.
    pub endian: Endian,
}

fn integer_params() -> Vec<ParamSpec> {
    let options = ["big", "little"].map(String::from).to_vec();
    vec![ParamSpec::new("endian", "Byte order", ParamKind::Choice { options })]
}

pub struct BytesToInteger;

impl Op for BytesToInteger {
    const ID: &'static str = "bytes-to-integer";
    const NAME: &'static str = "Bytes to Integer";
    const DESCRIPTION: &'static str = "Reads all the bytes as one whole number, written in decimal";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Endianness");
    const TAGS: &'static [Tag] = &[Tag::NumberBase];

    type Config = IntegerConfig;
    type In<'a> = &'a [u8];
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        integer_params()
    }

    fn run(config: &IntegerConfig, input: &[u8]) -> Result<String, OpError> {
        Ok(match config.endian {
            Endian::Big => BigUint::from_bytes_be(input),
            Endian::Little => BigUint::from_bytes_le(input),
        }
        .to_string())
    }
}

pub struct IntegerToBytes;

impl Op for IntegerToBytes {
    const ID: &'static str = "integer-to-bytes";
    const NAME: &'static str = "Integer to Bytes";
    const DESCRIPTION: &'static str =
        "Writes a whole number of any size (decimal, or hex with 0x) as bytes";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Endianness");
    const TAGS: &'static [Tag] = &[Tag::NumberBase];

    type Config = IntegerConfig;
    type In<'a> = &'a str;
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        integer_params()
    }

    fn run(config: &IntegerConfig, input: &str) -> Result<Vec<u8>, OpError> {
        let number = parse_number(input).ok_or_else(|| {
            OpError::invalid_input("Not a whole number (decimal, or hex with 0x)")
        })?;
        Ok(match config.endian {
            Endian::Big => number.to_bytes_be(),
            Endian::Little => number.to_bytes_le(),
        })
    }
}

/// `value` to the power -1 modulo `modulus`, if it exists.
fn inverse(value: &BigUint, modulus: &BigUint) -> Option<BigUint> {
    let (value, modulus) = (BigInt::from(value.clone()), BigInt::from(modulus.clone()));
    let extended = value.extended_gcd(&modulus);
    if !extended.gcd.is_one() {
        return None;
    }
    let (sign, magnitude) = extended.x.mod_floor(&modulus).into_parts();
    (sign != Sign::Minus).then_some(magnitude)
}

/// How far the search for a factor goes before giving up. Both bounds keep the
/// search to a fraction of a second on a 4096-bit modulus, since it runs again
/// whenever the graph does.
const TRIAL_LIMIT: u32 = 1_000_000;
const FERMAT_STEPS: u32 = 5000;

/// The two factors of a modulus that is weak in one of the ways challenges like: two
/// primes close to each other, or a small prime factor. A sound modulus is not
/// factored by this or by anything else in reasonable time.
fn factor(n: &BigUint) -> Option<(BigUint, BigUint, &'static str)> {
    if n < &BigUint::from(4u8) {
        return None;
    }
    let split = |p: BigUint, how| {
        let q = n / &p;
        (p, q, how)
    };
    if n.is_even() {
        return Some(split(BigUint::from(2u8), "it is even"));
    }
    // Fermat: n = a² - b² = (a - b)(a + b), found in a few steps when the primes are
    // close, which is what happens when one is picked as "the next prime" after the
    // other.
    let mut a = n.sqrt();
    if &(&a * &a) < n {
        a += 1u8;
    }
    for _ in 0..FERMAT_STEPS {
        let b_squared = &a * &a - n;
        let b = b_squared.sqrt();
        if &b * &b == b_squared {
            let p = &a - &b;
            if !p.is_one() {
                return Some(split(p, "its two primes are close together (Fermat's method)"));
            }
        }
        a += 1u8;
    }
    (3..TRIAL_LIMIT)
        .step_by(2)
        .find(|candidate| (n % *candidate).is_zero())
        .map(|small| split(BigUint::from(small), "it has a small prime factor"))
}

fn private_exponent(p: &BigUint, q: &BigUint, e: &BigUint) -> Result<BigUint, OpError> {
    let one = BigUint::one();
    if p <= &one || q <= &one {
        return Err(OpError::invalid_config("p and q have to be primes larger than 1"));
    }
    let phi = if p == q { p * (p - &one) } else { (p - &one) * (q - &one) };
    inverse(e, &phi).ok_or_else(|| {
        OpError::invalid_config("e shares a factor with (p-1)(q-1), so there is no private exponent")
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct RsaEncryptConfig {
    /// The modulus.
    pub n: String,
    /// The public exponent.
    pub e: String,
}

impl Default for RsaEncryptConfig {
    fn default() -> Self {
        RsaEncryptConfig { n: String::new(), e: "65537".into() }
    }
}

pub struct RsaEncrypt;

impl Op for RsaEncrypt {
    const ID: &'static str = "rsa-encrypt";
    const NAME: &'static str = "RSA Encrypt (Textbook)";
    const DESCRIPTION: &'static str =
        "Raises the input, read as a number, to the power e modulo n. No padding: for study, not for secrets";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encryption];

    type Config = RsaEncryptConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        vec![
            ParamSpec::new("n", "Modulus n", ParamKind::Text),
            ParamSpec::new("e", "Exponent e", ParamKind::Text),
        ]
    }

    fn run(config: &RsaEncryptConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let n = required(&config.n, "the modulus n")?;
        let e = required(&config.e, "the exponent e")?;
        let message = BigUint::from_bytes_be(input);
        if message >= n {
            return Err(OpError::invalid_input(format!(
                "The input is {} bytes, too long for a {}-bit modulus",
                input.len(),
                n.bits()
            )));
        }
        Ok(message.modpow(&e, &n).to_bytes_be())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum RsaPadding {
    #[default]
    None,
    Pkcs1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct RsaDecryptConfig {
    pub n: String,
    /// The private exponent. When empty it is worked out from p, q and e.
    pub d: String,
    pub p: String,
    pub q: String,
    pub e: String,
    /// Padding to remove from the result.
    pub padding: RsaPadding,
}

impl Default for RsaDecryptConfig {
    fn default() -> Self {
        RsaDecryptConfig {
            n: String::new(),
            d: String::new(),
            p: String::new(),
            q: String::new(),
            e: "65537".into(),
            padding: RsaPadding::None,
        }
    }
}

/// The modulus and private exponent the settings describe, from whichever of d, the
/// primes, or a weak modulus is available.
fn private_key(config: &RsaDecryptConfig) -> Result<(BigUint, BigUint), OpError> {
    let n = setting(&config.n, "The modulus n")?;
    let d = setting(&config.d, "The private exponent d")?;
    let p = setting(&config.p, "The prime p")?;
    let q = setting(&config.q, "The prime q")?;
    if let Some(d) = d {
        let n = match (n, &p, &q) {
            (Some(n), _, _) => n,
            (None, Some(p), Some(q)) => p * q,
            _ => return Err(OpError::invalid_config("Enter the modulus n")),
        };
        return Ok((n, d));
    }
    let e = required(&config.e, "the exponent e")?;
    let (p, q) = match (p, q, &n) {
        (Some(p), Some(q), _) => (p, q),
        (Some(known), None, Some(n)) | (None, Some(known), Some(n)) => {
            if known.is_zero() || !(n % &known).is_zero() {
                return Err(OpError::invalid_config("The prime given does not divide n"));
            }
            (n / &known, known)
        }
        (None, None, Some(n)) => {
            let (p, q, _) = factor(n).ok_or_else(|| {
                OpError::invalid_config(
                    "Enter d, or the primes p and q: this n has no small factor and its primes are not close, so it cannot be factored here",
                )
            })?;
            (p, q)
        }
        _ => return Err(OpError::invalid_config("Enter the modulus n, or both primes p and q")),
    };
    let n = match n {
        Some(n) if n != &p * &q => {
            return Err(OpError::invalid_config("p times q is not n"));
        }
        Some(n) => n,
        None => &p * &q,
    };
    Ok((n, private_exponent(&p, &q, &e)?))
}

pub struct RsaDecrypt;

impl Op for RsaDecrypt {
    const ID: &'static str = "rsa-decrypt";
    const NAME: &'static str = "RSA Decrypt (Textbook)";
    const DESCRIPTION: &'static str =
        "Raises the input to the power d modulo n. Give d, or p and q, or only n and e if n is weak enough to factor";
    const LINK: Option<&'static str> = Some(LINK);
    const TAGS: &'static [Tag] = &[Tag::Encryption];

    type Config = RsaDecryptConfig;
    type In<'a> = &'a [u8];
    type Out = Vec<u8>;

    fn params() -> Vec<ParamSpec> {
        let paddings = ["none", "pkcs1"].map(String::from).to_vec();
        vec![
            ParamSpec::new("n", "Modulus n", ParamKind::Text),
            ParamSpec::new("d", "Private exponent d", ParamKind::Text),
            ParamSpec::new("p", "Prime p", ParamKind::Text),
            ParamSpec::new("q", "Prime q", ParamKind::Text),
            ParamSpec::new("e", "Exponent e", ParamKind::Text),
            ParamSpec::new("padding", "Remove padding", ParamKind::Choice { options: paddings }),
        ]
    }

    fn run(config: &RsaDecryptConfig, input: &[u8]) -> Result<Vec<u8>, OpError> {
        let (n, d) = private_key(config)?;
        let cipher = BigUint::from_bytes_be(input);
        if cipher >= n {
            return Err(OpError::invalid_input("The input is a larger number than the modulus"));
        }
        let plain = cipher.modpow(&d, &n).to_bytes_be();
        match config.padding {
            RsaPadding::None => Ok(plain),
            // 00 02, at least eight non-zero bytes, 00, then the message. The leading
            // zero is gone because the block was read as a number.
            RsaPadding::Pkcs1 => match plain.as_slice() {
                [2, rest @ ..] => rest
                    .iter()
                    .position(|byte| *byte == 0)
                    .filter(|end| *end >= 8)
                    .map(|end| rest[end + 1..].to_vec()),
                _ => None,
            }
            .ok_or_else(|| {
                OpError::invalid_input("The result is not PKCS#1 v1.5 padded: wrong key, or no padding was used")
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(default)]
pub struct RsaAnalyzeConfig {
    pub e: String,
}

impl Default for RsaAnalyzeConfig {
    fn default() -> Self {
        RsaAnalyzeConfig { e: "65537".into() }
    }
}

pub struct RsaAnalyze;

impl Op for RsaAnalyze {
    const ID: &'static str = "rsa-analyze-modulus";
    const NAME: &'static str = "RSA Analyze Modulus";
    const DESCRIPTION: &'static str =
        "Takes a modulus n and looks for the weaknesses that let it be factored, giving p, q and d if it finds one";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Fermat%27s_factorization_method");
    const TAGS: &'static [Tag] = &[Tag::Encryption, Tag::Analysis];

    type Config = RsaAnalyzeConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn params() -> Vec<ParamSpec> {
        vec![ParamSpec::new("e", "Exponent e", ParamKind::Text)]
    }

    fn run(config: &RsaAnalyzeConfig, input: &str) -> Result<String, OpError> {
        let n = parse_number(input).ok_or_else(|| {
            OpError::invalid_input("The input is not a whole number (decimal, or hex with 0x)")
        })?;
        let e = required(&config.e, "the exponent e")?;
        let mut report = format!("n has {} bits ({} decimal digits)\n", n.bits(), n.to_string().len());
        let Some((p, q, how)) = factor(&n) else {
            report.push_str(
                "Not factored: no prime factor below 1,000,000, and the primes are not close together",
            );
            return Ok(report);
        };
        report.push_str(&format!("Factored, because {how}\np = {p}\nq = {q}\n"));
        match private_exponent(&p, &q, &e) {
            Ok(d) => report.push_str(&format!("d = {d}")),
            Err(error) => report.push_str(&format!("No d for e = {e}: {}", error.message)),
        }
        Ok(report)
    }
}

/// One element of DER: its tag and its contents.
fn der_element(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = input.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        // The low bits say how many bytes the length itself takes.
        let count = usize::from(first & 0x7f);
        let (bytes, rest) = rest.split_at_checked(count).filter(|_| count <= 4)?;
        (bytes.iter().fold(0usize, |length, byte| length << 8 | usize::from(*byte)), rest)
    };
    let (contents, after) = rest.split_at_checked(length)?;
    Some((tag, contents, after))
}

const DER_INTEGER: u8 = 0x02;
const DER_BIT_STRING: u8 = 0x03;
const DER_OCTET_STRING: u8 = 0x04;
const DER_SEQUENCE: u8 = 0x30;

/// The numbers of the RSA key inside a DER structure, however it is wrapped: a bare
/// key is a sequence of integers, and the generic formats put that sequence inside a
/// bit string or an octet string next to an algorithm identifier.
fn der_key_numbers(der: &[u8]) -> Option<Vec<BigUint>> {
    let (tag, contents, _) = der_element(der)?;
    if tag != DER_SEQUENCE {
        return None;
    }
    let mut elements = Vec::new();
    let mut rest = contents;
    while !rest.is_empty() {
        let (tag, body, after) = der_element(rest)?;
        elements.push((tag, body));
        rest = after;
    }
    let integers: Vec<BigUint> = elements
        .iter()
        .filter(|(tag, _)| *tag == DER_INTEGER)
        .map(|(_, body)| BigUint::from_bytes_be(body))
        .collect();
    if integers.len() == elements.len() && integers.len() >= 2 {
        return Some(integers);
    }
    elements.iter().find_map(|(tag, body)| match *tag {
        // A bit string starts with the count of unused bits in its last byte.
        DER_BIT_STRING => der_key_numbers(body.get(1..)?),
        DER_OCTET_STRING => der_key_numbers(body),
        _ => None,
    })
}

pub struct RsaParseKey;

impl Op for RsaParseKey {
    const ID: &'static str = "rsa-parse-key";
    const NAME: &'static str = "RSA Parse Key";
    const DESCRIPTION: &'static str =
        "Reads an RSA key in PEM form (BEGIN PUBLIC KEY, BEGIN RSA PRIVATE KEY and the like) and lists its numbers";
    const LINK: Option<&'static str> = Some("https://en.wikipedia.org/wiki/Privacy-Enhanced_Mail");
    const TAGS: &'static [Tag] = &[Tag::Encryption, Tag::DataFormat];

    type Config = crate::core::NoConfig;
    type In<'a> = &'a str;
    type Out = String;

    fn run(_: &Self::Config, input: &str) -> Result<String, OpError> {
        if input.contains("ENCRYPTED") {
            return Err(OpError::invalid_input("The key is encrypted with a passphrase"));
        }
        // What is left without the BEGIN and END lines is Base64 of the DER structure.
        let body: String = input
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .flat_map(str::chars)
            .filter(|c| !c.is_whitespace())
            .collect();
        let der = base64_simd::STANDARD
            .decode_to_vec(body)
            .map_err(|_| OpError::invalid_input("Not a PEM key: the part between the BEGIN and END lines is not Base64"))?;
        let numbers = der_key_numbers(&der)
            .ok_or_else(|| OpError::invalid_input("No RSA key found in this PEM block"))?;
        Ok(match numbers.as_slice() {
            [n, e] => format!("RSA public key, {} bits\nn = {n}\ne = {e}", n.bits()),
            // A private key starts with a version number, then the numbers in order.
            [_, n, e, d, p, q, ..] => format!(
                "RSA private key, {} bits\nn = {n}\ne = {e}\nd = {d}\np = {p}\nq = {q}",
                n.bits()
            ),
            _ => return Err(OpError::invalid_input("No RSA key found in this PEM block")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ErrorCode;

    // The worked example from the RSA Wikipedia article.
    const N: &str = "3233";
    const E: &str = "17";
    const D: &str = "413";

    fn decrypt_config(n: &str, d: &str, p: &str, q: &str, e: &str) -> RsaDecryptConfig {
        RsaDecryptConfig { n: n.into(), d: d.into(), p: p.into(), q: q.into(), e: e.into(), padding: RsaPadding::None }
    }

    #[test]
    fn the_textbook_example() {
        let encrypt = RsaEncryptConfig { n: N.into(), e: E.into() };
        let cipher = RsaEncrypt::run(&encrypt, &[65]).unwrap();
        assert_eq!(BigUint::from_bytes_be(&cipher), BigUint::from(2790u32));
        // With d, with the primes, with one prime, and with nothing but a weak n.
        for config in [
            decrypt_config(N, D, "", "", ""),
            decrypt_config(N, "", "61", "53", E),
            decrypt_config("", "", "61", "53", E),
            decrypt_config(N, "", "61", "", E),
            decrypt_config(N, "", "", "", E),
        ] {
            assert_eq!(RsaDecrypt::run(&config, &cipher).unwrap(), [65], "{config:?}");
        }
    }

    #[test]
    fn a_modulus_of_close_primes_is_factored_and_used() {
        // Two primes around 2^64, a few thousand apart: far beyond trial division.
        let p = BigUint::parse_bytes(b"18446744073709551629", 10).unwrap();
        let q = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let n = (&p * &q).to_string();
        let report = RsaAnalyze::run(&RsaAnalyzeConfig::default(), &n).unwrap();
        assert!(report.contains("close together"), "{report}");
        assert!(report.contains("18446744073709551557") && report.contains("18446744073709551629"));
        assert!(report.contains("d = "));

        let message = b"nohut";
        let cipher = RsaEncrypt::run(&RsaEncryptConfig { n: n.clone(), e: "65537".into() }, message).unwrap();
        assert_eq!(RsaDecrypt::run(&decrypt_config(&n, "", "", "", "65537"), &cipher).unwrap(), message);
    }

    #[test]
    fn a_small_prime_factor_is_found() {
        let large = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let n = (large * BigUint::from(65_537u32)).to_string();
        let report = RsaAnalyze::run(&RsaAnalyzeConfig { e: "3".into() }, &n).unwrap();
        assert!(report.contains("small prime factor") && report.contains("p = 65537"), "{report}");
    }

    #[test]
    fn a_sound_modulus_is_reported_as_not_factored() {
        // RSA-100, factored in 1991 with rather more effort than this.
        let n = "1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139";
        let report = RsaAnalyze::run(&RsaAnalyzeConfig::default(), n).unwrap();
        assert!(report.contains("330 bits") && report.contains("Not factored"), "{report}");
        let error = RsaDecrypt::run(&decrypt_config(n, "", "", "", "65537"), &[1]).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidConfig);
    }

    #[test]
    fn pkcs1_padding_is_removed() {
        // A block built by hand under a modulus large enough to hold it.
        let p = BigUint::parse_bytes(b"18446744073709551629", 10).unwrap();
        let q = BigUint::parse_bytes(b"18446744073709551557", 10).unwrap();
        let n = &p * &q;
        let mut block = vec![2u8];
        block.extend([0xaa; 8]);
        block.push(0);
        block.extend(b"hi");
        let cipher = BigUint::from_bytes_be(&block).modpow(&BigUint::from(65537u32), &n).to_bytes_be();
        let mut config = decrypt_config("", "", &p.to_string(), &q.to_string(), "65537");
        config.padding = RsaPadding::Pkcs1;
        assert_eq!(RsaDecrypt::run(&config, &cipher).unwrap(), b"hi");
        config.padding = RsaPadding::None;
        assert_eq!(RsaDecrypt::run(&config, &cipher).unwrap(), block);
    }

    #[test]
    fn numbers_are_read_in_decimal_or_hex_and_bytes_in_either_order() {
        assert_eq!(parse_number("0xFF"), Some(BigUint::from(255u8)));
        assert_eq!(parse_number("1_000 000"), Some(BigUint::from(1_000_000u32)));
        assert_eq!(parse_number("12a"), None);
        let big = IntegerConfig { endian: Endian::Big };
        let little = IntegerConfig { endian: Endian::Little };
        assert_eq!(BytesToInteger::run(&big, &[1, 0]).unwrap(), "256");
        assert_eq!(BytesToInteger::run(&little, &[1, 0]).unwrap(), "1");
        assert_eq!(IntegerToBytes::run(&big, "0x0102").unwrap(), [1, 2]);
        assert_eq!(IntegerToBytes::run(&little, "258").unwrap(), [2, 1]);
        // A flag as challenges print it: one big decimal number.
        assert_eq!(IntegerToBytes::run(&big, "121424772953121").unwrap(), b"nohut!");
        assert_eq!(BytesToInteger::run(&big, b"nohut!").unwrap(), "121424772953121");
        assert_eq!(IntegerToBytes::run(&big, "not a number").unwrap_err().code, ErrorCode::InvalidInput);
    }

    #[test]
    fn inconsistent_settings_are_config_errors() {
        for config in [
            decrypt_config("", "", "", "", E),
            decrypt_config(N, "", "7", "", E),
            decrypt_config("3234", "", "61", "53", E),
            decrypt_config(N, "", "61", "53", "3"),
            decrypt_config(N, "x", "", "", E),
        ] {
            let error = RsaDecrypt::run(&config, &[1]).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidConfig, "{config:?}");
        }
        let too_big = RsaEncrypt::run(&RsaEncryptConfig { n: N.into(), e: E.into() }, b"too long").unwrap_err();
        assert_eq!(too_big.code, ErrorCode::InvalidInput);
    }

    // A throwaway 512-bit key from `openssl genrsa`, in the four PEM layouts.
    const KEY_N: &str = "9149988259630918411102109321269683555117347049056278823031413073475846765755104319359290911251155363253307202852137032308364204762324481779927525165777881";
    const KEY_D: &str = "8082906149245043112933846789532432972823957307445629280873104177423603043419975086848731095035958234769685003385438240143482146432941289122824224434935041";
    const KEY_P: &str = "102496155640956093585101628041038959264367199804744037377427124884687686203049";
    const KEY_Q: &str = "89271526355420743441080153941914974652021444563374243175657528526597510107569";
    const PEM_PRIVATE: &str = "-----BEGIN RSA PRIVATE KEY-----\nMIIBPAIBAAJBAK60NgKEVnvh/V07TwP719PzDDYPULpR+5XJt/fMLWqI6uuNAEYN\npgrKgnrJX9KzYoWFv8Eugdmk+QlLZA9yQ9kCAwEAAQJBAJpUbBnRGi0ZuB4yFF0q\njsCXRkRDR0cwwRTTO4bVgZFHp+a1GuPfxTint0k/8Q4Fhx/mgU2Mf5F8RHcAfQVF\n1QECIQDimsStQKORK8smb+hhj53V57BHAOSdQTvYArPEydf2qQIhAMVd5LJjR21H\nuJVRtj2+HadoIuJxSc70a/fTcdUs85GxAiEAzE5eJRayYqDpeK5NBilGVSWfeWUE\nugOmwQsuqa6SWIECIQCuxvsjTe+OwpifAoFUI+u2t+1Mge5OqvyBtmlz0h7PwQIg\nMFXyIF8RtUsmUHvpEUUaQ2L0Ywfh2QeTT73gvzlsT88=\n-----END RSA PRIVATE KEY-----\n";
    const PEM_PRIVATE_PKCS8: &str = "-----BEGIN PRIVATE KEY-----\nMIIBVgIBADANBgkqhkiG9w0BAQEFAASCAUAwggE8AgEAAkEArrQ2AoRWe+H9XTtP\nA/vX0/MMNg9QulH7lcm398wtaojq640ARg2mCsqCeslf0rNihYW/wS6B2aT5CUtk\nD3JD2QIDAQABAkEAmlRsGdEaLRm4HjIUXSqOwJdGRENHRzDBFNM7htWBkUen5rUa\n49/FOKe3ST/xDgWHH+aBTYx/kXxEdwB9BUXVAQIhAOKaxK1Ao5EryyZv6GGPndXn\nsEcA5J1BO9gCs8TJ1/apAiEAxV3ksmNHbUe4lVG2Pb4dp2gi4nFJzvRr99Nx1Szz\nkbECIQDMTl4lFrJioOl4rk0GKUZVJZ95ZQS6A6bBCy6prpJYgQIhAK7G+yNN747C\nmJ8CgVQj67a37UyB7k6q/IG2aXPSHs/BAiAwVfIgXxG1SyZQe+kRRRpDYvRjB+HZ\nB5NPveC/OWxPzw==\n-----END PRIVATE KEY-----\n";
    const PEM_PUBLIC: &str = "-----BEGIN PUBLIC KEY-----\nMFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBAK60NgKEVnvh/V07TwP719PzDDYPULpR\n+5XJt/fMLWqI6uuNAEYNpgrKgnrJX9KzYoWFv8Eugdmk+QlLZA9yQ9kCAwEAAQ==\n-----END PUBLIC KEY-----\n";
    const PEM_PUBLIC_PKCS1: &str = "-----BEGIN RSA PUBLIC KEY-----\nMEgCQQCutDYChFZ74f1dO08D+9fT8ww2D1C6UfuVybf3zC1qiOrrjQBGDaYKyoJ6\nyV/Ss2KFhb/BLoHZpPkJS2QPckPZAgMBAAE=\n-----END RSA PUBLIC KEY-----\n";

    #[test]
    fn keys_are_read_from_every_pem_layout() {
        let parse = |pem: &str| RsaParseKey::run(&crate::core::NoConfig {}, pem).unwrap();
        for pem in [PEM_PUBLIC, PEM_PUBLIC_PKCS1] {
            assert_eq!(parse(pem), format!("RSA public key, 512 bits\nn = {KEY_N}\ne = 65537"));
        }
        for pem in [PEM_PRIVATE, PEM_PRIVATE_PKCS8] {
            assert_eq!(
                parse(pem),
                format!("RSA private key, 512 bits\nn = {KEY_N}\ne = 65537\nd = {KEY_D}\np = {KEY_P}\nq = {KEY_Q}")
            );
        }
    }

    #[test]
    fn a_parsed_key_decrypts_what_it_encrypted() {
        let cipher = RsaEncrypt::run(&RsaEncryptConfig { n: KEY_N.into(), e: "65537".into() }, b"nohut").unwrap();
        let config = decrypt_config(KEY_N, KEY_D, "", "", "");
        assert_eq!(RsaDecrypt::run(&config, &cipher).unwrap(), b"nohut");
        let from_primes = decrypt_config("", "", KEY_P, KEY_Q, "65537");
        assert_eq!(RsaDecrypt::run(&from_primes, &cipher).unwrap(), b"nohut");
    }

    #[test]
    fn text_that_is_not_a_key_is_explained() {
        let parse = |text: &str| RsaParseKey::run(&crate::core::NoConfig {}, text).unwrap_err().message;
        assert!(parse("hello there!").contains("not Base64"));
        assert!(parse("-----BEGIN X-----\nAAAA\n-----END X-----").contains("No RSA key"));
        assert!(parse("-----BEGIN ENCRYPTED PRIVATE KEY-----\nAAAA").contains("passphrase"));
    }
}
