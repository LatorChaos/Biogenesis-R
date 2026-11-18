// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-only

//! Bounded legacy genotype XML, separate from placement and filesystem commits
//! Internal DTD parameter entities share bounded declaration and expansion state
//! CESU-8 portions adapt OpenJDK 21u eac07229; retained notices accompany xml_charset_data/NOTICE.txt

#[path = "xml_dtd.rs"]
mod dtd;

use crate::java_numbers::double_text;
use crate::xml_charset;
use crate::{DEFAULT_COLORS, Gene, Genome, GenomeModifiers, GenomeProperties, REACTION_NAMES};
use dtd::{ChildCursor, DocumentIds, Dtd, ExpansionBudget, ModelScratch, RetainedPayloadBudget, xml_space};
use quick_xml::{Reader, encoding::DecodingReader, events::Event};
use std::{collections::BTreeMap, fmt, io::Read};

pub(crate) const MAX_DOCUMENT_BYTES: usize = 16 << 20;
const MAX_GENES: usize = 65_536;
const MAX_DEPTH: usize = 32;
const MAX_ATTRIBUTES: usize = 128;
const MAX_NUMBER_BYTES: usize = 4096;
const MAX_DECLARATION_BYTES: usize = 4096;

const COLORS: [&str; 46] = [
    "green", "forest", "spring", "summer", "lime", "leaf", "c4", "jade", "grass", "bark", "purple",
    "red", "fire", "orange", "maroon", "pink", "cream", "silver", "spike", "lilac", "gray",
    "violet", "olive", "sky", "blue", "ochre", "fallow", "spore", "white", "plague", "coral",
    "mint", "lavender", "magenta", "rose", "cyan", "teal", "yellow", "auburn", "indigo", "blond",
    "flower", "darkgray", "gold", "dark", "eye",
];
const ROOT_DTD_NAMES: [&str; 23] = [
    "symmetry",
    "mutationrate",
    "activity",
    "modifiescream",
    "modifiesfallow",
    "modifiesspore",
    "adaptspore",
    "plague",
    "mirror",
    "disperse",
    "generation",
    "sibling",
    "altruist",
    "familial",
    "social",
    "peaceful",
    "passive",
    "clockwise",
    "mimicall",
    "modifiespink",
    "modifieslilac",
    "modifiessky",
    "selfish",
];

/// Explicit decode, representability, ownership, and resource failures
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XmlError {
    Malformed(String),
    MissingAttribute(String),
    InvalidAttribute(String),
    Encoding(String),
    ExternalResource,
    UnsupportedDtd(&'static str),
    ResourceLimit {
        resource: &'static str,
        maximum: usize,
    },
    InvalidGenome(&'static str),
    UnsupportedColor {
        gene: usize,
        argb: Option<u32>,
    },
    BorrowedGene(usize),
    NullGeneArray,
    NullGene(usize),
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(reason) => write!(f, "Malformed genotype XML: {reason}"),
            Self::MissingAttribute(name) => write!(f, "Missing XML attribute: {name}"),
            Self::InvalidAttribute(name) => write!(f, "Invalid XML attribute: {name}"),
            Self::Encoding(reason) => write!(f, "Invalid or unsupported XML encoding: {reason}"),
            Self::ExternalResource => f.write_str("External XML resources are prohibited"),
            Self::UnsupportedDtd(feature) => write!(f, "Unsupported DTD feature: {feature}"),
            Self::ResourceLimit { resource, maximum } => write!(
                f,
                "XML resource limit exceeded: {resource} (maximum {maximum})"
            ),
            Self::InvalidGenome(reason) => write!(f, "Genome cannot use legacy XML: {reason}"),
            Self::UnsupportedColor { gene, argb } => {
                write!(f, "Gene {gene} has no legacy XML color name: {argb:?}")
            }
            Self::NullGeneArray => f.write_str("Null Gene array has no XML representation"),
            Self::NullGene(index) => write!(f, "Null Gene slot {index} has no XML representation"),
            Self::BorrowedGene(index) => write!(f, "Gene {index} is currently being edited"),
        }
    }
}
impl std::error::Error for XmlError {}
fn malformed(reason: &str) -> XmlError {
    XmlError::Malformed(reason.to_owned())
}
fn limit(resource: &'static str, maximum: usize) -> XmlError {
    XmlError::ResourceLimit { resource, maximum }
}
fn syntax(error: impl fmt::Display) -> XmlError {
    XmlError::Malformed(error.to_string())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum XmlVersion {
    #[default]
    Xml10,
    Xml11,
}

impl XmlVersion {
    fn reference_character(self, ch: char) -> bool {
        dtd::valid_xml_char(ch)
            || (self == Self::Xml11 && matches!(ch as u32, 0x1..=0x1f))
    }

    fn literal_character(self, ch: char) -> bool {
        self.reference_character(ch)
            && !(self == Self::Xml11
                && matches!(ch as u32, 0x1..=0x8 | 0xb..=0xc | 0xe..=0x1f | 0x7f..=0x84 | 0x86..=0x9f))
    }
}

fn declaration(text: &str) -> Result<(XmlVersion, Option<String>), XmlError> {
    let mut reader = Reader::from_str(text);
    match reader.read_event().map_err(syntax)? {
        Event::Decl(declaration) => {
            let content = std::str::from_utf8(declaration.as_ref()).map_err(syntax)?;
            let first_version = declaration.version();
            let xml11 = matches!(&first_version, Ok(value) if value.as_ref() == b"1.1");
            let normalized = if xml11 {
                Some(content.replace(['\u{85}', '\u{2028}'], "\n"))
            } else {
                None
            };
            let content = normalized.as_deref().unwrap_or(content);
            let attributes = quick_xml::events::BytesStart::from_content(content, 3);
            let mut rank = 0;
            let mut encoding = None;
            for attribute in attributes.attributes() {
                let attribute = attribute.map_err(syntax)?;
                let next = match attribute.key.as_ref() {
                    b"version" => 1,
                    b"encoding" => {
                        encoding = Some(std::str::from_utf8(attribute.value.as_ref())
                            .map(str::to_owned)
                            .map_err(|error| XmlError::Encoding(error.to_string()))?);
                        2
                    }
                    b"standalone"
                        if attribute.value.as_ref() == b"yes"
                            || attribute.value.as_ref() == b"no" =>
                    {
                        3
                    }
                    _ => return Err(malformed("Invalid XML declaration attribute")),
                };
                if next <= rank {
                    return Err(malformed("Misordered XML declaration attributes"));
                }
                rank = next;
            }
            let version = match first_version.map_err(syntax)?.as_ref() {
                b"1.0" => XmlVersion::Xml10,
                b"1.1" => XmlVersion::Xml11,
                _ => return Err(XmlError::Encoding("Unsupported XML version".to_owned())),
            };
            Ok((version, encoding))
        }
        _ => Ok((XmlVersion::Xml10, None)),
    }
}

#[derive(Clone, Copy)]
enum Charset {
    Utf8,
    JavaUtf8,
    Utf16,
    Utf16Le,
    Utf16Be,
    JavaUtf16(bool),
    JavaUtf32(bool),
    Ucs2,
    Ucs4,
    Cesu8,
    Latin1,
    Iso88599,
    Windows1252,
    Ascii,
    JavaAscii,
    Legacy(&'static str),
}

fn declared_charset(label: &str) -> Result<Charset, XmlError> {
    let mut name = label.bytes();
    if !name.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        || !name.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(XmlError::Encoding("Invalid XML encoding name".to_owned()));
    }
    // The scanner's native UCS4 switch preserves the original declaration spelling
    if label == "ISO-10646-UCS-4" {
        return Ok(Charset::Ucs4);
    }
    if label == "UTF-16BE" {
        return Ok(Charset::Utf16Be);
    }
    if label == "UTF-16LE" {
        return Ok(Charset::Utf16Le);
    }
    match label.to_ascii_uppercase().as_str() {
        "UTF-8" => return Ok(Charset::Utf8),
        "UTF-16" => return Ok(Charset::Utf16),
        "ISO-10646-UCS-2" => return Ok(Charset::Ucs2),
        _ => {}
    }
    let encoding = xml_charset::java_encoding(label).ok_or_else(|| {
        XmlError::Encoding(
            "Declared charset is outside the supported boundary".to_owned(),
        )
    })?;
    Ok(match encoding.name {
        "UTF-8" => Charset::JavaUtf8,
        "UTF-16" | "UTF-16BE" => Charset::JavaUtf16(true),
        "UTF-16LE" | "x-UTF-16LE-BOM" => Charset::JavaUtf16(false),
        "UTF-32" | "UTF-32BE" | "X-UTF-32BE-BOM" => Charset::JavaUtf32(true),
        "UTF-32LE" | "X-UTF-32LE-BOM" => Charset::JavaUtf32(false),
        "ISO-8859-1" => Charset::Latin1,
        "ISO-8859-9" => Charset::Iso88599,
        "windows-1252" => Charset::Windows1252,
        "CESU-8" => Charset::Cesu8,
        "US-ASCII" if encoding.strict_ascii => Charset::Ascii,
        "US-ASCII" => Charset::JavaAscii,
        name => Charset::Legacy(name),
    })
}

fn decoded_char(output: &mut String, value: char) -> Result<(), XmlError> {
    if output.len() + value.len_utf8() > MAX_DOCUMENT_BYTES {
        return Err(limit("decoded bytes", MAX_DOCUMENT_BYTES));
    }
    output.push(value);
    Ok(())
}

fn decode_java_utf8(bytes: &[u8]) -> Result<String, XmlError> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        if first < 0x80 {
            decoded_char(&mut output, char::from(first))?;
            index += 1;
            continue;
        }
        let length = match first {
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf4 => 4,
            _ => 1,
        };
        if length == 1 {
            decoded_char(&mut output, '\u{fffd}')?;
            index += 1;
            continue;
        }
        let mut consumed = 1;
        while consumed < length && index + consumed < bytes.len() {
            let next = bytes[index + consumed];
            if !(0x80..=0xbf).contains(&next)
                || (consumed == 1
                    && ((first == 0xe0 && next < 0xa0)
                        || (first == 0xf0 && next < 0x90)
                        || (first == 0xf4 && next >= 0x90)))
            {
                break;
            }
            consumed += 1;
        }
        let value = if consumed == length {
            let mut value = u32::from(first & (0x7f >> length));
            for byte in &bytes[index + 1..index + length] {
                value = (value << 6) | u32::from(byte & 0x3f);
            }
            char::from_u32(value).unwrap_or('\u{fffd}')
        } else {
            '\u{fffd}'
        };
        decoded_char(&mut output, value)?;
        index += consumed;
    }
    Ok(output)
}

fn decode_java_utf16(bytes: &[u8], big_endian: bool) -> Result<String, XmlError> {
    let bytes = bytes
        .strip_prefix(&[0xfe, 0xff])
        .or_else(|| bytes.strip_prefix(&[0xff, 0xfe]))
        .unwrap_or(bytes);
    let unit = |index| {
        if big_endian {
            u16::from_be_bytes([bytes[index], bytes[index + 1]])
        } else {
            u16::from_le_bytes([bytes[index], bytes[index + 1]])
        }
    };
    let mut output = String::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        let first = unit(index);
        index += 2;
        let value = if (0xd800..=0xdbff).contains(&first) {
            if index + 1 < bytes.len() {
                let second = unit(index);
                // Java's replacement decoder consumes both units for a malformed surrogate pair
                index += 2;
                if (0xdc00..=0xdfff).contains(&second) {
                    char::from_u32(
                        0x10000 + ((u32::from(first) - 0xd800) << 10) + u32::from(second) - 0xdc00,
                    )
                    .unwrap()
                } else {
                    '\u{fffd}'
                }
            } else {
                '\u{fffd}'
            }
        } else {
            char::from_u32(u32::from(first)).unwrap_or('\u{fffd}')
        };
        decoded_char(&mut output, value)?;
    }
    if index < bytes.len() {
        decoded_char(&mut output, '\u{fffd}')?;
    }
    Ok(output)
}

// CESU-8 portions: Copyright (c) 2011, 2021, Oracle and/or its affiliates
fn decode_cesu8(bytes: &[u8]) -> Result<String, XmlError> {
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let mut output = String::new();
    let mut high = None;
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let length = match first {
            0..=0x7f => 1,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            _ => 0,
        };
        let mut consumed = 1;
        while consumed < length && index + consumed < bytes.len() {
            let next = bytes[index + consumed];
            if !(0x80..=0xbf).contains(&next) || (consumed == 1 && first == 0xe0 && next < 0xa0) {
                break;
            }
            consumed += 1;
        }
        let unit = if length != 0 && consumed == length {
            let mut value = u16::from(first & (0x7f >> length));
            if length == 1 { value = u16::from(first); }
            for byte in &bytes[index + 1..index + length] { value = (value << 6) | u16::from(byte & 0x3f); }
            value
        } else {
            0xfffd
        };
        index += consumed;
        if let Some(previous) = high.take() {
            if !(0xdc00..=0xdfff).contains(&unit) {
                return Err(XmlError::Encoding("CESU-8 contains an XML surrogate scalar".to_owned()));
            }
            decoded_char(&mut output, char::from_u32(0x10000 + ((u32::from(previous) - 0xd800) << 10) + u32::from(unit) - 0xdc00).unwrap())?;
        } else if (0xd800..=0xdbff).contains(&unit) {
            high = Some(unit);
        } else {
            decoded_char(&mut output, char::from_u32(u32::from(unit)).ok_or_else(|| XmlError::Encoding("CESU-8 contains an XML surrogate scalar".to_owned()))?)?;
        }
    }
    if high.is_some() { return Err(XmlError::Encoding("CESU-8 ends with an XML surrogate scalar".to_owned())); }
    Ok(output)
}

fn decode_java_utf32(bytes: &[u8], big_endian: bool) -> Result<String, XmlError> {
    let mut output = String::new();
    let mut chunks = bytes.chunks_exact(4);
    for chunk in &mut chunks {
        let raw = [chunk[0], chunk[1], chunk[2], chunk[3]];
        let scalar = if big_endian {
            u32::from_be_bytes(raw)
        } else {
            u32::from_le_bytes(raw)
        };
        if (0xd800..=0xdfff).contains(&scalar) {
            return Err(XmlError::Encoding(
                "UTF-32 contains an XML surrogate scalar".to_owned(),
            ));
        }
        decoded_char(&mut output, char::from_u32(scalar).unwrap_or('\u{fffd}'))?;
    }
    if !chunks.remainder().is_empty() {
        decoded_char(&mut output, '\u{fffd}')?;
    }
    Ok(output)
}

fn decode_ucs(bytes: &[u8], big_endian: bool, width: usize) -> Result<String, XmlError> {
    let bytes = if width == 2 {
        bytes.strip_prefix(&[0xfe, 0xff]).or_else(|| bytes.strip_prefix(&[0xff, 0xfe])).unwrap_or(bytes)
    } else {
        bytes
    };
    let mut words = bytes.chunks_exact(width);
    let units = words.by_ref().map(|word| {
        if width == 2 {
            if big_endian { u16::from_be_bytes([word[0], word[1]]) } else { u16::from_le_bytes([word[0], word[1]]) }
        } else {
            let value = [word[0], word[1], word[2], word[3]];
            if big_endian { u32::from_be_bytes(value) as u16 } else { u32::from_le_bytes(value) as u16 }
        }
    });
    let mut output = String::new();
    for value in char::decode_utf16(units) {
        let value = value.map_err(|_| XmlError::Encoding("UCS contains an XML surrogate scalar".to_owned()))?;
        decoded_char(&mut output, value)?;
    }
    if !words.remainder().is_empty() {
        return Err(XmlError::Encoding("Incomplete UCS code unit".to_owned()));
    }
    Ok(output)
}

fn decode_single_byte(
    bytes: &[u8],
    charset: Charset,
    at_document_start: bool,
) -> Result<String, XmlError> {
    const WINDOWS_C1: [char; 32] = [
        '\u{20ac}', '\u{fffd}', '\u{201a}', '\u{192}', '\u{201e}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{2c6}', '\u{2030}', '\u{160}', '\u{2039}', '\u{152}', '\u{fffd}', '\u{17d}',
        '\u{fffd}', '\u{fffd}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}',
        '\u{2013}', '\u{2014}', '\u{2dc}', '\u{2122}', '\u{161}', '\u{203a}', '\u{153}',
        '\u{fffd}', '\u{17e}', '\u{178}',
    ];
    if at_document_start && bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(XmlError::Encoding(
            "UTF-8 BOM conflicts with single-byte declaration".to_owned(),
        ));
    }
    let mut output = String::new();
    for byte in bytes {
        let value = match (charset, *byte) {
            (Charset::JavaAscii, value @ 0..=0x7f) => char::from(value),
            (Charset::JavaAscii, _) => '\u{fffd}',
            (Charset::Windows1252, value @ 0x80..=0x9f) => WINDOWS_C1[usize::from(value - 0x80)],
            (Charset::Iso88599, 0xd0) => '\u{11e}',
            (Charset::Iso88599, 0xdd) => '\u{130}',
            (Charset::Iso88599, 0xde) => '\u{15e}',
            (Charset::Iso88599, 0xf0) => '\u{11f}',
            (Charset::Iso88599, 0xfd) => '\u{131}',
            (Charset::Iso88599, 0xfe) => '\u{15f}',
            (Charset::Latin1 | Charset::Windows1252 | Charset::Iso88599, value) => char::from(value),
            _ => unreachable!("Only supported single-byte charsets use this decoder"),
        };
        decoded_char(&mut output, value)?;
    }
    Ok(output)
}

fn source_declaration(
    bytes: &[u8],
    utf16: Option<bool>,
    utf32: Option<bool>,
) -> Result<Option<String>, XmlError> {
    if bytes.starts_with(&[0x4c, 0x6f, 0xa7, 0x94]) {
        let prefix = &bytes[..bytes.len().min(MAX_DECLARATION_BYTES + 1)];
        let header = xml_charset::decode("IBM037", prefix)?;
        let end = header.find("?>").map(|index| index + 2);
        let Some(end) = end else {
            return if bytes.len() > MAX_DECLARATION_BYTES {
                Err(limit("XML declaration bytes", MAX_DECLARATION_BYTES))
            } else {
                Err(malformed("Unclosed XML declaration"))
            };
        };
        if end > MAX_DECLARATION_BYTES {
            return Err(limit("XML declaration bytes", MAX_DECLARATION_BYTES));
        }
        Ok(Some(header[..end].to_owned()))
    } else if let Some(big_endian) = utf16.or(utf32) {
        let bytes = bytes
            .strip_prefix(&[0xfe, 0xff])
            .or_else(|| bytes.strip_prefix(&[0xff, 0xfe]))
            .unwrap_or(bytes);
        let mut header = String::new();
        let width = if utf32.is_some() { 4 } else { 2 };
        for pair in bytes.chunks_exact(width) {
            let unit = if width == 4 {
                let raw = [pair[0], pair[1], pair[2], pair[3]];
                if big_endian {
                    u32::from_be_bytes(raw)
                } else {
                    u32::from_le_bytes(raw)
                }
            } else if big_endian {
                u32::from(u16::from_be_bytes([pair[0], pair[1]]))
            } else {
                u32::from(u16::from_le_bytes([pair[0], pair[1]]))
            };
            if unit > 0x7f {
                if header.len() <= 5 {
                    return Ok(None);
                }
                if !matches!(unit, 0x85 | 0x2028) {
                    return Err(XmlError::Encoding(
                        "XML declaration must contain ASCII characters".to_owned(),
                    ));
                }
            }
            header.push(char::from_u32(unit).unwrap());
            if header.len() == 6 && !xml_space(char::from(unit as u8)) && unit != u32::from(b'?') {
                return Ok(None);
            }
            if header.len() <= 5 && !"<?xml".starts_with(&header) {
                return Ok(None);
            }
            if header.len() > MAX_DECLARATION_BYTES {
                return Err(limit("XML declaration bytes", MAX_DECLARATION_BYTES));
            }
            if header.ends_with("?>") {
                if !header.is_ascii()
                    && !matches!(declaration(&header), Ok((XmlVersion::Xml11, _)))
                {
                    return Err(XmlError::Encoding(
                        "XML declaration must contain ASCII characters".to_owned(),
                    ));
                }
                return Ok(Some(header));
            }
        }
        if header.starts_with("<?xml") {
            Err(malformed("Unclosed XML declaration"))
        } else {
            Ok(None)
        }
    } else {
        let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
        if !bytes.starts_with(b"<?xml")
            || bytes
                .get(5)
                .is_some_and(|byte| !xml_space(char::from(*byte)) && *byte != b'?')
        {
            return Ok(None);
        }
        let prefix = &bytes[..bytes.len().min(MAX_DECLARATION_BYTES + 1)];
        let end = prefix
            .windows(2)
            .position(|part| part == b"?>")
            .map(|at| at + 2);
        let Some(end) = end else {
            return if bytes.len() > MAX_DECLARATION_BYTES {
                Err(limit("XML declaration bytes", MAX_DECLARATION_BYTES))
            } else {
                Err(malformed("Unclosed XML declaration"))
            };
        };
        if end > MAX_DECLARATION_BYTES {
            return Err(limit("XML declaration bytes", MAX_DECLARATION_BYTES));
        }
        std::str::from_utf8(&bytes[..end])
            .map(|text| Some(text.to_owned()))
            .map_err(|error| XmlError::Encoding(error.to_string()))
    }
}

fn decode_document(bytes: &[u8]) -> Result<(String, XmlVersion), XmlError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(limit("source bytes", MAX_DOCUMENT_BYTES));
    }
    if bytes.starts_with(&[0, 0, 0xfe, 0xff]) || bytes.starts_with(&[0xff, 0xfe, 0, 0]) {
        return Err(XmlError::Encoding(
            "UTF-32 BOM input is outside the proven legacy subset".to_owned(),
        ));
    }
    let utf32 = if bytes.starts_with(&[0, 0, 0, b'<']) {
        Some(true)
    } else if bytes.starts_with(&[b'<', 0, 0, 0]) {
        Some(false)
    } else {
        None
    };
    let utf16 = if bytes.starts_with(&[0xfe, 0xff]) || bytes.starts_with(&[0, b'<', 0, b'?']) {
        Some(true)
    } else if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[b'<', 0, b'?', 0]) {
        Some(false)
    } else {
        None
    };
    let source_header = source_declaration(bytes, utf16, utf32)?;
    let charset = source_header
        .as_deref()
        .map(declaration)
        .transpose()?
        .and_then(|(_, encoding)| encoding)
        .as_deref()
        .map(declared_charset)
        .transpose()?;
    let matches_source = match charset {
        Some(Charset::Utf16) => utf16.is_some(),
        Some(Charset::Utf16Be) => utf16 == Some(true),
        Some(Charset::Utf16Le) => utf16 == Some(false),
        Some(Charset::JavaUtf16(big_endian)) => utf16 == Some(big_endian),
        Some(Charset::JavaUtf32(big_endian)) => utf32 == Some(big_endian),
        Some(Charset::Ucs2) => utf16.is_some(),
        Some(Charset::Ucs4) => utf32.is_some(),
        Some(_) => utf16.is_none() && utf32.is_none(),
        None => utf32.is_none(),
    };
    if !matches_source {
        return Err(XmlError::Encoding(
            "Declared charset conflicts with source bytes".to_owned(),
        ));
    }
    let legacy_reader = matches!(charset, Some(Charset::Ascii | Charset::JavaAscii | Charset::Latin1 | Charset::Iso88599 | Charset::Windows1252 | Charset::Legacy(_)));
    // Xerces consumes a UTF8 BOM before replacing the reader from the XML declaration
    let bytes = if legacy_reader {
        bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)
    } else {
        bytes
    };
    // Keep the UTF8 declaration consumed before Xerces switches to the declared reader
    let header = source_header.as_deref().filter(|header| {
        legacy_reader && !header.is_ascii() && bytes.starts_with(b"<?xml")
    });
    let bytes = header.map_or(bytes, |header| &bytes[header.len()..]);
    let mut decoded = match charset {
        Some(Charset::Ascii) => {
            if !bytes.is_ascii() {
                return Err(XmlError::Encoding(
                    "US-ASCII source contains non-ASCII bytes".to_owned(),
                ));
            }
            std::str::from_utf8(bytes).unwrap().to_owned()
        }
        Some(charset @ (Charset::Latin1 | Charset::Windows1252 | Charset::Iso88599 | Charset::JavaAscii)) => {
            decode_single_byte(bytes, charset, header.is_none())?
        }
        Some(Charset::JavaUtf8) => decode_java_utf8(bytes)?,
        Some(Charset::JavaUtf16(big_endian)) => decode_java_utf16(bytes, big_endian)?,
        Some(Charset::JavaUtf32(big_endian)) => decode_java_utf32(bytes, big_endian)?,
        Some(Charset::Ucs2) => decode_ucs(bytes, utf16.unwrap(), 2)?,
        Some(Charset::Ucs4) => decode_ucs(bytes, utf32.unwrap(), 4)?,
        Some(Charset::Cesu8) => decode_cesu8(bytes)?,
        Some(Charset::Legacy(name)) => xml_charset::decode(name, bytes)?,
        _ => {
            let mut decoder = DecodingReader::new(bytes);
            let mut text = String::new();
            (&mut decoder)
                .take((MAX_DOCUMENT_BYTES + 1) as u64)
                .read_to_string(&mut text)
                .map_err(|error| XmlError::Encoding(error.to_string()))?;
            if text.len() > MAX_DOCUMENT_BYTES {
                return Err(limit("decoded bytes", MAX_DOCUMENT_BYTES));
            }
            text
        }
    };
    if let Some(header) = header {
        if decoded.len() + header.len() > MAX_DOCUMENT_BYTES {
            return Err(limit("decoded bytes", MAX_DOCUMENT_BYTES));
        }
        decoded.insert_str(0, header);
    }
    let (version, _) = declaration(&decoded)?;
    let mut normalized = String::with_capacity(decoded.len());
    let mut characters = decoded.chars().peekable();
    while let Some(ch) = characters.next() {
        if !version.literal_character(ch) {
            return Err(malformed(match version {
                XmlVersion::Xml10 => "Forbidden XML 1.0 character",
                XmlVersion::Xml11 => "Forbidden XML 1.1 literal character",
            }));
        }
        match ch {
            '\r' => {
                if characters.peek().is_some_and(|next| {
                    *next == '\n' || (version == XmlVersion::Xml11 && *next == '\u{85}')
                }) {
                    characters.next();
                }
                normalized.push('\n');
            }
            '\u{85}' | '\u{2028}' if version == XmlVersion::Xml11 => normalized.push('\n'),
            _ => normalized.push(ch),
        }
    }
    Ok((normalized, version))
}

struct Node {
    name: String,
    cursor: Option<ChildCursor>,
    children: usize,
    text: bool,
    nonspace: bool,
}

/// Decode bounded XML bytes into fresh genes and default lineage, without filesystem or simulation access
/// Decode observed native and legacy charset policies with bounded output
/// Resolve declarations through qualified Java policies without WHATWG label fallback
/// Preserve native strictness, replacement behavior, and observed source transport conflicts
pub fn parse_genome_xml(bytes: &[u8]) -> Result<Genome, XmlError> {
    parse_genome_xml_with(bytes, |_, _| {}, |_| {}, || {})
}

#[cfg(test)]
pub(crate) fn parse_genome_xml_observed(
    bytes: &[u8],
    elements: &mut Vec<(String, BTreeMap<String, String>)>,
    complete: &mut bool,
) -> Result<Genome, XmlError> {
    parse_genome_xml_with(bytes, |name, attributes| elements.push((name.to_owned(), attributes.clone())), |_| {}, || *complete = true)
}

#[cfg(test)]
pub(crate) fn parse_genome_xml_observed_text(
    bytes: &[u8],
    elements: &mut Vec<(String, BTreeMap<String, String>)>,
    text: &mut String,
    complete: &mut bool,
) -> Result<Genome, XmlError> {
    parse_genome_xml_with(
        bytes,
        |name, attributes| elements.push((name.to_owned(), attributes.clone())),
        |value| text.push_str(value),
        || *complete = true,
    )
}

struct ValidationState<V, T> {
    version: XmlVersion,
    budget: ExpansionBudget,
    retained: RetainedPayloadBudget,
    identities: DocumentIds,
    stack: Vec<Node>,
    model_scratch: ModelScratch,
    direct: Vec<(String, BTreeMap<String, String>)>,
    root: Option<(String, BTreeMap<String, String>)>,
    closed: bool,
    declaration_seen: bool,
    prior_content: bool,
    visit: V,
    visit_text: T,
}

impl<V: FnMut(&str, &BTreeMap<String, String>), T: FnMut(&str)> ValidationState<V, T> {
    fn new(version: XmlVersion, visit: V, visit_text: T) -> Self {
        Self {
            version,
            budget: ExpansionBudget::default(),
            retained: RetainedPayloadBudget::default(),
            identities: DocumentIds::default(),
            stack: Vec::new(),
            model_scratch: ModelScratch::default(),
            direct: Vec::new(),
            root: None,
            closed: false,
            declaration_seen: false,
            prior_content: false,
            visit,
            visit_text,
        }
    }

    fn replacement<'a>(
        &mut self,
        value: &str,
        grammar: &'a Dtd,
        trail: &mut Vec<&'a str>,
    ) -> Result<(), XmlError> {
        let boundary = self.stack.len();
        // A replacement BOM is character data, not a new transport signature
        let body = value.trim_start_matches('\u{feff}');
        if body.len() != value.len() {
            self.budget.generated(value.len() - body.len(), trail.len() + 1)?;
            (self.visit_text)(&value[..value.len() - body.len()]);
            let node = self.stack.last_mut().unwrap();
            node.text = true;
            node.nonspace = true;
        }
        let mut reader = Reader::from_str(body);
        reader.config_mut().check_comments = true;
        loop {
            let before = reader.buffer_position() as usize;
            let event = reader.read_event().map_err(syntax)?;
            if matches!(event, Event::Eof) {
                return if self.stack.len() == boundary { Ok(()) }
                    else { Err(malformed("Incomplete entity replacement structure")) };
            }
            let span = reader.buffer_position() as usize - before;
            self.event(event, &reader, Some(grammar), trail, Some(boundary), span)?;
        }
    }

    fn event<'a>(
        &mut self,
        event: Event<'_>,
        reader: &Reader<&[u8]>,
        grammar: Option<&'a Dtd>,
        trail: &mut Vec<&'a str>,
        boundary: Option<usize>,
        span: usize,
    ) -> Result<(), XmlError> {
        // References charge emitted characters, not their lexical spelling
        if !trail.is_empty() && !matches!(event, Event::Text(_) | Event::GeneralRef(_)) {
            self.budget.generated(span, trail.len() + 1)?;
        }
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                if self.closed {
                    return Err(malformed("Multiple document roots"));
                }
                if self.stack.len() >= MAX_DEPTH {
                    return Err(limit("element depth", MAX_DEPTH));
                }
                let grammar = grammar
                    .ok_or_else(|| malformed("A validating DOCTYPE is required"))?;
                let name = std::str::from_utf8(element.name().as_ref())
                    .map_err(syntax)?
                    .to_owned();
                if !dtd::valid_name(&name, self.version) {
                    return Err(malformed("Invalid XML element name"));
                }
                let mut raw = BTreeMap::new();
                for attribute in element.attributes() {
                    if raw.len() >= MAX_ATTRIBUTES {
                        return Err(limit("attributes per element", MAX_ATTRIBUTES));
                    }
                    let attribute = attribute.map_err(syntax)?;
                    let key = std::str::from_utf8(attribute.key.as_ref())
                        .map_err(syntax)?
                        .to_owned();
                    if !dtd::valid_name(&key, self.version) {
                        return Err(malformed("Invalid XML attribute name"));
                    }
                    let value = reader
                        .decoder()
                        .decode(attribute.value.as_ref())
                        .map_err(syntax)?
                        .into_owned();
                    if raw.insert(key, value).is_some() {
                        return Err(malformed("Duplicate XML attribute"));
                    }
                }
                let attributes = if trail.is_empty() {
                    grammar.attributes(&name, raw, &mut self.budget)?
                } else {
                    grammar.attributes_with_entities(&name, raw, &mut self.budget, trail)?
                };
                grammar.identities(&name, &attributes, &mut self.identities, &mut self.retained)?;
                (self.visit)(&name, &attributes);
                if let Some(parent) = self.stack.last_mut() {
                    grammar.child(&parent.name, &name, &mut parent.cursor, &mut self.model_scratch)?;
                    parent.children += 1;
                    if self.stack.len() == 1 {
                        if self.direct.len() >= MAX_GENES {
                            return Err(limit("genes", MAX_GENES));
                        }
                        retain_attributes(&name, &attributes, &mut self.retained)?;
                        self.direct.push((name.clone(), attributes));
                    }
                } else {
                    if name != grammar.root() {
                        return Err(malformed("Document root violates its DOCTYPE"));
                    }
                    retain_attributes(&name, &attributes, &mut self.retained)?;
                    self.root = Some((name.clone(), attributes));
                }
                let cursor = grammar.cursor(&name, &mut self.model_scratch)?;
                if empty {
                    grammar.finish(&name, 0, false, false, cursor.as_ref())?;
                    if self.stack.is_empty() {
                        self.closed = true;
                    }
                } else {
                    self.stack.push(Node {
                        name,
                        cursor,
                        children: 0,
                        text: false,
                        nonspace: false,
                    });
                }
                self.prior_content = true;
            }
            Event::End(element) => {
                if boundary.is_some_and(|depth| self.stack.len() <= depth) {
                    return Err(malformed("Closing element crosses an entity boundary"));
                }
                let node = self.stack
                    .pop()
                    .ok_or_else(|| malformed("Unmatched closing element"))?;
                if node.name.as_bytes() != element.name().as_ref() {
                    return Err(malformed("Mismatched closing element"));
                }
                grammar.unwrap().finish(
                    &node.name,
                    node.children,
                    node.text,
                    node.nonspace,
                    node.cursor.as_ref(),
                )?;
                if self.stack.is_empty() {
                    self.closed = true;
                }
            }
            Event::Text(value) => {
                let text = value.decode().map_err(syntax)?;
                if text.contains("]]>") {
                    return Err(malformed("CDATA terminator in ordinary text"));
                }
                if let Some(node) = self.stack.last_mut() {
                    let grammar = grammar.unwrap();
                    let expanded = grammar.text(&text, &mut self.budget)?;
                    if !trail.is_empty() {
                        self.budget.generated(expanded.len(), trail.len() + 1)?;
                    }
                    if !grammar.ignorable_whitespace(&node.name, &expanded) {
                        (self.visit_text)(&expanded);
                    }
                    node.text |= !expanded.is_empty();
                    node.nonspace |= expanded.chars().any(|ch| !xml_space(ch));
                } else if text.chars().any(|ch| !xml_space(ch)) {
                    return Err(malformed("Text outside document element"));
                }
                if !text.is_empty() {
                    self.prior_content = true;
                }
            }
            Event::CData(value) => {
                let node = self.stack
                    .last_mut()
                    .ok_or_else(|| malformed("CDATA outside document element"))?;
                let text = value.decode().map_err(syntax)?;
                (self.visit_text)(&text);
                node.text |= !text.is_empty();
                node.nonspace |= text.chars().any(|ch| !xml_space(ch));
            }
            Event::GeneralRef(value) => {
                if self.stack.is_empty() {
                    return Err(malformed("Entity outside document element"));
                }
                let name = value.decode().map_err(syntax)?;
                let grammar = grammar.unwrap();
                match grammar.content_reference(&name, &mut self.budget)? {
                    dtd::ContentReference::Character(value) => {
                        self.budget.generated(value.len_utf8(), trail.len() + 1)?;
                        (self.visit_text)(value.encode_utf8(&mut [0; 4]));
                        let node = self.stack.last_mut().unwrap();
                        node.text = true;
                        node.nonspace |= !xml_space(value);
                    }
                    dtd::ContentReference::Replacement { name, value } => {
                        dtd::enter_entity(name, trail)?;
                        let result = self.replacement(value, grammar, trail);
                        trail.pop();
                        result?;
                    }
                }
            }
            Event::PI(value) => {
                let target = std::str::from_utf8(value.target()).map_err(syntax)?;
                if !dtd::valid_name(target, self.version) {
                    return Err(malformed("Invalid processing instruction target"));
                }
                if target.eq_ignore_ascii_case("xml") {
                    return Err(malformed("Reserved processing instruction target"));
                }
                self.prior_content = true;
            }
            Event::Comment(_) => self.prior_content = true,
            Event::Decl(_) | Event::DocType(_) => {
                return Err(malformed("Declaration inside entity replacement"));
            }
            Event::Eof => unreachable!("EOF belongs to the current reader boundary"),
        }
        Ok(())
    }
}

fn parse_genome_xml_with(
    bytes: &[u8],
    visit: impl FnMut(&str, &BTreeMap<String, String>),
    visit_text: impl FnMut(&str),
    mut complete: impl FnMut(),
) -> Result<Genome, XmlError> {
    let (document, version) = decode_document(bytes)?;
    let mut reader = Reader::from_str(&document);
    reader.config_mut().check_comments = true;
    let mut dtd = None;
    let mut state = ValidationState::new(version, visit, visit_text);
    loop {
        let before = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(syntax)?;
        let span = reader.buffer_position() as usize - before;
        match event {
            Event::Decl(_) => {
                if state.declaration_seen || state.prior_content || state.root.is_some() {
                    return Err(malformed("Misplaced XML declaration"));
                }
                state.declaration_seen = true;
            }
            Event::DocType(value) => {
                if dtd.is_some() || state.root.is_some() || state.closed {
                    return Err(malformed("Misplaced or repeated DOCTYPE"));
                }
                let raw = &document.as_bytes()[before..reader.buffer_position() as usize];
                if !raw.starts_with(b"<!DOCTYPE") {
                    return Err(malformed("Invalid DOCTYPE spelling"));
                }
                dtd = Some(Dtd::parse_bounded(version, &value.decode().map_err(syntax)?, &mut state.budget, &mut state.retained)?);
                state.prior_content = true;
            }
            Event::Eof => break,
            event => state.event(event, &reader, dtd.as_ref(), &mut Vec::new(), None, span)?,
        }
    }
    if !state.closed || !state.stack.is_empty() {
        return Err(malformed("Missing or incomplete document root"));
    }
    state.identities.finish()?;
    complete();
    let (name, attributes) = state.root.ok_or_else(|| malformed("Missing genome"))?;
    if name != "genetic_code" {
        return Err(malformed("Expected genetic_code document root"));
    }
    let properties = parse_properties(&attributes)?;
    drop(attributes);
    let mut genes = Vec::with_capacity(state.direct.len());
    for (name, attributes) in state.direct {
        if name != "gene" {
            return Err(malformed("Expected a gene child"));
        }
        genes.push(parse_gene(&attributes)?.shared());
    }
    Ok(Genome::from_parts(&genes, properties))
}

fn retain_attributes(
    name: &str,
    attributes: &BTreeMap<String, String>,
    budget: &mut RetainedPayloadBudget,
) -> Result<(), XmlError> {
    budget.charge(name.len())?;
    for (name, value) in attributes {
        budget.charge(name.len())?;
        budget.charge(value.len())?;
    }
    Ok(())
}

fn required<'a>(values: &'a BTreeMap<String, String>, name: &str) -> Result<&'a str, XmlError> {
    values
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| XmlError::MissingAttribute(name.to_owned()))
}
fn integer(values: &BTreeMap<String, String>, name: &str) -> Result<i32, XmlError> {
    let value = required(values, name)?;
    if value.len() > MAX_NUMBER_BYTES {
        return Err(limit("numeric attribute bytes", MAX_NUMBER_BYTES));
    }
    java_int(value).ok_or_else(|| XmlError::InvalidAttribute(name.to_owned()))
}
fn range(
    values: &BTreeMap<String, String>,
    name: &str,
    minimum: i32,
    maximum: i32,
) -> Result<i32, XmlError> {
    let value = integer(values, name)?;
    if (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(XmlError::InvalidAttribute(name.to_owned()))
    }
}
fn flag(values: &BTreeMap<String, String>, name: &str) -> Result<bool, XmlError> {
    match required(values, name)? {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(XmlError::InvalidAttribute(name.to_owned())),
    }
}
fn parse_properties(values: &BTreeMap<String, String>) -> Result<GenomeProperties, XmlError> {
    Ok(GenomeProperties {
        symmetry: range(values, "symmetry", 1, 8)?,
        mirror: i32::from(flag(values, "mirror")?),
        mutation_rate: range(values, "mutationrate", 0, 10000)?,
        activity: range(values, "activity", 0, 2)?,
        modifiers: GenomeModifiers {
            cream: range(values, "modifiescream", 1, 3)?,
            fallow: range(values, "modifiesfallow", 1, 4)?,
            spore: range(values, "modifiesspore", 1, 12)?,
            adapt_spore: range(values, "adaptspore", 1, 70)?,
            plague: flag(values, "plague")?,
            disperse_children: flag(values, "disperse")?,
            generation_battle: flag(values, "generation")?,
            sibling_battle: flag(values, "sibling")?,
            altruist: flag(values, "altruist")?,
            familial: flag(values, "familial")?,
            social: flag(values, "social")?,
            peaceful: flag(values, "peaceful")?,
            passive: flag(values, "passive")?,
            clockwise: flag(values, "clockwise")?,
            mimic_all: flag(values, "mimicall")?,
            modifies_pink: flag(values, "modifiespink")?,
            modifies_lilac: flag(values, "modifieslilac")?,
            modifies_sky: flag(values, "modifiessky")?,
            selfish: flag(values, "selfish")?,
        },
    })
}
fn parse_gene(values: &BTreeMap<String, String>) -> Result<Gene, XmlError> {
    for name in ["length", "theta"] {
        if required(values, name)?.len() > MAX_NUMBER_BYTES {
            return Err(limit("numeric attribute bytes", MAX_NUMBER_BYTES));
        }
    }
    let length = java_double(required(values, "length")?)
        .ok_or_else(|| XmlError::InvalidAttribute("length".to_owned()))?;
    let theta = java_double(required(values, "theta")?)
        .ok_or_else(|| XmlError::InvalidAttribute("theta".to_owned()))?;
    let color = required(values, "color")?;
    if !COLORS.contains(&color) {
        return Err(XmlError::InvalidAttribute("color".to_owned()));
    }
    let argb = DEFAULT_COLORS
        .iter()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(color))
        .map(|(_, rgb, _)| 0xff00_0000 | rgb)
        .unwrap();
    let mut reactions = [0; 36];
    for (slot, name) in reactions.iter_mut().zip(REACTION_NAMES) {
        *slot = integer(values, &format!("{name}reaction"))?;
    }
    Ok(Gene::new_with_color(
        length,
        theta,
        Some(crate::source_palette_color(crate::ColorKind::from_argb(argb) as usize)
            .expect("Validated named XML color")),
        integer(values, "branch")?,
        reactions,
    ))
}

// XML retains its document-specific numeric bound
fn java_int(text: &str) -> Option<i32> {
    if text.len() > MAX_NUMBER_BYTES {
        return None;
    }
    crate::java_numbers::java_int(text)
}

fn java_double(text: &str) -> Option<f64> {
    if text.len() > MAX_NUMBER_BYTES {
        return None;
    }
    crate::java_numbers::java_double(text)
}

fn property_values(genome: &Genome) -> Vec<(&'static str, String)> {
    let m = genome.modifiers();
    let yes = |value| {
        if value {
            "yes".to_owned()
        } else {
            "no".to_owned()
        }
    };
    vec![
        ("symmetry", genome.symmetry().to_string()),
        ("mutationrate", genome.mutation_rate().to_string()),
        ("activity", genome.activity().to_string()),
        ("modifiescream", m.cream.to_string()),
        ("modifiesfallow", m.fallow.to_string()),
        ("modifiesspore", m.spore.to_string()),
        ("adaptspore", m.adapt_spore.to_string()),
        ("mirror", yes(genome.mirror() != 0)),
        ("plague", yes(m.plague)),
        ("disperse", yes(m.disperse_children)),
        ("generation", yes(m.generation_battle)),
        ("sibling", yes(m.sibling_battle)),
        ("altruist", yes(m.altruist)),
        ("familial", yes(m.familial)),
        ("social", yes(m.social)),
        ("peaceful", yes(m.peaceful)),
        ("passive", yes(m.passive)),
        ("clockwise", yes(m.clockwise)),
        ("mimicall", yes(m.mimic_all)),
        ("modifiespink", yes(m.modifies_pink)),
        ("modifieslilac", yes(m.modifies_lilac)),
        ("modifiessky", yes(m.modifies_sky)),
        ("selfish", yes(m.selfish)),
    ]
}

fn push(output: &mut String, text: &str) -> Result<(), XmlError> {
    if output.len().saturating_add(text.len()) > MAX_DOCUMENT_BYTES {
        return Err(limit("output bytes", MAX_DOCUMENT_BYTES));
    }
    output.push_str(text);
    Ok(())
}
fn canonical_dtd() -> String {
    let mut output = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE genetic_code [\n<!ELEMENT genetic_code (gene+)>\n".to_owned();
    for name in ROOT_DTD_NAMES {
        let enumeration = match name {
            "symmetry" => Some((1, 8)),
            "activity" => Some((0, 2)),
            "modifiescream" => Some((1, 3)),
            "modifiesfallow" => Some((1, 4)),
            "modifiesspore" => Some((1, 12)),
            "adaptspore" => Some((1, 70)),
            _ => None,
        };
        let kind = if let Some((start, end)) = enumeration {
            format!(
                "({})",
                (start..=end)
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join("|")
            )
        } else if name == "mutationrate" {
            "CDATA".to_owned()
        } else {
            "(yes|no)".to_owned()
        };
        output.push_str(&format!(
            "<!ATTLIST genetic_code {name} {kind} #REQUIRED>\n"
        ));
    }
    output.push_str("<!ELEMENT gene EMPTY>\n");
    for name in ["length", "theta", "color", "branch"] {
        let kind = if name == "color" {
            format!("({})", COLORS.join("|"))
        } else {
            "CDATA".to_owned()
        };
        output.push_str(&format!("<!ATTLIST gene {name} {kind} #REQUIRED>\n"));
    }
    for name in REACTION_NAMES {
        output.push_str(&format!("<!ATTLIST gene {name}reaction CDATA #REQUIRED>\n"));
    }
    output.push_str("]>\n");
    output
}

/// Preflight representability and return complete UTF-8 bytes; no file is opened or modified
/// Numeric text preserves Java parser round trips without promising Java Double.toString byte identity
pub fn encode_genome_xml(genome: &Genome) -> Result<Vec<u8>, XmlError> {
    let properties = property_values(genome);
    parse_properties(
        &properties
            .iter()
            .map(|(name, value)| (name.to_string(), value.clone()))
            .collect(),
    )?;
    let count = genome.gene_count().map_err(|_| XmlError::NullGeneArray)?;
    if count == 0 {
        return Err(XmlError::InvalidGenome(
            "Canonical gene+ XML requires at least one gene",
        ));
    }
    if count > MAX_GENES {
        return Err(limit("genes", MAX_GENES));
    }
    let mut genes = Vec::with_capacity(count);
    for index in 0..count {
        let reference = genome.gene_ref(index).map_err(|_| XmlError::NullGene(index))?;
        let gene = reference
            .try_borrow()
            .map_err(|_| XmlError::BorrowedGene(index))?;
        let argb = gene.color();
        let color = DEFAULT_COLORS
            .iter()
            .find(|(_, rgb, _)| Some(0xff00_0000 | rgb) == argb)
            .map(|(name, _, _)| name.to_ascii_lowercase())
            .ok_or(XmlError::UnsupportedColor { gene: index, argb })?;
        genes.push((
            gene.length(),
            gene.theta(),
            color,
            gene.branch(),
            *gene.reactions(),
        ));
    }
    let mut output = canonical_dtd();
    push(&mut output, "<genetic_code")?;
    for (name, value) in properties {
        push(&mut output, &format!(" {name}=\"{value}\""))?;
    }
    push(&mut output, ">\n")?;
    for (length, theta, color, branch, reactions) in genes {
        let mut line = format!(
            "\t<gene length=\"{}\" theta=\"{}\" color=\"{color}\" branch=\"{branch}\"",
            double_text(length),
            double_text(theta)
        );
        for (name, value) in REACTION_NAMES.into_iter().zip(reactions) {
            line.push_str(&format!(" {name}reaction=\"{value}\""));
        }
        line.push_str(" />\n");
        push(&mut output, &line)?;
    }
    push(&mut output, "</genetic_code>\n")?;
    Ok(output.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    fn genome() -> Genome {
        let mut properties = GenomeProperties {
            symmetry: 1,
            mirror: -7,
            mutation_rate: 10000,
            ..Default::default()
        };
        properties.modifiers.cream = 1;
        properties.modifiers.fallow = 2;
        properties.modifiers.spore = 12;
        properties.modifiers.adapt_spore = 70;
        let gene = Gene::new(
            -0.0,
            f64::INFINITY,
            Some(0xff00_ff00),
            i32::MIN,
            std::array::from_fn(|index| i32::MIN + index as i32),
        )
        .shared();
        let mut genome = Genome::from_parts(&[Rc::clone(&gene), gene], properties);
        genome.set_generation(91);
        genome.set_clade_id(Some("xml".to_owned()));
        genome
    }

    #[test]
    fn roundtrip_preserves_scalars_but_creates_fresh_genes_and_default_lineage() {
        let original = genome();
        let bytes = encode_genome_xml(&original).unwrap();
        let parsed = parse_genome_xml(&bytes).unwrap();
        assert_eq!(parsed.mirror(), 1);
        assert_eq!(parsed.generation(), 0);
        assert_eq!(parsed.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), None);
        assert!(!parsed.update_clade());
        assert!(!Rc::ptr_eq(&parsed.genes().unwrap()[0], &parsed.genes().unwrap()[1]));
        assert_eq!(parsed.gene_ref(0).unwrap().borrow().length().to_bits(), (-0.0f64).to_bits());
        assert_eq!(parsed.gene_ref(0).unwrap().borrow().theta(), f64::INFINITY);
        assert_eq!(parsed.gene_ref(0).unwrap().borrow().branch(), i32::MIN);
        assert_eq!(parsed.gene_ref(0).unwrap().borrow().reactions(), original.gene_ref(0).unwrap().borrow().reactions());
        assert!(Rc::ptr_eq(&original.genes().unwrap()[0], &original.genes().unwrap()[1]));
        assert_eq!(original.mirror(), -7);
    }

    #[test]
    fn java_lexical_rules_and_hexadecimal_rounding_keep_exact_bits() {
        for (text, bits) in [
            ("0x1.8p2", 6.0f64.to_bits()),
            ("0x1p-1074", 1),
            ("0x1p-1075", 0),
            ("-0x0p9999", 1 << 63),
            ("0x1.00000000000008p0", 1.0f64.to_bits()),
            ("0x1.00000000000018p0", 1.0f64.to_bits() + 2),
            ("0x1.fffffffffffff8p1023", f64::INFINITY.to_bits()),
        ] {
            assert_eq!(java_double(text).unwrap().to_bits(), bits, "{text}");
        }
        assert_eq!(java_double(" 1.25d "), Some(1.25));
        assert_eq!(java_double("1e309"), Some(f64::INFINITY));
        assert_eq!(
            java_double("-1e-400").unwrap().to_bits(),
            (-0.0f64).to_bits()
        );
        for text in ["nan", "inf", "Infinityf", "1,5", "0x1", "1.0dd", "+-1"] {
            assert!(java_double(text).is_none(), "{text}");
        }
        assert_eq!(java_int("-2147483648"), Some(i32::MIN));
        assert_eq!(java_int("+2147483647"), Some(i32::MAX));
        assert_eq!(java_int("\u{0663}\u{0664}"), Some(34));
        for text in ["2147483648", " 7 ", "1.0", "0x2", "\u{1d7d8}"] {
            assert!(java_int(text).is_none(), "{text}");
        }
    }

    #[test]
    fn writer_rejects_nonrepresentable_data_before_returning_any_document() {
        for color in [None, Some(0x12ff_00ff), crate::ColorKind::Brown.argb()] {
            let original = genome();
            original.gene_ref(1).unwrap().borrow_mut().set_color(color);
            assert!(matches!(
                encode_genome_xml(&original),
                Err(XmlError::UnsupportedColor { .. })
            ));
            assert_eq!(original.gene_ref(0).unwrap().borrow().color(), color);
        }
        let mut original = genome();
        original.symmetry = 9;
        assert!(matches!(
            encode_genome_xml(&original),
            Err(XmlError::InvalidAttribute(_))
        ));
        let original = genome();
        let _held_handle = original.gene_ref(0).unwrap();
        let _held = _held_handle.borrow_mut();
        assert!(matches!(
            encode_genome_xml(&original),
            Err(XmlError::BorrowedGene(0))
        ));
    }

    #[test]
    fn valid_large_genotypes_do_not_inherit_the_editor_gene_limit() {
        let original = genome();
        let many = Genome::from_parts(
            &vec![Rc::clone(&original.genes().unwrap()[0]); 129],
            original.properties(),
        );
        assert_eq!(
            parse_genome_xml(&encode_genome_xml(&many).unwrap())
                .unwrap()
                .genes().unwrap()
                .len(),
            129
        );
    }

    #[test]
    fn internal_defaults_and_gene_star_are_real_validation_rules() {
        let bytes = encode_genome_xml(&genome()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let default = text
            .replace("mutationrate CDATA #REQUIRED", "mutationrate CDATA \"321\"")
            .replace(" mutationrate=\"10000\"", "");
        assert_eq!(
            parse_genome_xml(default.as_bytes())
                .unwrap()
                .mutation_rate(),
            321
        );
        let empty = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("<gene "))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_genome_xml(empty.as_bytes()).is_err());
        let parsed = parse_genome_xml(empty.replace("(gene+)", "(gene*)").as_bytes()).unwrap();
        assert!(parsed.genes().unwrap().is_empty());
        assert!(matches!(
            encode_genome_xml(&parsed),
            Err(XmlError::InvalidGenome(_))
        ));
    }

    #[test]
    fn declaration_duplicates_and_numeric_limits_have_explicit_failures() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let duplicate = text.replacen(
            "encoding=\"UTF-8\"",
            "encoding=\"UTF-8\" encoding=\"UTF-8\"",
            1,
        );
        assert!(parse_genome_xml(duplicate.as_bytes()).is_err());
        let unknown = text.replacen("version=\"1.0\"", "version=\"1.0\" other=\"yes\"", 1);
        assert!(parse_genome_xml(unknown.as_bytes()).is_err());
        let oversized = text.replacen(
            "branch=\"-2147483648\"",
            &format!("branch=\"{}\"", "0".repeat(MAX_NUMBER_BYTES + 1)),
            1,
        );
        assert!(matches!(
            parse_genome_xml(oversized.as_bytes()),
            Err(XmlError::ResourceLimit {
                resource: "numeric attribute bytes",
                ..
            })
        ));
    }

    #[test]
    fn supported_declared_charset_forms_preserve_values() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        for name in [
            "UTF-8",
            "UTF-16",
            "UTF-16BE",
            "UTF-16LE",
            "ISO-8859-1",
            "US-ASCII",
        ] {
            for label in [name.to_owned(), name.to_ascii_lowercase()] {
                let document = text.replace("encoding=\"UTF-8\"", &format!("encoding=\"{label}\""));
                let bytes = if name == "UTF-16LE" {
                    [
                        vec![0xff, 0xfe],
                        document.encode_utf16().flat_map(u16::to_le_bytes).collect(),
                    ]
                    .concat()
                } else if name.starts_with("UTF-16") {
                    [
                        vec![0xfe, 0xff],
                        document.encode_utf16().flat_map(u16::to_be_bytes).collect(),
                    ]
                    .concat()
                } else {
                    document.into_bytes()
                };
                let parsed = parse_genome_xml(&bytes).unwrap();
                assert_eq!(parsed.genes().unwrap().len(), 2, "{label}");
                assert_eq!(
                    parsed.gene_ref(0).unwrap().borrow().length().to_bits(),
                    (-0.0_f64).to_bits(),
                    "{label}"
                );
            }
        }
        let pi = text.replacen(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
            "<?xml-stylesheet href=\"caf\u{e9}\"?>",
            1,
        );
        assert_eq!(parse_genome_xml(pi.as_bytes()).unwrap().genes().unwrap().len(), 2);
        let utf16pi: Vec<u8> = [
            vec![0xfe, 0xff],
            pi.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        ]
        .concat();
        assert_eq!(parse_genome_xml(&utf16pi).unwrap().genes().unwrap().len(), 2);
        let unicode_pi = pi.replace("xml-stylesheet", "xml\u{e9}");
        let utf16pi: Vec<u8> = [
            vec![0xfe, 0xff],
            unicode_pi
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect(),
        ]
        .concat();
        assert_eq!(parse_genome_xml(&utf16pi).unwrap().genes().unwrap().len(), 2);
        let latin = text.replace("encoding=\"UTF-8\"", "encoding=\"ISO-8859-1\"");
        let mut bytes = latin.into_bytes();
        bytes.extend_from_slice(b"<!-- ");
        bytes.push(0x80);
        bytes.push(0xe9);
        bytes.extend_from_slice(b" -->");
        assert_eq!(parse_genome_xml(&bytes).unwrap().genes().unwrap().len(), 2);
    }

    #[test]
    fn processing_instruction_target_cannot_be_empty() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let invalid = text.replace("<!DOCTYPE", "<??><!DOCTYPE");
        assert!(matches!(
            parse_genome_xml(invalid.as_bytes()),
            Err(XmlError::Malformed(_))
        ));
    }

    #[test]
    fn processing_instruction_target_requires_an_xml_name() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        for instruction in ["<?1bad?>", "<?XML?>"] {
            let invalid = text.replace("<!DOCTYPE", &format!("{instruction}<!DOCTYPE"));
            assert!(matches!(
                parse_genome_xml(invalid.as_bytes()),
                Err(XmlError::Malformed(_))
            ));
        }
    }

    #[test]
    fn windows1252_decodes_source_punctuation_and_undefined_byte_replacements() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let mut bytes = text
            .replace("encoding=\"UTF-8\"", "encoding=\"windows-1252\"")
            .into_bytes();
        bytes.extend_from_slice(b"<!-- ");
        bytes.extend_from_slice(&[0x80, 0x81, 0x82, 0x85, 0x9f, 0xe9]);
        bytes.extend_from_slice(b" -->");
        assert_eq!(parse_genome_xml(&bytes).unwrap().genes().unwrap().len(), 2);
        assert!(
            decode_document(&bytes)
                .unwrap()
                .0
                .contains("\u{20ac}\u{fffd}\u{201a}\u{2026}\u{178}\u{e9}")
        );
    }

    #[test]
    fn proven_utf8_aliases_keep_java_replacement_separate_from_strict_utf8() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        for label in ["UTF8", "unicode-1-1-utf-8"] {
            let mut bytes = text
                .replace("encoding=\"UTF-8\"", &format!("encoding=\"{label}\""))
                .into_bytes();
            bytes.extend_from_slice(b"<!-- ");
            bytes.extend_from_slice(&[0xed, 0xa0, 0x80]);
            bytes.extend_from_slice(b" -->");
            assert_eq!(parse_genome_xml(&bytes).unwrap().genes().unwrap().len(), 2);
            assert!(
                decode_document(&bytes)
                    .unwrap()
                    .0
                    .contains("<!-- \u{fffd} -->")
            );
        }
        let mut strict = text.into_bytes();
        strict.extend_from_slice(b"<!-- ");
        strict.extend_from_slice(&[0xed, 0xa0, 0x80]);
        strict.extend_from_slice(b" -->");
        assert!(matches!(
            parse_genome_xml(&strict),
            Err(XmlError::Encoding(_))
        ));
    }

    #[test]
    fn java_utf16_aliases_preserve_observed_surrogate_replacement() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let document = text.replace("encoding=\"UTF-8\"", "encoding=\"UnicodeBigUnmarked\"");
        let mut bytes: Vec<u8> = document.encode_utf16().flat_map(u16::to_be_bytes).collect();
        bytes.extend_from_slice(&[
            0, 60, 0, 33, 0, 45, 0, 45, 0, 32, 0xd8, 0, 0, 65, 0, 32, 0, 45, 0, 45, 0, 62,
        ]);
        assert_eq!(parse_genome_xml(&bytes).unwrap().genes().unwrap().len(), 2);
        assert!(
            decode_document(&bytes)
                .unwrap()
                .0
                .contains("<!-- \u{fffd} -->")
        );
        let strict = document.replace("UnicodeBigUnmarked", "UTF-16BE");
        let mut bytes: Vec<u8> = strict.encode_utf16().flat_map(u16::to_be_bytes).collect();
        bytes.extend_from_slice(&[
            0, 60, 0, 33, 0, 45, 0, 45, 0, 32, 0xdc, 0, 0, 32, 0, 45, 0, 45, 0, 62,
        ]);
        assert!(matches!(
            parse_genome_xml(&bytes),
            Err(XmlError::Encoding(_))
        ));
    }

    #[test]
    fn explicit_utf32_preserves_proven_endian_and_scalar_boundaries() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        for (label, big_endian) in [
            ("UTF-32BE", true),
            ("UTF_32BE", true),
            ("UTF-32LE", false),
            ("UTF_32LE", false),
        ] {
            let document = text.replace("encoding=\"UTF-8\"", &format!("encoding=\"{label}\""));
            let mut units: Vec<u32> = document.chars().map(u32::from).collect();
            units.extend("<!-- ".chars().map(u32::from));
            units.push(0x110000);
            units.extend(" -->".chars().map(u32::from));
            let encode = |values: &[u32]| {
                values
                    .iter()
                    .flat_map(|value| {
                        if big_endian {
                            value.to_be_bytes()
                        } else {
                            value.to_le_bytes()
                        }
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(parse_genome_xml(&encode(&units)).unwrap().genes().unwrap().len(), 2);
            let index = units.len() - 5;
            assert_eq!(units[index], 0x110000);
            units[index] = 0xd800;
            assert!(matches!(
                parse_genome_xml(&encode(&units)),
                Err(XmlError::Encoding(_))
            ));
        }
    }

    #[test]
    fn replacement_decoders_preserve_the_decoded_byte_budget() {
        let bytes = vec![0x80; MAX_DOCUMENT_BYTES / 3 + 1];
        assert!(matches!(
            decode_java_utf8(&bytes),
            Err(XmlError::ResourceLimit {
                resource: "decoded bytes",
                ..
            })
        ));
        let bytes = vec![0x81; MAX_DOCUMENT_BYTES / 3 + 1];
        assert!(matches!(
            decode_single_byte(&bytes, Charset::Windows1252, true),
            Err(XmlError::ResourceLimit {
                resource: "decoded bytes",
                ..
            })
        ));
        let bytes = vec![0xd0; MAX_DOCUMENT_BYTES / 2];
        assert_eq!(decode_single_byte(&bytes, Charset::Iso88599, true).unwrap().len(), MAX_DOCUMENT_BYTES);
        let bytes = vec![0xd0; MAX_DOCUMENT_BYTES / 2 + 1];
        assert!(matches!(
            decode_single_byte(&bytes, Charset::Iso88599, true),
            Err(XmlError::ResourceLimit { resource: "decoded bytes", .. })
        ));

        let header = "<?xml version=\"1.1\"\u{85}encoding=\"ISO-8859-1\"?>";
        let normalized_header = header.replace('\u{85}', "\n");
        let decoded_body_bytes = MAX_DOCUMENT_BYTES - header.len();
        let mut document = header.as_bytes().to_vec();
        document.resize(header.len() + decoded_body_bytes / 2, 0xe9);
        if decoded_body_bytes % 2 != 0 {
            document.push(b'a');
        }
        assert!(document.len() < MAX_DOCUMENT_BYTES);
        let (decoded, version) = decode_document(&document).unwrap();
        assert_eq!(version, XmlVersion::Xml11);
        assert_eq!(decoded.len(), MAX_DOCUMENT_BYTES - 1);
        assert!(decoded.starts_with(normalized_header.as_str()));
        drop(decoded);
        document.push(b'a');
        assert!(document.len() < MAX_DOCUMENT_BYTES);
        assert!(matches!(
            decode_document(&document),
            Err(XmlError::ResourceLimit {
                resource: "decoded bytes",
                maximum: MAX_DOCUMENT_BYTES,
            })
        ));
        drop(document);

        let mut document = header.as_bytes().to_vec();
        document.extend_from_slice(&[0xef, 0xbb, 0xbf]);
        let (decoded, version) = decode_document(&document).unwrap();
        assert_eq!(version, XmlVersion::Xml11);
        assert_eq!(
            decoded,
            format!("{}\u{ef}\u{bb}\u{bf}", normalized_header),
        );
        assert!(matches!(
            decode_single_byte(&[0xef, 0xbb, 0xbf], Charset::Latin1, true),
            Err(XmlError::Encoding(_))
        ));
    }

    #[test]
    fn iso_8859_9_carriers_match_original_dom_codepoints() {
        fn carrier(bytes: &[u8]) -> String {
            let (document, _) = decode_document(bytes).unwrap();
            let mut reader = Reader::from_str(&document);
            let mut grammar = None;
            loop {
                match reader.read_event().unwrap() {
                    Event::DocType(value) => grammar = Some(Dtd::parse(&value.decode().unwrap()).unwrap()),
                    Event::Start(element) if element.name().as_ref() == b"genetic_code" => {
                        let mut raw = BTreeMap::new();
                        for attribute in element.attributes() {
                            let attribute = attribute.unwrap();
                            let key = std::str::from_utf8(attribute.key.as_ref()).unwrap().to_owned();
                            let value = reader.decoder().decode(attribute.value.as_ref()).unwrap().into_owned();
                            assert!(raw.insert(key, value).is_none());
                        }
                        let attributes = grammar.unwrap().attributes("genetic_code", raw, &mut ExpansionBudget::default()).unwrap();
                        return attributes["probe"].chars().map(|value| format!("{:08x}", u32::from(value))).collect::<Vec<_>>().join(",");
                    }
                    Event::Eof => panic!("Captured carrier root is absent"),
                    _ => {}
                }
            }
        }
        let base = crate::oracle_tests::fixture_path("crates/core").join("tests/fixtures/xml_iso_8859_9/cases");
        let mut file = None;
        let mut successful = 0;
        let mut cases = 0;
        for line in include_str!("../tests/fixtures/xml_iso_8859_9/observations.tsv").lines() {
            let row: Vec<_> = line.split('\t').collect();
            match row[0] {
                "C" => { file = None; cases += 1; }
                "F" => {
                    assert!(row[1].bytes().all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || b"-.".contains(&value)));
                    assert!(row[1].ends_with(".bgg") && !row[1].starts_with('.'));
                    file = Some((row[1], row[3].parse::<usize>().unwrap()));
                }
                "D" => {
                    let (file, length) = file.unwrap();
                    let bytes = std::fs::read(base.join(file)).unwrap();
                    assert_eq!(bytes.len(), length);
                    assert_eq!(carrier(&bytes), row[1], "{file}");
                    successful += 1;
                }
                _ => {}
            }
        }
        assert_eq!((cases, successful), (260, 226));
        let casing = base.parent().unwrap().join("casing");
        let mut names = Vec::new();
        for line in include_str!("../tests/fixtures/xml_iso_8859_9/casing/observations.tsv").lines() {
            let row: Vec<_> = line.split('\t').collect();
            assert_eq!(row.len(), 5);assert_eq!(&row[1..3], &["success", "1"]);
            let bytes = std::fs::read(casing.join(format!("{}.bgg", row[0]))).unwrap();
            assert_eq!(carrier(&bytes), row[3]);names.push(row[4]);
        }
        assert_eq!(names, ["iso-8859-9", "Iso-8859-9"]);
    }

    #[test]
    fn declared_ascii_rejects_non_ascii_bytes_without_whatwg_aliasing() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let ascii = text.replace("encoding=\"UTF-8\"", "encoding=\"US-ASCII\"");
        assert_eq!(parse_genome_xml(ascii.as_bytes()).unwrap().genes().unwrap().len(), 2);
        let bad = ascii.replace(
            "<genetic_code symmetry",
            "<!-- caf\u{e9} --><genetic_code symmetry",
        );
        assert!(matches!(
            parse_genome_xml(bad.as_bytes()),
            Err(XmlError::Encoding(_))
        ));
        let mut single_byte = ascii.into_bytes();
        single_byte.extend_from_slice(b"<!-- ");
        single_byte.push(0xe9);
        single_byte.extend_from_slice(b" -->");
        assert!(matches!(
            parse_genome_xml(&single_byte),
            Err(XmlError::Encoding(_))
        ));
    }

    #[test]
    fn unproved_declared_charsets_fail_before_body_decoding() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        assert!(text.is_ascii());
        for name in ["Shift_JIS", "ISO_8859-9", "ISO8859_9", "latin5"] {
            let document = text.replace("encoding=\"UTF-8\"", &format!("encoding=\"{name}\""));
            let parsed = parse_genome_xml(document.as_bytes()).unwrap();
            assert_eq!(parsed.genes().unwrap().len(), 2, "{name}");
            assert_eq!(parsed.gene_ref(0).unwrap().borrow().length().to_bits(), (-0.0_f64).to_bits(), "{name}");
        }
        for name in [
            "UTF_8",
            "ISO88591",
            "x-cp1252",
            "windows1252",
        ] {
            let unknown = text.replace("encoding=\"UTF-8\"", &format!("encoding=\"{name}\""));
            assert!(
                matches!(
                    parse_genome_xml(unknown.as_bytes()),
                    Err(XmlError::Encoding(_))
                ),
                "{name}"
            );
        }
        let unknown = text.replace("encoding=\"UTF-8\"", "encoding=\"Windows-1252\"");
        let bytes = [
            vec![0xfe, 0xff],
            unknown.encode_utf16().flat_map(u16::to_be_bytes).collect(),
        ]
        .concat();
        assert!(matches!(
            parse_genome_xml(&bytes),
            Err(XmlError::Encoding(_))
        ));
        let unknown = text.replace("encoding=\"UTF-8\"", "encoding=\"UTF-32\"");
        let utf32be: Vec<u8> = [
            vec![0, 0, 0xfe, 0xff],
            unknown
                .chars()
                .flat_map(|ch| (ch as u32).to_be_bytes())
                .collect(),
        ]
        .concat();
        let utf32le: Vec<u8> = [
            vec![0xff, 0xfe, 0, 0],
            unknown
                .chars()
                .flat_map(|ch| (ch as u32).to_le_bytes())
                .collect(),
        ]
        .concat();
        for bytes in [utf32be, utf32le] {
            assert!(matches!(
                parse_genome_xml(&bytes),
                Err(XmlError::Encoding(_))
            ));
        }
    }

    #[test]
    fn encoding_and_document_boundaries_reject_conflicts_and_trailing_roots() {
        let text = String::from_utf8(encode_genome_xml(&genome()).unwrap()).unwrap();
        let correct = text.replace("encoding=\"UTF-8\"", "encoding=\"UTF-16\"");
        let utf16 = |text: &str| {
            [
                vec![0xfe, 0xff],
                text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
            ]
            .concat()
        };
        assert_eq!(parse_genome_xml(&utf16(&correct)).unwrap().genes().unwrap().len(), 2);
        assert!(matches!(
            parse_genome_xml(&utf16(&text)),
            Err(XmlError::Encoding(_))
        ));
        assert!(parse_genome_xml(format!("{text}<genetic_code/>").as_bytes()).is_err());
        assert!(matches!(
            parse_genome_xml(&vec![b' '; MAX_DOCUMENT_BYTES + 1]),
            Err(XmlError::ResourceLimit { .. })
        ));
    }
    #[test]
    fn retained_attribute_limit_rejects_owned_copies_and_recovers() {
        let mut bound = dtd::RetainedPayloadBudget::default();
        bound.charge(dtd::MAX_RETAINED_PAYLOAD_BYTES).unwrap();
        assert!(matches!(bound.charge(1), Err(XmlError::ResourceLimit { resource: "retained XML payload bytes", .. })));
        let original = include_str!("../tests/fixtures/xml_tokenized_attributes/cases/id-basic.bgg");
        let large = original.replace("probe=\"alpha\"", &format!("probe=\"{}\"", "a".repeat(MAX_DOCUMENT_BYTES / 2 + 1_048_576)));
        assert!(large.len() < MAX_DOCUMENT_BYTES);
        assert!(matches!(parse_genome_xml(large.as_bytes()), Err(XmlError::ResourceLimit { resource: "retained XML payload bytes", .. })));
        assert_eq!(parse_genome_xml(original.as_bytes()).unwrap().gene_count().unwrap(), 2);
    }

    #[test]
    fn idref_lists_use_byte_bounds_without_entity_reference_count_limit() {
        let original = include_str!("../tests/fixtures/xml_tokenized_attributes/cases/four-ids-ordered-repeated-refs.bgg");
        let repeated = original.replace("refs=\"delta beta alpha gamma delta\"", &format!("refs=\"{}\"", "alpha ".repeat(100_001)));
        assert!(repeated.len() < MAX_DOCUMENT_BYTES);
        assert_eq!(parse_genome_xml(repeated.as_bytes()).unwrap().gene_count().unwrap(), 4);
    }

    fn replacement_document(declarations: &str, content: &str, model: &str) -> String {
        let seed = include_str!("../tests/fixtures/xml/cases/0000/input.bgg");
        let source = seed.replacen("<!ELEMENT genetic_code (gene+)>", &format!("<!ELEMENT genetic_code {model}>"), 1);
        let start = source.find("<genetic_code").unwrap();
        let body = start + source[start..].find('>').unwrap() + 1;
        format!("{}{}\n</genetic_code>\n", &source[..body], content)
            .replacen("]>", &format!("{declarations}\n]>"), 1)
    }

    #[test]
    fn replacement_markup_reuses_validation_and_rejects_cross_entity_structures() {
        let original = include_str!("../../../tests/fixtures/java/xml_entity_markup/cases/single-empty-gene.bgg");
        let repeated = original.replace("(gene+)", "(gene,(gene|aux))")
            .replace("]>", "<!ELEMENT aux EMPTY>\n]>")
            .replace("&one;", "&one;&one;");
        let parsed = parse_genome_xml(repeated.as_bytes()).unwrap();
        let genes = parsed.genes().unwrap();
        assert_eq!(genes.len(), 2);
        assert!(!Rc::ptr_eq(&genes[0], &genes[1]));
        assert_eq!(genes[0].borrow().length().to_bits(), genes[1].borrow().length().to_bits());
        assert!(matches!(parse_genome_xml(include_bytes!("../../../tests/fixtures/java/xml_entity_markup/cases/open-in-entity-close-literal.bgg")), Err(XmlError::Malformed(_))));
        assert!(matches!(parse_genome_xml(include_bytes!("../../../tests/fixtures/java/xml_entity_markup/cases/open-literal-close-in-entity.bgg")), Err(XmlError::Malformed(_))));
        for replacement in ["<?xml version=\"1.0\"?>", "<!DOCTYPE genetic_code>", "<!-- bad--comment -->"] {
            let document = replacement_document(&format!("<!ENTITY invalid '{replacement}'>"), "&invalid;", "(#PCDATA|gene)*");
            assert!(matches!(parse_genome_xml(document.as_bytes()), Err(XmlError::Malformed(_))));
        }
        assert_eq!(parse_genome_xml(original.as_bytes()).unwrap().gene_count().unwrap(), 1);
    }

    #[test]
    fn replacement_markup_shares_cumulative_limits_and_fresh_parse_recovery() {
        const GENERATED: usize = 1 << 20;
        let exact = replacement_document(&format!("<!ENTITY text '{}'>", "x".repeat(GENERATED / 2)), "&text;", "(#PCDATA|gene)*");
        assert_eq!(parse_genome_xml(exact.as_bytes()).unwrap().gene_count().unwrap(), 0);
        let exceeded = exact.replacen("<!ENTITY text '", "<!ENTITY text 'x", 1);
        assert!(matches!(parse_genome_xml(exceeded.as_bytes()), Err(XmlError::ResourceLimit { resource: "expanded entity bytes", maximum: GENERATED })));

        let grammar = Dtd::parse("r [<!ELEMENT r (#PCDATA)><!ENTITY bom '&#xfeff;&#xfeff;'><!ENTITY outer '&bom;'>]").unwrap();
        // Each BOM has three UTF-8 bytes; both must survive every ancestry level
        for (reference, cost) in [("&bom;", 12), ("&outer;", 18)] {
            let mut scalar = ExpansionBudget::default();
            scalar.generated(GENERATED - cost, 1).unwrap();
            assert_eq!(grammar.text(reference, &mut scalar).unwrap(), "\u{feff}\u{feff}");
            let mut observed = String::new();
            let mut state = ValidationState::new(XmlVersion::Xml10, |_: &str, _: &BTreeMap<String, String>| {}, |value| observed.push_str(value));
            let mut opening = Reader::from_str("<r>");
            let event = opening.read_event().unwrap();
            state.event(event, &opening, Some(&grammar), &mut Vec::new(), None, 3).unwrap();
            state.budget.generated(GENERATED - cost, 1).unwrap();
            let mut input = Reader::from_str(reference);
            let event = input.read_event().unwrap();
            let mut trail = Vec::new();
            state.event(event, &input, Some(&grammar), &mut trail, None, reference.len()).unwrap();
            assert!(state.stack.last().unwrap().text && state.stack.last().unwrap().nonspace);
            assert!(trail.is_empty());
            let mut input = Reader::from_str(reference);
            let event = input.read_event().unwrap();
            assert!(matches!(state.event(event, &input, Some(&grammar), &mut trail, None, reference.len()), Err(XmlError::ResourceLimit { resource: "expanded entity bytes", .. })));
            assert!(trail.is_empty());
            assert_eq!(observed, "\u{feff}\u{feff}");
        }
        let seed = include_str!("../tests/fixtures/xml/cases/0000/input.bgg");
        let start = seed.find("<genetic_code").unwrap();
        let body = start + seed[start..].find('>').unwrap() + 1;
        let gene = &seed[body..seed.find("</genetic_code>").unwrap()];
        for model in ["(gene+)", "EMPTY"] {
            let content = if model == "EMPTY" { "&bom;".to_owned() } else { format!("&bom;{gene}") };
            let document = replacement_document("<!ENTITY bom '&#xfeff;&#xfeff;'>", &content, model);
            assert!(matches!(parse_genome_xml(document.as_bytes()), Err(XmlError::Malformed(_))));
        }
        let valid = include_bytes!("../../../tests/fixtures/java/xml_entity_markup/cases/single-empty-gene.bgg");
        assert_eq!(parse_genome_xml(valid).unwrap().gene_count().unwrap(), 1);
    }

}


#[cfg(test)]
mod resource_stress {
    #[test]
    #[ignore = "Root-owned release screen; no Pi or performance acceptance"]
    fn nullable_ambiguous_model_and_repeated_children() {
        const POSITIONS: usize = 2048;
        const CHILDREN: usize = 2048;
        let seed = include_str!("../tests/fixtures/xml_child_models/cases/seed-0000.bgg");
        let model = format!("(({})*)", vec!["gene?"; POSITIONS].join(","));
        let source = seed.replacen("(gene+)", &model, 1);
        let root = source.find("<genetic_code").unwrap();
        let body = root + source[root..].find('>').unwrap() + 1;
        let gene = source.find("<gene ").unwrap();
        let gene_end = gene + source[gene..].find("/>").unwrap() + 2;
        let close = source.find("</genetic_code>").unwrap();
        let document = format!("{}\n{}\n{}", &source[..body], source[gene..gene_end].repeat(CHILDREN), &source[close..]);
        assert!(document.len() < super::MAX_DOCUMENT_BYTES && CHILDREN < super::MAX_GENES);
        let started = std::time::Instant::now();
        let genome = super::parse_genome_xml(document.as_bytes()).unwrap();
        let seconds = started.elapsed().as_secs_f64();
        assert_eq!(genome.genes().unwrap().len(), CHILDREN);
        println!("XML_CHILD_MODEL_STRESS PASS positions={POSITIONS} children={CHILDREN} bytes={} parse_seconds={seconds:.9}", document.len());
    }
}
