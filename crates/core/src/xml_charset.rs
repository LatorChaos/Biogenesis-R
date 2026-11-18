// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-only

//! Reader-policy lookup and bounded legacy charset decoding
//! State rules adapt OpenJDK 21u eac07229 under GPLv2; upstream notices apply only to those portions
//! Retained upstream attribution and license headers accompany xml_charset_data/NOTICE.txt

use crate::xml::{MAX_DOCUMENT_BYTES, XmlError};

#[derive(Clone, Copy)]
pub(crate) struct JavaEncoding {
    pub(crate) name: &'static str,
    pub(crate) strict_ascii: bool,
}

pub(crate) fn java_encoding(label: &str) -> Option<JavaEncoding> {
    let key = label.to_ascii_uppercase();
    JAVA_ENCODINGS.binary_search_by_key(&key.as_str(), |entry| entry.0).ok().map(|index| {
        let (_, name, strict_ascii) = JAVA_ENCODINGS[index];
        JavaEncoding { name, strict_ascii }
    })
}

// Installed EncodingMap routing followed by the pinned JDK canonical alias inventory
// Native XML readers take precedence in xml.rs
const JAVA_ENCODINGS: &[(&str, &str, bool)] = &[
    ("037", "IBM037", false),
    ("1006", "x-IBM1006", false),
    ("1025", "x-IBM1025", false),
    ("1026", "IBM1026", false),
    ("1046", "x-IBM1046", false),
    ("1047", "IBM1047", false),
    ("1089", "ISO-8859-6", false),
    ("1097", "x-IBM1097", false),
    ("1098", "x-IBM1098", false),
    ("1112", "x-IBM1112", false),
    ("1122", "x-IBM1122", false),
    ("1123", "x-IBM1123", false),
    ("1124", "x-IBM1124", false),
    ("1129", "x-IBM1129", false),
    ("1140", "IBM01140", false),
    ("1141", "IBM01141", false),
    ("1142", "IBM01142", false),
    ("1143", "IBM01143", false),
    ("1144", "IBM01144", false),
    ("1145", "IBM01145", false),
    ("1146", "IBM01146", false),
    ("1147", "IBM01147", false),
    ("1148", "IBM01148", false),
    ("1149", "IBM01149", false),
    ("1166", "x-IBM1166", false),
    ("1364", "x-IBM1364", false),
    ("1381", "x-IBM1381", false),
    ("1383", "x-IBM1383", false),
    ("273", "IBM273", false),
    ("277", "IBM277", false),
    ("278", "IBM278", false),
    ("280", "IBM280", false),
    ("284", "IBM284", false),
    ("285", "IBM285", false),
    ("290", "IBM290", false),
    ("29626C", "x-IBM29626C", false),
    ("297", "IBM297", false),
    ("300", "x-IBM300", false),
    ("33722", "x-IBM33722", false),
    ("420", "IBM420", false),
    ("424", "IBM424", false),
    ("437", "IBM437", false),
    ("500", "IBM500", false),
    ("5601", "EUC-KR", false),
    ("646", "US-ASCII", false),
    ("737", "x-IBM737", false),
    ("775", "IBM775", false),
    ("813", "ISO-8859-7", false),
    ("819", "ISO-8859-1", false),
    ("833", "x-IBM833", false),
    ("834", "x-IBM834", false),
    ("838", "IBM-Thai", false),
    ("850", "IBM850", false),
    ("852", "IBM852", false),
    ("855", "IBM855", false),
    ("856", "x-IBM856", false),
    ("857", "IBM857", false),
    ("858", "IBM00858", false),
    ("860", "IBM860", false),
    ("861", "IBM861", false),
    ("862", "IBM862", false),
    ("863", "IBM863", false),
    ("864", "IBM864", false),
    ("865", "IBM865", false),
    ("866", "IBM866", false),
    ("868", "IBM868", false),
    ("869", "IBM869", false),
    ("870", "IBM870", false),
    ("871", "IBM871", false),
    ("874", "x-IBM874", false),
    ("875", "x-IBM875", false),
    ("8859_1", "ISO-8859-1", false),
    ("8859_13", "ISO-8859-13", false),
    ("8859_15", "ISO-8859-15", false),
    ("8859_2", "ISO-8859-2", false),
    ("8859_3", "ISO-8859-3", false),
    ("8859_4", "ISO-8859-4", false),
    ("8859_5", "ISO-8859-5", false),
    ("8859_6", "ISO-8859-6", false),
    ("8859_7", "ISO-8859-7", false),
    ("8859_8", "ISO-8859-8", false),
    ("8859_9", "ISO-8859-9", false),
    ("912", "ISO-8859-2", false),
    ("913", "ISO-8859-3", false),
    ("914", "ISO-8859-4", false),
    ("915", "ISO-8859-5", false),
    ("916", "ISO-8859-8", false),
    ("918", "IBM918", false),
    ("920", "ISO-8859-9", false),
    ("921", "x-IBM921", false),
    ("922", "x-IBM922", false),
    ("923", "ISO-8859-15", false),
    ("930", "x-IBM930", false),
    ("932", "x-IBM942C", false),
    ("933", "x-IBM933", false),
    ("935", "x-IBM935", false),
    ("937", "x-IBM937", false),
    ("939", "x-IBM939", false),
    ("942", "x-IBM942", false),
    ("942C", "x-IBM942C", false),
    ("943", "x-IBM943", false),
    ("943C", "x-IBM943C", false),
    ("948", "x-IBM948", false),
    ("949", "x-IBM949", false),
    ("949C", "x-IBM949C", false),
    ("950", "x-IBM950", false),
    ("964", "x-IBM964", false),
    ("970", "x-IBM970", false),
    ("ANSI-1251", "windows-1251", false),
    ("ANSI_X3.4-1968", "US-ASCII", true),
    ("ANSI_X3.4-1986", "US-ASCII", true),
    ("ARABIC", "ISO-8859-6", false),
    ("ASCII", "US-ASCII", true),
    ("ASCII7", "US-ASCII", false),
    ("ASMO-708", "ISO-8859-6", false),
    ("BIG5", "Big5", false),
    ("BIG5-HKSCS", "Big5-HKSCS", false),
    ("BIG5-HKSCS-2001", "x-Big5-HKSCS-2001", false),
    ("BIG5-HKSCS:UNICODE3.0", "x-Big5-HKSCS-2001", false),
    ("BIG5HK", "Big5-HKSCS", false),
    ("BIG5HK-2001", "x-Big5-HKSCS-2001", false),
    ("BIG5HKSCS", "Big5-HKSCS", false),
    ("BIG5HKSCS-2001", "x-Big5-HKSCS-2001", false),
    ("BIG5_HKSCS", "Big5-HKSCS", false),
    ("BIG5_HKSCS_2001", "x-Big5-HKSCS-2001", false),
    ("BIG5_SOLARIS", "x-Big5-Solaris", false),
    ("CCSID00858", "IBM00858", false),
    ("CCSID01140", "IBM01140", false),
    ("CCSID01141", "IBM01141", false),
    ("CCSID01142", "IBM01142", false),
    ("CCSID01143", "IBM01143", false),
    ("CCSID01144", "IBM01144", false),
    ("CCSID01145", "IBM01145", false),
    ("CCSID01146", "IBM01146", false),
    ("CCSID01147", "IBM01147", false),
    ("CCSID01148", "IBM01148", false),
    ("CCSID01149", "IBM01149", false),
    ("CESU-8", "CESU-8", false),
    ("CESU8", "CESU-8", false),
    ("CNS11643", "x-EUC-TW", false),
    ("CP-AR", "IBM868", false),
    ("CP-GR", "IBM869", false),
    ("CP-IS", "IBM861", false),
    ("CP00858", "IBM00858", false),
    ("CP01140", "IBM01140", false),
    ("CP01141", "IBM01141", false),
    ("CP01142", "IBM01142", false),
    ("CP01143", "IBM01143", false),
    ("CP01144", "IBM01144", false),
    ("CP01145", "IBM01145", false),
    ("CP01146", "IBM01146", false),
    ("CP01147", "IBM01147", false),
    ("CP01148", "IBM01148", false),
    ("CP01149", "IBM01149", false),
    ("CP037", "IBM037", false),
    ("CP1006", "x-IBM1006", false),
    ("CP1025", "x-IBM1025", false),
    ("CP1026", "IBM1026", false),
    ("CP1046", "x-IBM1046", false),
    ("CP1047", "IBM1047", false),
    ("CP1089", "ISO-8859-6", false),
    ("CP1097", "x-IBM1097", false),
    ("CP1098", "x-IBM1098", false),
    ("CP1112", "x-IBM1112", false),
    ("CP1122", "x-IBM1122", false),
    ("CP1123", "x-IBM1123", false),
    ("CP1124", "x-IBM1124", false),
    ("CP1129", "x-IBM1129", false),
    ("CP1140", "IBM01140", false),
    ("CP1141", "IBM01141", false),
    ("CP1142", "IBM01142", false),
    ("CP1143", "IBM01143", false),
    ("CP1144", "IBM01144", false),
    ("CP1145", "IBM01145", false),
    ("CP1146", "IBM01146", false),
    ("CP1147", "IBM01147", false),
    ("CP1148", "IBM01148", false),
    ("CP1149", "IBM01149", false),
    ("CP1166", "x-IBM1166", false),
    ("CP1250", "windows-1250", false),
    ("CP1251", "windows-1251", false),
    ("CP1252", "windows-1252", false),
    ("CP1253", "windows-1253", false),
    ("CP1254", "windows-1254", false),
    ("CP1255", "windows-1255", false),
    ("CP1256", "windows-1256", false),
    ("CP1257", "windows-1257", false),
    ("CP1258", "windows-1258", false),
    ("CP1364", "x-IBM1364", false),
    ("CP1381", "x-IBM1381", false),
    ("CP1383", "x-IBM1383", false),
    ("CP273", "IBM273", false),
    ("CP277", "IBM277", false),
    ("CP278", "IBM278", false),
    ("CP280", "IBM280", false),
    ("CP284", "IBM284", false),
    ("CP285", "IBM285", false),
    ("CP290", "IBM290", false),
    ("CP29626C", "x-IBM29626C", false),
    ("CP297", "IBM297", false),
    ("CP300", "x-IBM300", false),
    ("CP33722", "x-IBM33722", false),
    ("CP367", "US-ASCII", true),
    ("CP420", "IBM420", false),
    ("CP424", "IBM424", false),
    ("CP437", "IBM437", false),
    ("CP500", "IBM500", false),
    ("CP50220", "x-windows-50220", false),
    ("CP50221", "x-windows-50221", false),
    ("CP5346", "windows-1250", false),
    ("CP5347", "windows-1251", false),
    ("CP5348", "windows-1252", false),
    ("CP5349", "windows-1253", false),
    ("CP5350", "windows-1254", false),
    ("CP5353", "windows-1257", false),
    ("CP737", "x-IBM737", false),
    ("CP775", "IBM775", false),
    ("CP813", "ISO-8859-7", false),
    ("CP819", "ISO-8859-1", false),
    ("CP833", "x-IBM833", false),
    ("CP834", "x-IBM834", false),
    ("CP838", "IBM-Thai", false),
    ("CP850", "IBM850", false),
    ("CP852", "IBM852", false),
    ("CP855", "IBM855", false),
    ("CP856", "x-IBM856", false),
    ("CP857", "IBM857", false),
    ("CP858", "IBM00858", false),
    ("CP860", "IBM860", false),
    ("CP861", "IBM861", false),
    ("CP862", "IBM862", false),
    ("CP863", "IBM863", false),
    ("CP864", "IBM864", false),
    ("CP865", "IBM865", false),
    ("CP866", "IBM866", false),
    ("CP868", "IBM868", false),
    ("CP869", "IBM869", false),
    ("CP870", "IBM870", false),
    ("CP871", "IBM871", false),
    ("CP874", "x-IBM874", false),
    ("CP875", "x-IBM875", false),
    ("CP912", "ISO-8859-2", false),
    ("CP913", "ISO-8859-3", false),
    ("CP914", "ISO-8859-4", false),
    ("CP915", "ISO-8859-5", false),
    ("CP916", "ISO-8859-8", false),
    ("CP918", "IBM918", false),
    ("CP920", "ISO-8859-9", false),
    ("CP921", "x-IBM921", false),
    ("CP922", "x-IBM922", false),
    ("CP923", "ISO-8859-15", false),
    ("CP930", "x-IBM930", false),
    ("CP932", "x-IBM942C", false),
    ("CP933", "x-IBM933", false),
    ("CP935", "x-IBM935", false),
    ("CP936", "GBK", false),
    ("CP937", "x-IBM937", false),
    ("CP939", "x-IBM939", false),
    ("CP942", "x-IBM942", false),
    ("CP942C", "x-IBM942C", false),
    ("CP943", "x-IBM943", false),
    ("CP943C", "x-IBM943C", false),
    ("CP948", "x-IBM948", false),
    ("CP949", "x-IBM949", false),
    ("CP949C", "x-IBM949C", false),
    ("CP950", "x-IBM950", false),
    ("CP964", "x-IBM964", false),
    ("CP970", "x-IBM970", false),
    ("CPEUCCN", "x-IBM1383", false),
    ("CPIBM284", "IBM284", false),
    ("CPIBM285", "IBM285", false),
    ("CPIBM297", "IBM297", false),
    ("CPIBM37", "IBM037", false),
    ("CS-EBCDIC-CP-CA", "IBM037", false),
    ("CS-EBCDIC-CP-NL", "IBM037", false),
    ("CS-EBCDIC-CP-US", "IBM037", false),
    ("CS-EBCDIC-CP-WT", "IBM037", false),
    ("CSASCII", "US-ASCII", true),
    ("CSBIG5", "Big5", false),
    ("CSCESU-8", "CESU-8", false),
    ("CSEUCKR", "EUC-KR", false),
    ("CSEUCPKDFMTJAPANESE", "EUC-JP", false),
    ("CSGB2312", "GB2312", false),
    ("CSHALFWIDTHKATAKANA", "JIS_X0201", false),
    ("CSIBM037", "IBM037", false),
    ("CSIBM1026", "IBM1026", false),
    ("CSIBM273", "IBM273", false),
    ("CSIBM277", "IBM277", false),
    ("CSIBM278", "IBM278", false),
    ("CSIBM280", "IBM280", false),
    ("CSIBM284", "IBM284", false),
    ("CSIBM285", "IBM285", false),
    ("CSIBM290", "IBM290", false),
    ("CSIBM297", "IBM297", false),
    ("CSIBM420", "IBM420", false),
    ("CSIBM424", "IBM424", false),
    ("CSIBM500", "IBM500", false),
    ("CSIBM855", "IBM855", false),
    ("CSIBM857", "IBM857", false),
    ("CSIBM860", "IBM860", false),
    ("CSIBM861", "IBM861", false),
    ("CSIBM862", "IBM862", false),
    ("CSIBM863", "IBM863", false),
    ("CSIBM864", "IBM864", false),
    ("CSIBM865", "IBM865", false),
    ("CSIBM866", "IBM866", false),
    ("CSIBM868", "IBM868", false),
    ("CSIBM869", "IBM869", false),
    ("CSIBM870", "IBM870", false),
    ("CSIBM871", "IBM871", false),
    ("CSIBM918", "IBM918", false),
    ("CSISO13JISC6220JP", "JIS_X0201", false),
    ("CSISO153GOST1976874", "x-ISCII91", false),
    ("CSISO159JISX02121990", "JIS_X0212-1990", false),
    ("CSISO2022CN", "ISO-2022-CN", false),
    ("CSISO2022JP", "ISO-2022-JP", false),
    ("CSISO2022JP2", "ISO-2022-JP-2", false),
    ("CSISO2022KR", "ISO-2022-KR", false),
    ("CSISO87JISX0208", "x-JIS0208", false),
    ("CSISO885915", "ISO-8859-15", false),
    ("CSISO885916", "ISO-8859-16", false),
    ("CSISOLATIN0", "ISO-8859-15", false),
    ("CSISOLATIN1", "ISO-8859-1", false),
    ("CSISOLATIN2", "ISO-8859-2", false),
    ("CSISOLATIN3", "ISO-8859-3", false),
    ("CSISOLATIN4", "ISO-8859-4", false),
    ("CSISOLATIN5", "ISO-8859-9", false),
    ("CSISOLATIN9", "ISO-8859-15", false),
    ("CSISOLATINARABIC", "ISO-8859-6", false),
    ("CSISOLATINCYRILLIC", "ISO-8859-5", false),
    ("CSISOLATINGREEK", "ISO-8859-7", false),
    ("CSISOLATINHEBREW", "ISO-8859-8", false),
    ("CSJISENCODING", "ISO-2022-JP", false),
    ("CSKOI8R", "KOI8-R", false),
    ("CSKSC56011987", "EUC-KR", false),
    ("CSPC775BALTIC", "IBM775", false),
    ("CSPC850MULTILINGUAL", "IBM850", false),
    ("CSPC862LATINHEBREW", "IBM862", false),
    ("CSPC8CODEPAGE437", "IBM437", false),
    ("CSPCP852", "IBM852", false),
    ("CSPCP855", "IBM855", false),
    ("CSSHIFTJIS", "Shift_JIS", false),
    ("CSWINDOWS31J", "windows-31j", false),
    ("CYRILLIC", "ISO-8859-5", false),
    ("EBCDIC-CP-AR1", "IBM420", false),
    ("EBCDIC-CP-AR2", "IBM918", false),
    ("EBCDIC-CP-BE", "IBM500", false),
    ("EBCDIC-CP-BH", "IBM500", false),
    ("EBCDIC-CP-CA", "IBM037", false),
    ("EBCDIC-CP-CH", "IBM500", false),
    ("EBCDIC-CP-DK", "IBM277", false),
    ("EBCDIC-CP-ES", "IBM284", false),
    ("EBCDIC-CP-FI", "IBM278", false),
    ("EBCDIC-CP-FR", "IBM297", false),
    ("EBCDIC-CP-GB", "IBM285", false),
    ("EBCDIC-CP-HE", "IBM424", false),
    ("EBCDIC-CP-IS", "IBM871", false),
    ("EBCDIC-CP-IT", "IBM280", false),
    ("EBCDIC-CP-NL", "IBM037", false),
    ("EBCDIC-CP-NO", "IBM277", false),
    ("EBCDIC-CP-ROECE", "IBM870", false),
    ("EBCDIC-CP-SE", "IBM278", false),
    ("EBCDIC-CP-US", "IBM037", false),
    ("EBCDIC-CP-WT", "IBM037", false),
    ("EBCDIC-CP-YU", "IBM870", false),
    ("EBCDIC-DE-273+EURO", "IBM01141", false),
    ("EBCDIC-DK-277+EURO", "IBM01142", false),
    ("EBCDIC-ES-284+EURO", "IBM01145", false),
    ("EBCDIC-FI-278+EURO", "IBM01143", false),
    ("EBCDIC-FR-277+EURO", "IBM01147", false),
    ("EBCDIC-GB", "IBM285", false),
    ("EBCDIC-GB-285+EURO", "IBM01146", false),
    ("EBCDIC-INTERNATIONAL-500+EURO", "IBM01148", false),
    ("EBCDIC-IT-280+EURO", "IBM01144", false),
    ("EBCDIC-JP-KANA", "IBM290", false),
    ("EBCDIC-NO-277+EURO", "IBM01142", false),
    ("EBCDIC-S-871+EURO", "IBM01149", false),
    ("EBCDIC-SE-278+EURO", "IBM01143", false),
    ("EBCDIC-SV", "IBM278", false),
    ("EBCDIC-US-037+EURO", "IBM01140", false),
    ("ECMA-114", "ISO-8859-6", false),
    ("ECMA-118", "ISO-8859-7", false),
    ("ELOT_928", "ISO-8859-7", false),
    ("EUC-CN", "GB2312", false),
    ("EUC-JP", "EUC-JP", false),
    ("EUC-JP-LINUX", "x-euc-jp-linux", false),
    ("EUC-KR", "EUC-KR", false),
    ("EUC-TW", "x-EUC-TW", false),
    ("EUCCN", "GB2312", false),
    ("EUCJIS", "EUC-JP", false),
    ("EUCJP", "EUC-JP", false),
    ("EUCJP-OPEN", "x-eucJP-Open", false),
    ("EUCKR", "EUC-KR", false),
    ("EUCTW", "x-EUC-TW", false),
    ("EUC_CN", "GB2312", false),
    ("EUC_JP", "EUC-JP", false),
    ("EUC_JP_LINUX", "x-euc-jp-linux", false),
    ("EUC_JP_SOLARIS", "x-eucJP-Open", false),
    ("EUC_KR", "EUC-KR", false),
    ("EUC_TW", "x-EUC-TW", false),
    ("EXTENDED_UNIX_CODE_PACKED_FORMAT_FOR_JAPANESE", "EUC-JP", false),
    ("GB18030", "GB18030", false),
    ("GB18030-2022", "GB18030", false),
    ("GB2312", "GB2312", false),
    ("GB2312-1980", "GB2312", false),
    ("GB2312-80", "GB2312", false),
    ("GBK", "GBK", false),
    ("GREEK", "ISO-8859-7", false),
    ("GREEK8", "ISO-8859-7", false),
    ("HEBREW", "ISO-8859-8", false),
    ("IBM-037", "IBM037", false),
    ("IBM-1006", "x-IBM1006", false),
    ("IBM-1025", "x-IBM1025", false),
    ("IBM-1026", "IBM1026", false),
    ("IBM-1046", "x-IBM1046", false),
    ("IBM-1047", "IBM1047", false),
    ("IBM-1089", "ISO-8859-6", false),
    ("IBM-1097", "x-IBM1097", false),
    ("IBM-1098", "x-IBM1098", false),
    ("IBM-1112", "x-IBM1112", false),
    ("IBM-1122", "x-IBM1122", false),
    ("IBM-1123", "x-IBM1123", false),
    ("IBM-1124", "x-IBM1124", false),
    ("IBM-1129", "x-IBM1129", false),
    ("IBM-1140", "IBM01140", false),
    ("IBM-1141", "IBM01141", false),
    ("IBM-1142", "IBM01142", false),
    ("IBM-1143", "IBM01143", false),
    ("IBM-1144", "IBM01144", false),
    ("IBM-1145", "IBM01145", false),
    ("IBM-1146", "IBM01146", false),
    ("IBM-1147", "IBM01147", false),
    ("IBM-1148", "IBM01148", false),
    ("IBM-1149", "IBM01149", false),
    ("IBM-1166", "x-IBM1166", false),
    ("IBM-1252", "windows-1252", false),
    ("IBM-1364", "x-IBM1364", false),
    ("IBM-1381", "x-IBM1381", false),
    ("IBM-1383", "x-IBM1383", false),
    ("IBM-273", "IBM273", false),
    ("IBM-277", "IBM277", false),
    ("IBM-278", "IBM278", false),
    ("IBM-280", "IBM280", false),
    ("IBM-284", "IBM284", false),
    ("IBM-285", "IBM285", false),
    ("IBM-290", "IBM290", false),
    ("IBM-29626C", "x-IBM29626C", false),
    ("IBM-297", "IBM297", false),
    ("IBM-300", "x-IBM300", false),
    ("IBM-33722", "x-IBM33722", false),
    ("IBM-33722_VASCII_VPUA", "x-IBM33722", false),
    ("IBM-367", "US-ASCII", true),
    ("IBM-37", "IBM037", false),
    ("IBM-420", "IBM420", false),
    ("IBM-424", "IBM424", false),
    ("IBM-437", "IBM437", false),
    ("IBM-500", "IBM500", false),
    ("IBM-5050", "x-IBM33722", false),
    ("IBM-737", "x-IBM737", false),
    ("IBM-775", "IBM775", false),
    ("IBM-813", "ISO-8859-7", false),
    ("IBM-819", "ISO-8859-1", false),
    ("IBM-833", "x-IBM833", false),
    ("IBM-834", "x-IBM834", false),
    ("IBM-838", "IBM-Thai", false),
    ("IBM-850", "IBM850", false),
    ("IBM-852", "IBM852", false),
    ("IBM-855", "IBM855", false),
    ("IBM-856", "x-IBM856", false),
    ("IBM-857", "IBM857", false),
    ("IBM-858", "IBM00858", false),
    ("IBM-860", "IBM860", false),
    ("IBM-861", "IBM861", false),
    ("IBM-862", "IBM862", false),
    ("IBM-863", "IBM863", false),
    ("IBM-864", "IBM864", false),
    ("IBM-865", "IBM865", false),
    ("IBM-866", "IBM866", false),
    ("IBM-868", "IBM868", false),
    ("IBM-869", "IBM869", false),
    ("IBM-870", "IBM870", false),
    ("IBM-871", "IBM871", false),
    ("IBM-874", "x-IBM874", false),
    ("IBM-875", "x-IBM875", false),
    ("IBM-912", "ISO-8859-2", false),
    ("IBM-913", "ISO-8859-3", false),
    ("IBM-914", "ISO-8859-4", false),
    ("IBM-915", "ISO-8859-5", false),
    ("IBM-916", "ISO-8859-8", false),
    ("IBM-918", "IBM918", false),
    ("IBM-920", "ISO-8859-9", false),
    ("IBM-921", "x-IBM921", false),
    ("IBM-922", "x-IBM922", false),
    ("IBM-923", "ISO-8859-15", false),
    ("IBM-930", "x-IBM930", false),
    ("IBM-932", "x-IBM942C", false),
    ("IBM-933", "x-IBM933", false),
    ("IBM-935", "x-IBM935", false),
    ("IBM-937", "x-IBM937", false),
    ("IBM-939", "x-IBM939", false),
    ("IBM-942", "x-IBM942", false),
    ("IBM-942C", "x-IBM942C", false),
    ("IBM-943", "x-IBM943", false),
    ("IBM-943C", "x-IBM943C", false),
    ("IBM-948", "x-IBM948", false),
    ("IBM-949", "x-IBM949", false),
    ("IBM-949C", "x-IBM949C", false),
    ("IBM-950", "x-IBM950", false),
    ("IBM-964", "x-IBM964", false),
    ("IBM-970", "x-IBM970", false),
    ("IBM-EUCCN", "x-IBM1383", false),
    ("IBM-EUCJP", "x-IBM29626C", false),
    ("IBM-EUCKR", "x-IBM970", false),
    ("IBM-EUCTW", "x-IBM964", false),
    ("IBM-THAI", "IBM-Thai", false),
    ("IBM00858", "IBM00858", false),
    ("IBM01140", "IBM01140", false),
    ("IBM01141", "IBM01141", false),
    ("IBM01142", "IBM01142", false),
    ("IBM01143", "IBM01143", false),
    ("IBM01144", "IBM01144", false),
    ("IBM01145", "IBM01145", false),
    ("IBM01146", "IBM01146", false),
    ("IBM01147", "IBM01147", false),
    ("IBM01148", "IBM01148", false),
    ("IBM01149", "IBM01149", false),
    ("IBM037", "IBM037", false),
    ("IBM1006", "x-IBM1006", false),
    ("IBM1025", "x-IBM1025", false),
    ("IBM1026", "IBM1026", false),
    ("IBM1046", "x-IBM1046", false),
    ("IBM1047", "IBM1047", false),
    ("IBM1089", "ISO-8859-6", false),
    ("IBM1097", "x-IBM1097", false),
    ("IBM1098", "x-IBM1098", false),
    ("IBM1112", "x-IBM1112", false),
    ("IBM1122", "x-IBM1122", false),
    ("IBM1123", "x-IBM1123", false),
    ("IBM1124", "x-IBM1124", false),
    ("IBM1129", "x-IBM1129", false),
    ("IBM1140", "IBM01140", false),
    ("IBM1141", "IBM01141", false),
    ("IBM1142", "IBM01142", false),
    ("IBM1143", "IBM01143", false),
    ("IBM1144", "IBM01144", false),
    ("IBM1145", "IBM01145", false),
    ("IBM1146", "IBM01146", false),
    ("IBM1147", "IBM01147", false),
    ("IBM1148", "IBM01148", false),
    ("IBM1149", "IBM01149", false),
    ("IBM1166", "x-IBM1166", false),
    ("IBM1252", "windows-1252", false),
    ("IBM1364", "x-IBM1364", false),
    ("IBM1381", "x-IBM1381", false),
    ("IBM1383", "x-IBM1383", false),
    ("IBM273", "IBM273", false),
    ("IBM277", "IBM277", false),
    ("IBM278", "IBM278", false),
    ("IBM280", "IBM280", false),
    ("IBM284", "IBM284", false),
    ("IBM285", "IBM285", false),
    ("IBM290", "IBM290", false),
    ("IBM29626C", "x-IBM29626C", false),
    ("IBM297", "IBM297", false),
    ("IBM300", "x-IBM300", false),
    ("IBM33722", "x-IBM33722", false),
    ("IBM367", "US-ASCII", true),
    ("IBM420", "IBM420", false),
    ("IBM424", "IBM424", false),
    ("IBM437", "IBM437", false),
    ("IBM500", "IBM500", false),
    ("IBM737", "x-IBM737", false),
    ("IBM775", "IBM775", false),
    ("IBM813", "ISO-8859-7", false),
    ("IBM819", "ISO-8859-1", false),
    ("IBM833", "x-IBM833", false),
    ("IBM834", "x-IBM834", false),
    ("IBM838", "IBM-Thai", false),
    ("IBM850", "IBM850", false),
    ("IBM852", "IBM852", false),
    ("IBM855", "IBM855", false),
    ("IBM856", "x-IBM856", false),
    ("IBM857", "IBM857", false),
    ("IBM858", "IBM00858", false),
    ("IBM860", "IBM860", false),
    ("IBM861", "IBM861", false),
    ("IBM862", "IBM862", false),
    ("IBM863", "IBM863", false),
    ("IBM864", "IBM864", false),
    ("IBM865", "IBM865", false),
    ("IBM866", "IBM866", false),
    ("IBM868", "IBM868", false),
    ("IBM869", "IBM869", false),
    ("IBM870", "IBM870", false),
    ("IBM871", "IBM871", false),
    ("IBM874", "x-IBM874", false),
    ("IBM875", "x-IBM875", false),
    ("IBM912", "ISO-8859-2", false),
    ("IBM913", "ISO-8859-3", false),
    ("IBM914", "ISO-8859-4", false),
    ("IBM915", "ISO-8859-5", false),
    ("IBM916", "ISO-8859-8", false),
    ("IBM918", "IBM918", false),
    ("IBM920", "ISO-8859-9", false),
    ("IBM921", "x-IBM921", false),
    ("IBM922", "x-IBM922", false),
    ("IBM923", "ISO-8859-15", false),
    ("IBM930", "x-IBM930", false),
    ("IBM932", "x-IBM942C", false),
    ("IBM933", "x-IBM933", false),
    ("IBM935", "x-IBM935", false),
    ("IBM937", "x-IBM937", false),
    ("IBM939", "x-IBM939", false),
    ("IBM942", "x-IBM942", false),
    ("IBM942C", "x-IBM942C", false),
    ("IBM943", "x-IBM943", false),
    ("IBM943C", "x-IBM943C", false),
    ("IBM948", "x-IBM948", false),
    ("IBM949", "x-IBM949", false),
    ("IBM949C", "x-IBM949C", false),
    ("IBM950", "x-IBM950", false),
    ("IBM964", "x-IBM964", false),
    ("IBM970", "x-IBM970", false),
    ("IBMEUCCN", "x-IBM1383", false),
    ("ISCII", "x-ISCII91", false),
    ("ISCII91", "x-ISCII91", false),
    ("ISO-10646-UCS-2", "UTF-16BE", false),
    ("ISO-2022-CN", "ISO-2022-CN", false),
    ("ISO-2022-CN-CNS", "x-ISO-2022-CN-CNS", false),
    ("ISO-2022-CN-GB", "x-ISO-2022-CN-GB", false),
    ("ISO-2022-JP", "ISO-2022-JP", false),
    ("ISO-2022-JP-2", "ISO-2022-JP-2", false),
    ("ISO-2022-KR", "ISO-2022-KR", false),
    ("ISO-8859-1", "ISO-8859-1", false),
    ("ISO-8859-11", "x-iso-8859-11", false),
    ("ISO-8859-13", "ISO-8859-13", false),
    ("ISO-8859-15", "ISO-8859-15", false),
    ("ISO-8859-16", "ISO-8859-16", false),
    ("ISO-8859-2", "ISO-8859-2", false),
    ("ISO-8859-3", "ISO-8859-3", false),
    ("ISO-8859-4", "ISO-8859-4", false),
    ("ISO-8859-5", "ISO-8859-5", false),
    ("ISO-8859-6", "ISO-8859-6", false),
    ("ISO-8859-7", "ISO-8859-7", false),
    ("ISO-8859-8", "ISO-8859-8", false),
    ("ISO-8859-8-I", "ISO-8859-8", false),
    ("ISO-8859-9", "ISO-8859-9", false),
    ("ISO-IR-100", "ISO-8859-1", false),
    ("ISO-IR-101", "ISO-8859-2", false),
    ("ISO-IR-109", "ISO-8859-3", false),
    ("ISO-IR-110", "ISO-8859-4", false),
    ("ISO-IR-126", "ISO-8859-7", false),
    ("ISO-IR-127", "ISO-8859-6", false),
    ("ISO-IR-138", "ISO-8859-8", false),
    ("ISO-IR-144", "ISO-8859-5", false),
    ("ISO-IR-148", "ISO-8859-9", false),
    ("ISO-IR-149", "EUC-KR", false),
    ("ISO-IR-153", "x-ISCII91", false),
    ("ISO-IR-159", "JIS_X0212-1990", false),
    ("ISO-IR-226", "ISO-8859-16", false),
    ("ISO-IR-6", "US-ASCII", true),
    ("ISO-IR-87", "x-JIS0208", false),
    ("ISO2022CN", "ISO-2022-CN", false),
    ("ISO2022CN_CNS", "x-ISO-2022-CN-CNS", false),
    ("ISO2022CN_GB", "x-ISO-2022-CN-GB", false),
    ("ISO2022JP", "ISO-2022-JP", false),
    ("ISO2022JP2", "ISO-2022-JP-2", false),
    ("ISO2022KR", "ISO-2022-KR", false),
    ("ISO646-US", "US-ASCII", true),
    ("ISO8859-1", "ISO-8859-1", false),
    ("ISO8859-13", "ISO-8859-13", false),
    ("ISO8859-15", "ISO-8859-15", false),
    ("ISO8859-2", "ISO-8859-2", false),
    ("ISO8859-3", "ISO-8859-3", false),
    ("ISO8859-4", "ISO-8859-4", false),
    ("ISO8859-5", "ISO-8859-5", false),
    ("ISO8859-6", "ISO-8859-6", false),
    ("ISO8859-7", "ISO-8859-7", false),
    ("ISO8859-8", "ISO-8859-8", false),
    ("ISO8859-9", "ISO-8859-9", false),
    ("ISO8859_1", "ISO-8859-1", false),
    ("ISO8859_11", "x-iso-8859-11", false),
    ("ISO8859_13", "ISO-8859-13", false),
    ("ISO8859_15", "ISO-8859-15", false),
    ("ISO8859_15_FDIS", "ISO-8859-15", false),
    ("ISO8859_16", "ISO-8859-16", false),
    ("ISO8859_2", "ISO-8859-2", false),
    ("ISO8859_3", "ISO-8859-3", false),
    ("ISO8859_4", "ISO-8859-4", false),
    ("ISO8859_5", "ISO-8859-5", false),
    ("ISO8859_6", "ISO-8859-6", false),
    ("ISO8859_7", "ISO-8859-7", false),
    ("ISO8859_8", "ISO-8859-8", false),
    ("ISO8859_9", "ISO-8859-9", false),
    ("ISO_646.IRV:1983", "US-ASCII", false),
    ("ISO_646.IRV:1991", "US-ASCII", true),
    ("ISO_8859-1", "ISO-8859-1", false),
    ("ISO_8859-13", "ISO-8859-13", false),
    ("ISO_8859-15", "ISO-8859-15", false),
    ("ISO_8859-16", "ISO-8859-16", false),
    ("ISO_8859-16:2001", "ISO-8859-16", false),
    ("ISO_8859-1:1987", "ISO-8859-1", false),
    ("ISO_8859-2", "ISO-8859-2", false),
    ("ISO_8859-2:1987", "ISO-8859-2", false),
    ("ISO_8859-3", "ISO-8859-3", false),
    ("ISO_8859-3:1988", "ISO-8859-3", false),
    ("ISO_8859-4", "ISO-8859-4", false),
    ("ISO_8859-4:1988", "ISO-8859-4", false),
    ("ISO_8859-5", "ISO-8859-5", false),
    ("ISO_8859-5:1988", "ISO-8859-5", false),
    ("ISO_8859-6", "ISO-8859-6", false),
    ("ISO_8859-6:1987", "ISO-8859-6", false),
    ("ISO_8859-7", "ISO-8859-7", false),
    ("ISO_8859-7:1987", "ISO-8859-7", false),
    ("ISO_8859-8", "ISO-8859-8", false),
    ("ISO_8859-8:1988", "ISO-8859-8", false),
    ("ISO_8859-9", "ISO-8859-9", false),
    ("ISO_8859-9:1989", "ISO-8859-9", false),
    ("ISO_8859_1", "ISO-8859-1", false),
    ("JIS", "ISO-2022-JP", false),
    ("JIS0201", "JIS_X0201", false),
    ("JIS0208", "x-JIS0208", false),
    ("JIS0212", "JIS_X0212-1990", false),
    ("JISAUTODETECT", "x-JISAutoDetect", false),
    ("JIS_C6226-1983", "x-JIS0208", false),
    ("JIS_ENCODING", "ISO-2022-JP", false),
    ("JIS_X0201", "JIS_X0201", false),
    ("JIS_X0208-1983", "x-JIS0208", false),
    ("JIS_X0212-1990", "JIS_X0212-1990", false),
    ("JOHAB", "x-Johab", false),
    ("KOI8", "KOI8-R", false),
    ("KOI8-R", "KOI8-R", false),
    ("KOI8-U", "KOI8-U", false),
    ("KOI8_R", "KOI8-R", false),
    ("KOI8_U", "KOI8-U", false),
    ("KOREAN", "EUC-KR", false),
    ("KSC5601", "EUC-KR", false),
    ("KSC5601-1987", "EUC-KR", false),
    ("KSC5601-1992", "x-Johab", false),
    ("KSC5601_1987", "EUC-KR", false),
    ("KSC5601_1992", "x-Johab", false),
    ("KSC_5601", "EUC-KR", false),
    ("KS_C_5601-1987", "EUC-KR", false),
    ("KS_C_5601-1989", "EUC-KR", false),
    ("L1", "ISO-8859-1", false),
    ("L10", "ISO-8859-16", false),
    ("L2", "ISO-8859-2", false),
    ("L3", "ISO-8859-3", false),
    ("L4", "ISO-8859-4", false),
    ("L5", "ISO-8859-9", false),
    ("L9", "ISO-8859-15", false),
    ("LATIN-9", "ISO-8859-15", false),
    ("LATIN0", "ISO-8859-15", false),
    ("LATIN1", "ISO-8859-1", false),
    ("LATIN10", "ISO-8859-16", false),
    ("LATIN2", "ISO-8859-2", false),
    ("LATIN3", "ISO-8859-3", false),
    ("LATIN4", "ISO-8859-4", false),
    ("LATIN5", "ISO-8859-9", false),
    ("LATIN9", "ISO-8859-15", false),
    ("MACARABIC", "x-MacArabic", false),
    ("MACCENTRALEUROPE", "x-MacCentralEurope", false),
    ("MACCROATIAN", "x-MacCroatian", false),
    ("MACCYRILLIC", "x-MacCyrillic", false),
    ("MACDINGBAT", "x-MacDingbat", false),
    ("MACGREEK", "x-MacGreek", false),
    ("MACHEBREW", "x-MacHebrew", false),
    ("MACICELAND", "x-MacIceland", false),
    ("MACROMAN", "x-MacRoman", false),
    ("MACROMANIA", "x-MacRomania", false),
    ("MACSYMBOL", "x-MacSymbol", false),
    ("MACTHAI", "x-MacThai", false),
    ("MACTURKISH", "x-MacTurkish", false),
    ("MACUKRAINE", "x-MacUkraine", false),
    ("MS-874", "x-windows-874", false),
    ("MS1361", "x-Johab", false),
    ("MS50220", "x-windows-50220", false),
    ("MS50221", "x-windows-50221", false),
    ("MS874", "x-windows-874", false),
    ("MS932", "windows-31j", false),
    ("MS932-0213", "x-MS932_0213", false),
    ("MS932:2004", "x-MS932_0213", false),
    ("MS932_0213", "x-MS932_0213", false),
    ("MS936", "GBK", false),
    ("MS949", "x-windows-949", false),
    ("MS950", "x-windows-950", false),
    ("MS950_HKSCS", "x-MS950-HKSCS", false),
    ("MS950_HKSCS_XP", "x-MS950-HKSCS-XP", false),
    ("MS_936", "x-mswin-936", false),
    ("MS_949", "x-windows-949", false),
    ("MS_KANJI", "Shift_JIS", false),
    ("PC-MULTILINGUAL-850+EURO", "IBM00858", false),
    ("PCK", "x-PCK", false),
    ("SHIFT-JIS", "Shift_JIS", false),
    ("SHIFT_JIS", "Shift_JIS", false),
    ("SHIFT_JIS:2004", "x-SJIS_0213", false),
    ("SHIFT_JIS_0213:2004", "x-SJIS_0213", false),
    ("SJIS", "Shift_JIS", false),
    ("SJIS-0213", "x-SJIS_0213", false),
    ("SJIS:2004", "x-SJIS_0213", false),
    ("SJIS_0213", "x-SJIS_0213", false),
    ("SJIS_0213:2004", "x-SJIS_0213", false),
    ("ST_SEV_358-88", "x-ISCII91", false),
    ("SUN_EU_GREEK", "ISO-8859-7", false),
    ("TIS-620", "TIS-620", false),
    ("TIS620", "TIS-620", false),
    ("TIS620.2533", "TIS-620", false),
    ("UNICODE", "UTF-16", false),
    ("UNICODE-1-1-UTF-8", "UTF-8", false),
    ("UNICODEBIG", "UTF-16", false),
    ("UNICODEBIGUNMARKED", "UTF-16BE", false),
    ("UNICODELITTLE", "x-UTF-16LE-BOM", false),
    ("UNICODELITTLEUNMARKED", "UTF-16LE", false),
    ("US", "US-ASCII", true),
    ("US-ASCII", "US-ASCII", true),
    ("UTF-16", "UTF-16", false),
    ("UTF-16BE", "UTF-16", false),
    ("UTF-16LE", "x-UTF-16LE-BOM", false),
    ("UTF-32", "UTF-32", false),
    ("UTF-32BE", "UTF-32BE", false),
    ("UTF-32BE-BOM", "X-UTF-32BE-BOM", false),
    ("UTF-32LE", "UTF-32LE", false),
    ("UTF-32LE-BOM", "X-UTF-32LE-BOM", false),
    ("UTF-8", "UTF-8", false),
    ("UTF16", "UTF-16", false),
    ("UTF32", "UTF-32", false),
    ("UTF8", "UTF-8", false),
    ("UTF_16", "UTF-16", false),
    ("UTF_16BE", "UTF-16BE", false),
    ("UTF_16LE", "UTF-16LE", false),
    ("UTF_32", "UTF-32", false),
    ("UTF_32BE", "UTF-32BE", false),
    ("UTF_32BE_BOM", "X-UTF-32BE-BOM", false),
    ("UTF_32LE", "UTF-32LE", false),
    ("UTF_32LE_BOM", "X-UTF-32LE-BOM", false),
    ("WINDOWS-1250", "windows-1250", false),
    ("WINDOWS-1251", "windows-1251", false),
    ("WINDOWS-1252", "windows-1252", false),
    ("WINDOWS-1253", "windows-1253", false),
    ("WINDOWS-1254", "windows-1254", false),
    ("WINDOWS-1255", "windows-1255", false),
    ("WINDOWS-1256", "windows-1256", false),
    ("WINDOWS-1257", "windows-1257", false),
    ("WINDOWS-1258", "windows-1258", false),
    ("WINDOWS-31J", "windows-31j", false),
    ("WINDOWS-437", "IBM437", false),
    ("WINDOWS-874", "x-windows-874", false),
    ("WINDOWS-932", "windows-31j", false),
    ("WINDOWS-932-0213", "x-MS932_0213", false),
    ("WINDOWS-932:2004", "x-MS932_0213", false),
    ("WINDOWS-936", "GBK", false),
    ("WINDOWS-949", "x-windows-949", false),
    ("WINDOWS-950", "x-windows-950", false),
    ("WINDOWS-ISO2022JP", "x-windows-iso2022jp", false),
    ("WINDOWS949", "x-windows-949", false),
    ("X-BIG5-HKSCS-2001", "x-Big5-HKSCS-2001", false),
    ("X-BIG5-SOLARIS", "x-Big5-Solaris", false),
    ("X-EUC-CN", "GB2312", false),
    ("X-EUC-JP", "EUC-JP", false),
    ("X-EUC-JP-LINUX", "x-euc-jp-linux", false),
    ("X-EUC-TW", "x-EUC-TW", false),
    ("X-EUCJP", "EUC-JP", false),
    ("X-EUCJP-OPEN", "x-eucJP-Open", false),
    ("X-IBM1006", "x-IBM1006", false),
    ("X-IBM1025", "x-IBM1025", false),
    ("X-IBM1046", "x-IBM1046", false),
    ("X-IBM1097", "x-IBM1097", false),
    ("X-IBM1098", "x-IBM1098", false),
    ("X-IBM1112", "x-IBM1112", false),
    ("X-IBM1122", "x-IBM1122", false),
    ("X-IBM1123", "x-IBM1123", false),
    ("X-IBM1124", "x-IBM1124", false),
    ("X-IBM1129", "x-IBM1129", false),
    ("X-IBM1166", "x-IBM1166", false),
    ("X-IBM1364", "x-IBM1364", false),
    ("X-IBM1381", "x-IBM1381", false),
    ("X-IBM1383", "x-IBM1383", false),
    ("X-IBM29626C", "x-IBM29626C", false),
    ("X-IBM300", "x-IBM300", false),
    ("X-IBM33722", "x-IBM33722", false),
    ("X-IBM737", "x-IBM737", false),
    ("X-IBM833", "x-IBM833", false),
    ("X-IBM834", "x-IBM834", false),
    ("X-IBM856", "x-IBM856", false),
    ("X-IBM874", "x-IBM874", false),
    ("X-IBM875", "x-IBM875", false),
    ("X-IBM921", "x-IBM921", false),
    ("X-IBM922", "x-IBM922", false),
    ("X-IBM930", "x-IBM930", false),
    ("X-IBM932", "x-IBM942C", false),
    ("X-IBM933", "x-IBM933", false),
    ("X-IBM935", "x-IBM935", false),
    ("X-IBM937", "x-IBM937", false),
    ("X-IBM939", "x-IBM939", false),
    ("X-IBM942", "x-IBM942", false),
    ("X-IBM942C", "x-IBM942C", false),
    ("X-IBM943", "x-IBM943", false),
    ("X-IBM943C", "x-IBM943C", false),
    ("X-IBM948", "x-IBM948", false),
    ("X-IBM949", "x-IBM949", false),
    ("X-IBM949C", "x-IBM949C", false),
    ("X-IBM950", "x-IBM950", false),
    ("X-IBM964", "x-IBM964", false),
    ("X-IBM970", "x-IBM970", false),
    ("X-ISCII91", "x-ISCII91", false),
    ("X-ISO-2022-CN-CNS", "x-ISO-2022-CN-CNS", false),
    ("X-ISO-2022-CN-GB", "x-ISO-2022-CN-GB", false),
    ("X-ISO-8859-11", "x-iso-8859-11", false),
    ("X-JIS0208", "x-JIS0208", false),
    ("X-JISAUTODETECT", "x-JISAutoDetect", false),
    ("X-JOHAB", "x-Johab", false),
    ("X-MACARABIC", "x-MacArabic", false),
    ("X-MACCENTRALEUROPE", "x-MacCentralEurope", false),
    ("X-MACCROATIAN", "x-MacCroatian", false),
    ("X-MACCYRILLIC", "x-MacCyrillic", false),
    ("X-MACDINGBAT", "x-MacDingbat", false),
    ("X-MACGREEK", "x-MacGreek", false),
    ("X-MACHEBREW", "x-MacHebrew", false),
    ("X-MACICELAND", "x-MacIceland", false),
    ("X-MACROMAN", "x-MacRoman", false),
    ("X-MACROMANIA", "x-MacRomania", false),
    ("X-MACSYMBOL", "x-MacSymbol", false),
    ("X-MACTHAI", "x-MacThai", false),
    ("X-MACTURKISH", "x-MacTurkish", false),
    ("X-MACUKRAINE", "x-MacUkraine", false),
    ("X-MS932_0213", "x-MS932_0213", false),
    ("X-MS950-HKSCS", "x-MS950-HKSCS", false),
    ("X-MS950-HKSCS-XP", "x-MS950-HKSCS-XP", false),
    ("X-MSWIN-936", "x-mswin-936", false),
    ("X-PCK", "x-PCK", false),
    ("X-SJIS", "Shift_JIS", false),
    ("X-SJIS_0213", "x-SJIS_0213", false),
    ("X-UTF-16BE", "UTF-16BE", false),
    ("X-UTF-16LE", "UTF-16LE", false),
    ("X-UTF-16LE-BOM", "x-UTF-16LE-BOM", false),
    ("X-UTF-32BE", "UTF-32BE", false),
    ("X-UTF-32BE-BOM", "X-UTF-32BE-BOM", false),
    ("X-UTF-32LE", "UTF-32LE", false),
    ("X-UTF-32LE-BOM", "X-UTF-32LE-BOM", false),
    ("X-WINDOWS-50220", "x-windows-50220", false),
    ("X-WINDOWS-50221", "x-windows-50221", false),
    ("X-WINDOWS-874", "x-windows-874", false),
    ("X-WINDOWS-949", "x-windows-949", false),
    ("X-WINDOWS-950", "x-windows-950", false),
    ("X-WINDOWS-ISO2022JP", "x-windows-iso2022jp", false),
    ("X0201", "JIS_X0201", false),
    ("X0208", "x-JIS0208", false),
    ("X0212", "JIS_X0212-1990", false),
];

#[path = "xml_charset_data.rs"]
mod data;

struct Output {
    text: String,
    high: Option<u16>,
}

impl Output {
    fn new() -> Self { Self { text: String::new(), high: None } }

    fn scalar(&mut self, value: char) -> Result<(), XmlError> {
        if self.high.is_some() {
            return Err(XmlError::Encoding("Legacy decoder produced an XML surrogate scalar".to_owned()));
        }
        if self.text.len() + value.len_utf8() > MAX_DOCUMENT_BYTES {
            return Err(XmlError::ResourceLimit { resource: "decoded bytes", maximum: MAX_DOCUMENT_BYTES });
        }
        self.text.push(value);
        Ok(())
    }

    fn unit(&mut self, value: u16) -> Result<(), XmlError> {
        if let Some(high) = self.high.take() {
            if !(0xdc00..=0xdfff).contains(&value) {
                return Err(XmlError::Encoding("Legacy decoder produced an XML surrogate scalar".to_owned()));
            }
            self.scalar(char::from_u32(0x10000 + ((u32::from(high) - 0xd800) << 10) + u32::from(value) - 0xdc00).unwrap())
        } else if (0xd800..=0xdbff).contains(&value) {
            self.high = Some(value);
            Ok(())
        } else {
            self.scalar(char::from_u32(u32::from(value)).ok_or_else(|| XmlError::Encoding("Legacy decoder produced an XML surrogate scalar".to_owned()))?)
        }
    }

    fn finish(self) -> Result<String, XmlError> {
        if self.high.is_some() { return Err(XmlError::Encoding("Legacy decoder ended with an XML surrogate scalar".to_owned())); }
        Ok(self.text)
    }
}

#[derive(Clone, Copy)]
struct Mapping {
    result: u8,
    consumed: usize,
    error_length: usize,
    count: usize,
    units: [u16; 4],
}

#[derive(Clone, Copy)]
struct Table {
    indices: &'static [u8],
    count: usize,
}

impl Table {
    fn get(name: &str, profile: &str, eof: bool) -> Result<Self, XmlError> {
        let leaf = data::table(name, profile, eof).ok_or_else(|| missing(name, profile))?;
        let count = match leaf.domain { 1 => 256, 2 => 65536, 3 => 94 * 94, 4 => 128 * 128, _ => panic!("Static charset dense domain") };
        assert_eq!(leaf.count, count, "Static charset domain length");
        assert_eq!(leaf.indices.len(), count * 4, "Static charset index length");
        assert_eq!(data::pool().len() % 12, 0, "Static charset result pool width");
        Ok(Self { indices: leaf.indices, count })
    }
}

impl Mapping {
    fn read(table: Table, index: usize) -> Self {
        assert!(index < table.count, "Static charset domain index");
        let row = u32::from_le_bytes(table.indices[index * 4..(index + 1) * 4].try_into().unwrap()) as usize;
        let raw = &data::pool()[row * 12..(row + 1) * 12];
        let units = std::array::from_fn(|position| u16::from_le_bytes([raw[4 + position * 2], raw[5 + position * 2]]));
        assert!(raw[3] <= 4 && raw[0] & 3 <= 2, "Static charset record shape");
        Self { result: raw[0] & 3, consumed: usize::from(raw[1]), error_length: usize::from(raw[2]), count: usize::from(raw[3]), units }
    }

    fn emit(self, output: &mut Output) -> Result<(), XmlError> {
        for value in &self.units[..self.count] { output.unit(*value)?; }
        Ok(())
    }
}

fn missing(name: &str, profile: &str) -> XmlError {
    XmlError::Encoding(format!("No qualified decoder data for {name}/{profile}"))
}


fn decode_stateless(name: &str, bytes: &[u8], pairs: bool, output: &mut Output) -> Result<(), XmlError> {
    let byte = [Table::get(name, "single", false)?, Table::get(name, "single", true)?];
    let pair = if pairs { Some([Table::get(name, "pair", false)?, Table::get(name, "pair", true)?]) } else { None };
    let mut position = 0;
    while position < bytes.len() {
        let remaining = bytes.len() - position;
        let width = if pairs && remaining >= 2 { 2 } else { 1 };
        let index = if width == 2 { usize::from(u16::from_be_bytes([bytes[position], bytes[position + 1]])) } else { usize::from(bytes[position]) };
        let table = if width == 2 { pair.unwrap() } else { byte };
        let entry = Mapping::read(table[usize::from(remaining == width)], index);
        entry.emit(output)?;
        let mut consumed = entry.consumed;
        if entry.result != 0 {
            assert!(entry.error_length > 0, "Static charset error progress");
            output.scalar('\u{fffd}')?;
            consumed += entry.error_length;
        }
        assert!(consumed > 0 && consumed <= width, "Static charset input progress");
        position += consumed;
    }
    Ok(())
}

// ISCII91 state rules and direct map adapted from OpenJDK 21u eac07229
// Copyright (c) 2002, 2021, Oracle and/or its affiliates
const ISCII_DIRECT: [u16; 255] = [
    0x0000, 0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006, 0x0007, 0x0008, 0x0009, 0x000a, 0x000b,
    0x000c, 0x000d, 0x000e, 0x000f, 0x0010, 0x0011, 0x0012, 0x0013, 0x0014, 0x0015, 0x0016, 0x0017,
    0x0018, 0x0019, 0x001a, 0x001b, 0x001c, 0x001d, 0x001e, 0x001f, 0x0020, 0x0021, 0x0022, 0x0023,
    0x0024, 0x0025, 0x0026, 0x0027, 0x0028, 0x0029, 0x002a, 0x002b, 0x002c, 0x002d, 0x002e, 0x002f,
    0x0030, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x0038, 0x0039, 0x003a, 0x003b,
    0x003c, 0x003d, 0x003e, 0x003f, 0x0040, 0x0041, 0x0042, 0x0043, 0x0044, 0x0045, 0x0046, 0x0047,
    0x0048, 0x0049, 0x004a, 0x004b, 0x004c, 0x004d, 0x004e, 0x004f, 0x0050, 0x0051, 0x0052, 0x0053,
    0x0054, 0x0055, 0x0056, 0x0057, 0x0058, 0x0059, 0x005a, 0x005b, 0x005c, 0x005d, 0x005e, 0x005f,
    0x0060, 0x0061, 0x0062, 0x0063, 0x0064, 0x0065, 0x0066, 0x0067, 0x0068, 0x0069, 0x006a, 0x006b,
    0x006c, 0x006d, 0x006e, 0x006f, 0x0070, 0x0071, 0x0072, 0x0073, 0x0074, 0x0075, 0x0076, 0x0077,
    0x0078, 0x0079, 0x007a, 0x007b, 0x007c, 0x007d, 0x007e, 0x007f, 0xffff, 0xffff, 0xffff, 0xffff,
    0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff,
    0xffff, 0xffff, 0xffff, 0xffff, 0x0901, 0x0902, 0x0903, 0x0905, 0x0906, 0x0907, 0x0908, 0x0909,
    0x090a, 0x090b, 0x090e, 0x090f, 0x0910, 0x090d, 0x0912, 0x0913, 0x0914, 0x0911, 0x0915, 0x0916,
    0x0917, 0x0918, 0x0919, 0x091a, 0x091b, 0x091c, 0x091d, 0x091e, 0x091f, 0x0920, 0x0921, 0x0922,
    0x0923, 0x0924, 0x0925, 0x0926, 0x0927, 0x0928, 0x0929, 0x092a, 0x092b, 0x092c, 0x092d, 0x092e,
    0x092f, 0x095f, 0x0930, 0x0931, 0x0932, 0x0933, 0x0934, 0x0935, 0x0936, 0x0937, 0x0938, 0x0939,
    0x200d, 0x093e, 0x093f, 0x0940, 0x0941, 0x0942, 0x0943, 0x0946, 0x0947, 0x0948, 0x0945, 0x094a,
    0x094b, 0x094c, 0x0949, 0x094d, 0x093c, 0x0964, 0xffff, 0xffff, 0xffff, 0xffff, 0xfffd, 0xfffd,
    0x0966, 0x0967, 0x0968, 0x0969, 0x096a, 0x096b, 0x096c, 0x096d, 0x096e, 0x096f, 0xffff, 0xffff,
    0xffff, 0xffff, 0xffff,
];

fn decode_iscii(bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let mut context = 0xffff;
    let mut pending = false;
    for byte in bytes {
        let current = ISCII_DIRECT[usize::from(if *byte >= 128 { *byte - 1 } else { *byte })];
        if context == 0xfffd {
            output.unit(0xfffd)?;
            context = 0xffff;
            pending = false;
            continue;
        }
        match current {
            0x0901 | 0x0907 | 0x0908 | 0x090b | 0x093f | 0x0940 | 0x0943 | 0x0964 => {
                if pending { output.unit(context)?; }
                context = current;
                pending = true;
                continue;
            }
            0x093c => {
                let composed = match context {
                    0x0901 => Some(0x0950),
                    0x0907 => Some(0x090c),
                    0x0908 => Some(0x0961),
                    0x090b => Some(0x0960),
                    0x093f => Some(0x0962),
                    0x0940 => Some(0x0963),
                    0x0943 => Some(0x0944),
                    0x0964 => Some(0x093d),
                    _ => None,
                };
                if let Some(value) = composed { output.unit(value)?; }
                else if pending {
                    output.unit(context)?;
                    context = current;
                    continue;
                } else {
                    output.unit(if context == 0x094d { 0x200d } else { current })?;
                }
            }
            0x094d => {
                if pending {
                    output.unit(context)?;
                    context = current;
                    continue;
                }
                output.unit(if context == 0x094d { 0x200c } else { current })?;
            }
            0xffff => {
                if pending {
                    output.unit(context)?;
                    context = current;
                    continue;
                }
                output.unit(0xfffd)?;
                continue;
            }
            _ => {
                if pending {
                    output.unit(context)?;
                    context = current;
                    continue;
                }
                output.unit(current)?;
            }
        }
        context = current;
        pending = false;
    }
    if pending { output.unit(context)?; }
    Ok(())
}

fn plane(table: Table, first: u8, second: u8, output: &mut Output) -> Result<(), XmlError> {
    let first = first & 0x7f;
    let second = second & 0x7f;
    if !(0x21..=0x7e).contains(&first) || !(0x21..=0x7e).contains(&second) {
        return output.scalar('\u{fffd}');
    }
    assert_eq!(table.count, 128 * 128, "Static ISO2022 plane domain");
    let entry = Mapping::read(table, (usize::from(first) << 7) | usize::from(second));
    entry.emit(output)?;
    if entry.result != 0 { output.scalar('\u{fffd}')?; }
    Ok(())
}

#[derive(Clone, Copy)]
enum JapaneseState { Ascii, Roman, Kana, Pair0208, Pair0212, Shift }

fn direct(value: u16, consumed: usize) -> Mapping {
    Mapping { result: 0, consumed, error_length: 0, count: 1, units: [value, 0, 0, 0] }
}

fn empty(consumed: usize) -> Mapping {
    Mapping { result: 0, consumed, error_length: 0, count: 0, units: [0; 4] }
}

fn invalid(length: usize) -> Mapping {
    Mapping { result: 1, consumed: 0, error_length: length, count: 0, units: [0; 4] }
}

fn iso_pair(table: Table, first: u8, second: u8) -> Mapping {
    if !(0x21..=0x7e).contains(&first) || !(0x21..=0x7e).contains(&second) { return invalid(2); }
    let mut value = Mapping::read(table, (usize::from(first) << 7) | usize::from(second));
    if value.result == 0 { value.consumed = 2; }
    else { value.consumed = 0; value.error_length = 2; }
    value
}

struct Japanese {
    state: JapaneseState,
    previous: JapaneseState,
    jis0208: Table,
    jis0212: Option<Table>,
}

impl Japanese {
    fn new(name: &str) -> Result<Self, XmlError> {
        let jis0208 = Table::get(name, "jp0208", true)?;
        let jis0212 = if matches!(name, "ISO-2022-JP-2" | "x-windows-50220" | "x-windows-50221") {
            Some(Table::get(name, "jp0212", true)?)
        } else { None };
        Ok(Self { state: JapaneseState::Ascii, previous: JapaneseState::Ascii, jis0208, jis0212 })
    }

    fn step(&mut self, bytes: &[u8]) -> Mapping {
        let first = bytes[0];
        if first >= 128 { return invalid(1); }
        if first == 0x1b {
            if bytes.len() < 3 { return empty(0); }
            let mut length = 3;
            let next = match (bytes[1], bytes[2]) {
                (b'(', b'B') => Some(JapaneseState::Ascii),
                (b'(', b'J') => Some(JapaneseState::Roman),
                (b'(', b'I') => Some(JapaneseState::Kana),
                (b'$', b'@' | b'B') => Some(JapaneseState::Pair0208),
                (b'$', b'(') if self.jis0212.is_some() => {
                    if bytes.len() < 4 { return empty(0); }
                    length = 4;
                    (bytes[3] == b'D').then_some(JapaneseState::Pair0212)
                }
                _ => {
                    if bytes[1] != b'(' && bytes[1] != b'$' { length = 2; }
                    None
                }
            };
            if let Some(next) = next { self.state = next; return empty(length); }
            return invalid(length);
        }
        if first == 0x0e { self.previous = self.state; self.state = JapaneseState::Shift; return empty(1); }
        if first == 0x0f { self.state = self.previous; return empty(1); }
        match self.state {
            JapaneseState::Ascii => direct(u16::from(first), 1),
            JapaneseState::Roman => direct(match first { 0x5c => 0x00a5, 0x7e => 0x203e, _ => u16::from(first) }, 1),
            JapaneseState::Kana | JapaneseState::Shift => {
                if first <= 0x5f { direct(u16::from(first) + 0xff40, 1) } else { invalid(1) }
            }
            JapaneseState::Pair0208 | JapaneseState::Pair0212 => {
                if bytes.len() < 2 { return empty(0); }
                iso_pair(if matches!(self.state, JapaneseState::Pair0212) { self.jis0212.unwrap() } else { self.jis0208 }, first, bytes[1])
            }
        }
    }
}

fn apply(value: Mapping, remaining: usize, output: &mut Output) -> Result<usize, XmlError> {
    value.emit(output)?;
    if value.result != 0 {
        assert!(value.error_length > 0 && value.consumed + value.error_length <= remaining, "Charset state error progress");
        output.scalar('\u{fffd}')?;
        Ok(value.consumed + value.error_length)
    } else if value.consumed == 0 {
        output.scalar('\u{fffd}')?;
        Ok(remaining)
    } else {
        assert!(value.consumed <= remaining, "Charset state input progress");
        Ok(value.consumed)
    }
}

fn decode_iso_jp(name: &str, bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let mut state = Japanese::new(name)?;
    let mut position = 0;
    while position < bytes.len() {
        position += apply(state.step(&bytes[position..]), bytes.len() - position, output)?;
    }
    Ok(())
}

fn decode_iso_cn(bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let gb = Table::get("ISO-2022-CN", "cn-gb", true)?;
    let cns = [Table::get("ISO-2022-CN", "cn-cns1", true)?, Table::get("ISO-2022-CN", "cn-cns2", true)?, Table::get("ISO-2022-CN", "cn-cns3", true)?];
    let mut shift = false;
    let mut designated_cns = false;
    let mut position = 0;
    while position < bytes.len() {
        match bytes[position] {
            0x0e => { shift = true; position += 1; }
            0x0f => { shift = false; position += 1; }
            0x1b => {
                designated_cns = false;
                let remaining = &bytes[position..];
                if remaining.len() < 2 { output.scalar('\u{fffd}')?; break; }
                let second = remaining[1];
                if second >= 128 { output.scalar('\u{fffd}')?; position += 2; continue; }
                if second == b'$' {
                    if remaining.len() < 3 { output.scalar('\u{fffd}')?; break; }
                    let third = remaining[2];
                    if third >= 128 { output.scalar('\u{fffd}')?; position += 3; continue; }
                    if third == b'A' { position += 3; continue; }
                    if matches!(third, b')' | b'*' | b'+') {
                        if remaining.len() < 4 { output.scalar('\u{fffd}')?; break; }
                        let fourth = remaining[3];
                        match (third, fourth) {
                            (b')', b'A') | (b'*', b'H') | (b'+', b'I') => {}
                            (b')', b'G') => designated_cns = true,
                            _ => output.scalar('\u{fffd}')?,
                        }
                        position += 4;
                    } else { output.scalar('\u{fffd}')?; position += 3; }
                } else if matches!(second, b'N' | b'O') {
                    if remaining.len() < 4 { output.scalar('\u{fffd}')?; break; }
                    plane(cns[if second == b'N' { 1 } else { 2 }], remaining[2], remaining[3], output)?;
                    position += 4;
                } else { output.scalar('\u{fffd}')?; position += 2; }
            }
            first => {
                if !shift { output.scalar(char::from(first))?; position += 1; }
                else {
                    if position + 1 == bytes.len() { output.scalar('\u{fffd}')?; break; }
                    plane(if designated_cns { cns[0] } else { gb }, first, bytes[position + 1], output)?;
                    position += 2;
                }
            }
        }
    }
    Ok(())
}

fn decode_iso_kr(bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let table = Table::get("ISO-2022-KR", "kr-so", true)?;
    let mut shift = false;
    let mut position = 0;
    while position < bytes.len() {
        let first = bytes[position];
        match first {
            0x0e => { shift = true; position += 1; }
            0x0f => { shift = false; position += 1; }
            0x1b => {
                let remaining = &bytes[position..];
                if remaining.len() < 4 { output.scalar('\u{fffd}')?; break; }
                if remaining[1..4] == [b'$', b')', b'C'] { position += 4; }
                else if matches!(remaining[1], b'N' | b'O') { output.scalar('\u{fffd}')?; position += 4; }
                else { output.scalar('\u{fffd}')?; position += 2; }
            }
            _ => {
                if !shift { output.scalar(char::from(first))?; position += 1; }
                else {
                    if position + 1 == bytes.len() { output.scalar('\u{fffd}')?; break; }
                    plane(table, first, bytes[position + 1], output)?;
                    position += 2;
                }
            }
        }
    }
    Ok(())
}

fn decode_shift_ebcdic(name: &str, bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let single = Table::get(name, "single", true)?;
    let pair = Table::get(name, "shift_pair", true)?;
    let mut shift = false;
    let mut position = 0;
    while position < bytes.len() {
        let first = bytes[position];
        if first == 0x0e {
            if shift { output.scalar('\u{fffd}')?; }
            else { shift = true; }
            position += 1;
        } else if first == 0x0f {
            if !shift { output.scalar('\u{fffd}')?; }
            else { shift = false; }
            position += 1;
        } else {
            let entry;
            if shift {
                if position + 1 == bytes.len() { output.scalar('\u{fffd}')?; break; }
                entry = Mapping::read(pair, usize::from(u16::from_be_bytes([first, bytes[position + 1]])));
                position += 2;
            } else {
                entry = Mapping::read(single, usize::from(first));
                position += 1;
            }
            entry.emit(output)?;
            if entry.result != 0 { output.scalar('\u{fffd}')?; }
        }
    }
    Ok(())
}

fn unmappable(length: usize) -> Mapping {
    let mut value = invalid(length);
    value.result = 2;
    value
}

fn euc_plane(table: Table, first: u8, second: u8, width: usize) -> Mapping {
    if !(0xa1..=0xfe).contains(&first) || !(0xa1..=0xfe).contains(&second) { return unmappable(width); }
    assert_eq!(table.count, 94 * 94, "Static EUC plane domain");
    let mut value = Mapping::read(table, usize::from(first - 0xa1) * 94 + usize::from(second - 0xa1));
    value.consumed = if value.result == 0 { width } else { 0 };
    value.error_length = if value.result == 0 { 0 } else { width };
    value
}

struct Euc {
    name: &'static str,
    single: Table,
    ss2: Option<Table>,
    jis0208: Option<Table>,
    jis0212: Option<Table>,
    cns: [Option<Table>; 16],
}

impl Euc {
    fn new(name: &'static str) -> Result<Self, XmlError> {
        let single = Table::get(name, "single", false)?;
        let mut value = Self { name, single, ss2: None, jis0208: None, jis0212: None, cns: [None; 16] };
        if matches!(name, "x-EUC-TW" | "x-IBM964") {
            const PROFILES: [&str; 16] = ["cns01", "cns02", "cns03", "cns04", "cns05", "cns06", "cns07", "cns08", "cns09", "cns10", "cns11", "cns12", "cns13", "cns14", "cns15", "cns16"];
            for plane in 1..=16 {
                let needed = plane == 1 || if name == "x-IBM964" { matches!(plane, 2 | 12 | 13) } else { matches!(plane, 2..=7 | 15) };
                if needed { value.cns[plane - 1] = Some(Table::get(name, PROFILES[plane - 1], true)?); }
            }
        } else {
            value.ss2 = Some(Table::get(name, "ss2", true)?);
            value.jis0208 = Some(Table::get(name, "jp0208", true)?);
            value.jis0212 = Some(Table::get(name, "jp0212", true)?);
        }
        Ok(value)
    }

    fn step(&self, bytes: &[u8]) -> Mapping {
        let first = bytes[0];
        let single = Mapping::read(self.single, usize::from(first));
        if single.result != 0 || single.consumed != 0 { return single; }
        if matches!(self.name, "x-EUC-TW" | "x-IBM964") {
            if first == 0x8e {
                if bytes.len() < 4 { return empty(0); }
                let selector = bytes[1];
                let selected = if self.name == "x-IBM964" { matches!(selector, 0xa2 | 0xac | 0xad) } else { matches!(selector, 0xa2..=0xa7 | 0xaf) };
                if !selected { return invalid(2); }
                if !(0xa1..=0xfe).contains(&bytes[2]) {
                    return invalid(if self.name == "x-IBM964" { 3 } else { 4 });
                }
                if !(0xa1..=0xfe).contains(&bytes[3]) { return invalid(4); }
                euc_plane(self.cns[usize::from(selector - 0xa1)].unwrap(), bytes[2], bytes[3], 4)
            } else {
                if bytes.len() < 2 { return empty(0); }
                if !(0xa1..=0xfe).contains(&first) || !(0xa1..=0xfe).contains(&bytes[1]) {
                    return invalid(if self.name == "x-IBM964" { 2 } else { 1 });
                }
                euc_plane(self.cns[0].unwrap(), first, bytes[1], 2)
            }
        } else if first == 0x8f {
            if bytes.len() < 3 { return empty(0); }
            if self.name == "x-IBM33722" {
                if !(0xa1..=0xfe).contains(&bytes[1]) { return invalid(2); }
                if !(0xa1..=0xfe).contains(&bytes[2]) { return invalid(3); }
            }
            euc_plane(self.jis0212.unwrap(), bytes[1], bytes[2], 3)
        } else {
            if bytes.len() < 2 { return empty(0); }
            if first == 0x8e {
                let mut value = Mapping::read(self.ss2.unwrap(), usize::from(bytes[1]));
                value.consumed = if value.result == 0 { 2 } else { 0 };
                value.error_length = if value.result == 0 { 0 } else { 2 };
                return value;
            }
            let value = euc_plane(self.jis0208.unwrap(), first, bytes[1], 2);
            if self.name == "x-IBM33722" && value.result != 0 && (!(0xa1..=0xfe).contains(&first) || !(0xa1..=0xfe).contains(&bytes[1])) { invalid(2) }
            else { value }
        }
    }
}

fn decode_euc(name: &'static str, bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let decoder = Euc::new(name)?;
    let mut position = 0;
    while position < bytes.len() {
        position += apply(decoder.step(&bytes[position..]), bytes.len() - position, output)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Ranges {
    raw: &'static [u8],
    count: usize,
}

impl Ranges {
    fn get(eof: bool) -> Result<Self, XmlError> {
        let raw = data::ranges(eof).ok_or_else(|| missing("GB18030", "four_ranges"))?;
        assert!(raw.len() >= 16 && &raw[..8] == b"BGRCOD01", "Static GB18030 range header");
        let count = u32::from_le_bytes(raw[8..12].try_into().unwrap()) as usize;
        assert_eq!(u16::from_le_bytes(raw[12..14].try_into().unwrap()), 16, "Static GB18030 range width");
        assert_eq!(raw[14], 5, "Static GB18030 domain");
        assert_eq!(raw[15] != 0, eof, "Static GB18030 EOF mode");
        assert_eq!(raw.len(), 16 + count * 16, "Static GB18030 range length");
        Ok(Self { raw: &raw[16..], count })
    }

    fn emit(self, index: u32, output: &mut Output) -> Result<(), XmlError> {
        let mut low = 0;
        let mut high = self.count;
        while low < high {
            let middle = low + (high - low) / 2;
            let range = &self.raw[middle * 16..(middle + 1) * 16];
            let first = u32::from_le_bytes(range[0..4].try_into().unwrap());
            let last = u32::from_le_bytes(range[4..8].try_into().unwrap());
            if index < first { high = middle; }
            else if index > last { low = middle + 1; }
            else {
                if range[12] & 3 != 0 { return output.scalar('\u{fffd}'); }
                let value = u32::from_le_bytes(range[8..12].try_into().unwrap()) + index - first;
                return match range[15] {
                    1 => { assert!(value <= 0xffff, "Static GB18030 UTF16 unit"); output.unit(value as u16) }
                    2 => output.scalar(char::from_u32(value).expect("Static GB18030 supplementary scalar")),
                    _ => panic!("Static GB18030 result width"),
                };
            }
        }
        panic!("Static GB18030 range coverage")
    }
}

fn decode_gb18030(bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let pair = [Table::get("GB18030", "pair", false)?, Table::get("GB18030", "pair", true)?];
    let ranges = [Ranges::get(false)?, Ranges::get(true)?];
    let mut position = 0;
    while position < bytes.len() {
        let remaining = &bytes[position..];
        let first = remaining[0];
        if first < 128 { output.scalar(char::from(first))?; position += 1; }
        else if !(0x81..=0xfe).contains(&first) { output.scalar('\u{fffd}')?; position += 1; }
        else if remaining.len() < 2 { output.scalar('\u{fffd}')?; break; }
        else {
            let second = remaining[1];
            if second < 0x30 { output.scalar('\u{fffd}')?; position += 1; }
            else if (0x30..=0x39).contains(&second) {
                if remaining.len() < 4 { output.scalar('\u{fffd}')?; break; }
                if !(0x81..=0xfe).contains(&remaining[2]) { output.scalar('\u{fffd}')?; position += 3; }
                else if !(0x30..=0x39).contains(&remaining[3]) { output.scalar('\u{fffd}')?; position += 4; }
                else {
                    let index = (u32::from(first - 0x81) * 10 + u32::from(second - 0x30)) * 1260 + u32::from(remaining[2] - 0x81) * 10 + u32::from(remaining[3] - 0x30);
                    ranges[usize::from(remaining.len() == 4)].emit(index, output)?;
                    position += 4;
                }
            } else if second < 0x40 || matches!(second, 0x7f | 0xff) { output.scalar('\u{fffd}')?; position += 2; }
            else {
                let entry = Mapping::read(pair[usize::from(remaining.len() == 2)], usize::from(u16::from_be_bytes([first, second])));
                position += apply(entry, remaining.len(), output)?;
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct Probe {
    consumed: usize,
    invalid: bool,
    hiragana: u8,
    kana: u8,
}

impl Probe {
    fn accept(&mut self, value: Mapping) -> bool {
        if value.result != 0 { self.invalid = true; return false; }
        if value.consumed == 0 { return false; }
        for unit in &value.units[..value.count] {
            if (0x3040..=0x309f).contains(unit) { self.hiragana = self.hiragana.saturating_add(1).min(2); }
            if (0xff65..=0xff9f).contains(unit) { self.kana = self.kana.saturating_add(1).min(2); }
        }
        self.consumed += value.consumed;
        true
    }
}

fn probe_japanese(bytes: &[u8]) -> Result<Probe, XmlError> {
    let mut decoder = Japanese::new("ISO-2022-JP")?;
    let mut probe = Probe::default();
    while probe.consumed < bytes.len() {
        if !probe.accept(decoder.step(&bytes[probe.consumed..])) { break; }
    }
    Ok(probe)
}

fn probe_euc(bytes: &[u8]) -> Result<Probe, XmlError> {
    let decoder = Euc::new("EUC-JP")?;
    let mut probe = Probe::default();
    while probe.consumed < bytes.len() {
        if !probe.accept(decoder.step(&bytes[probe.consumed..])) { break; }
    }
    Ok(probe)
}

fn probe_sjis(bytes: &[u8]) -> Result<Probe, XmlError> {
    let single = Table::get("Shift_JIS", "single", false)?;
    let pair = Table::get("Shift_JIS", "pair", false)?;
    let mut probe = Probe::default();
    while probe.consumed < bytes.len() {
        let tail = &bytes[probe.consumed..];
        let value = if tail.len() >= 2 {
            Mapping::read(pair, usize::from(u16::from_be_bytes([tail[0], tail[1]])))
        } else { Mapping::read(single, usize::from(tail[0])) };
        if !probe.accept(value) { break; }
    }
    Ok(probe)
}

fn decode_auto_japanese(bytes: &[u8], output: &mut Output) -> Result<(), XmlError> {
    let leading = bytes.iter().position(|byte| *byte >= 128 || *byte == 0x1b).unwrap_or(bytes.len());
    for byte in &bytes[..leading] { output.scalar(char::from(*byte))?; }
    if leading == bytes.len() { return Ok(()); }
    let body = if bytes.starts_with(b"<?xml") {
        bytes.windows(2).take(4096).position(|value| value == b"?>").map(|position| position + 2).unwrap_or(0)
    } else { 0 };
    let chunk = body + (leading.saturating_sub(body) / 8192) * 8192;
    let mut end = (chunk + 8192).min(bytes.len());
    if !probe_japanese(&bytes[leading..end])?.invalid {
        return decode_iso_jp("ISO-2022-JP", &bytes[leading..], output);
    }
    let mut euc = probe_euc(&bytes[leading..end])?;
    if euc.invalid { return decode_stateless("Shift_JIS", &bytes[leading..], true, output); }
    let mut sjis = probe_sjis(&bytes[leading..end])?;
    if sjis.invalid { return decode_euc("EUC-JP", &bytes[leading..], output); }
    if euc.consumed == 0 && sjis.consumed == 0 && end < bytes.len() {
        end = (leading + 8192).min(bytes.len());
        euc = probe_euc(&bytes[leading..end])?;
        sjis = probe_sjis(&bytes[leading..end])?;
    }
    let use_euc = if euc.invalid { false }
        else if sjis.invalid || euc.consumed > sjis.consumed { true }
        else if euc.consumed < sjis.consumed { false }
        else { euc.hiragana > 1 || euc.kana > 1 };
    if use_euc { decode_euc("EUC-JP", &bytes[leading..], output) }
    else { decode_stateless("Shift_JIS", &bytes[leading..], true, output) }
}

const SINGLE_BYTE: &[&str] = &[
    "IBM-Thai",
    "IBM00858",
    "IBM01140",
    "IBM01141",
    "IBM01142",
    "IBM01143",
    "IBM01144",
    "IBM01145",
    "IBM01146",
    "IBM01147",
    "IBM01148",
    "IBM01149",
    "IBM037",
    "IBM1047",
    "IBM273",
    "IBM277",
    "IBM278",
    "IBM280",
    "IBM284",
    "IBM285",
    "IBM297",
    "IBM424",
    "IBM437",
    "IBM500",
    "IBM775",
    "IBM850",
    "IBM852",
    "IBM855",
    "IBM857",
    "IBM860",
    "IBM861",
    "IBM862",
    "IBM863",
    "IBM864",
    "IBM865",
    "IBM866",
    "IBM868",
    "IBM869",
    "IBM870",
    "IBM871",
    "IBM918",
    "ISO-8859-13",
    "ISO-8859-15",
    "ISO-8859-16",
    "ISO-8859-2",
    "ISO-8859-3",
    "ISO-8859-4",
    "ISO-8859-5",
    "ISO-8859-6",
    "ISO-8859-7",
    "ISO-8859-8",
    "JIS_X0201",
    "KOI8-R",
    "KOI8-U",
    "TIS-620",
    "windows-1250",
    "windows-1251",
    "windows-1253",
    "windows-1254",
    "windows-1255",
    "windows-1256",
    "windows-1257",
    "windows-1258",
    "x-IBM1006",
    "x-IBM1025",
    "x-IBM1046",
    "x-IBM1097",
    "x-IBM1098",
    "x-IBM1112",
    "x-IBM1122",
    "x-IBM1123",
    "x-IBM1124",
    "x-IBM1129",
    "x-IBM1166",
    "x-IBM737",
    "x-IBM833",
    "x-IBM856",
    "x-IBM874",
    "x-IBM875",
    "x-IBM921",
    "x-IBM922",
    "x-MacArabic",
    "x-MacCentralEurope",
    "x-MacCroatian",
    "x-MacCyrillic",
    "x-MacGreek",
    "x-MacHebrew",
    "x-MacIceland",
    "x-MacRoman",
    "x-MacRomania",
    "x-MacThai",
    "x-MacTurkish",
    "x-MacUkraine",
    "x-iso-8859-11",
    "x-windows-874",
];

const DOUBLE_BYTE: &[&str] = &[
    "Big5",
    "Big5-HKSCS",
    "EUC-KR",
    "GB2312",
    "GBK",
    "Shift_JIS",
    "windows-31j",
    "x-Big5-HKSCS-2001",
    "x-Big5-Solaris",
    "x-IBM1381",
    "x-IBM1383",
    "x-IBM942",
    "x-IBM942C",
    "x-IBM943",
    "x-IBM943C",
    "x-IBM948",
    "x-IBM949",
    "x-IBM949C",
    "x-IBM950",
    "x-IBM970",
    "x-Johab",
    "x-MS932_0213",
    "x-MS950-HKSCS",
    "x-MS950-HKSCS-XP",
    "x-PCK",
    "x-SJIS_0213",
    "x-mswin-936",
    "x-windows-949",
    "x-windows-950",
];

pub(crate) fn decode(name: &'static str, bytes: &[u8]) -> Result<String, XmlError> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(XmlError::ResourceLimit { resource: "source bytes", maximum: MAX_DOCUMENT_BYTES });
    }
    let mut output = Output::new();
    if SINGLE_BYTE.binary_search(&name).is_ok() { decode_stateless(name, bytes, false, &mut output)?; }
    else if DOUBLE_BYTE.binary_search(&name).is_ok() { decode_stateless(name, bytes, true, &mut output)?; }
    else {
        match name {
            "ISO-2022-JP" | "ISO-2022-JP-2" | "x-windows-50220" | "x-windows-50221" | "x-windows-iso2022jp" => decode_iso_jp(name, bytes, &mut output)?,
            "ISO-2022-CN" | "x-ISO-2022-CN-CNS" | "x-ISO-2022-CN-GB" => decode_iso_cn(bytes, &mut output)?,
            "ISO-2022-KR" => decode_iso_kr(bytes, &mut output)?,
            "x-IBM1364" | "x-IBM933" | "x-IBM935" | "x-IBM937" | "x-IBM939" => decode_shift_ebcdic(name, bytes, &mut output)?,
            "EUC-JP" | "x-IBM29626C" | "x-IBM33722" | "x-euc-jp-linux" | "x-eucJP-Open" | "x-EUC-TW" | "x-IBM964" => decode_euc(name, bytes, &mut output)?,
            "GB18030" => decode_gb18030(bytes, &mut output)?,
            "x-ISCII91" => decode_iscii(bytes, &mut output)?,
            "x-JISAutoDetect" => decode_auto_japanese(bytes, &mut output)?,
            _ => return Err(missing(name, "reader")),
        }
    }
    output.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso2022_and_ebcdic_controls_preserve_decoder_state() {
        assert_eq!(decode("ISO-2022-JP", b"A\x1b(J\\~\x1b(I!\x0e\"\x0f#\x1b(BA").unwrap(), "A\u{00a5}\u{203e}\u{ff61}\u{ff62}\u{ff63}A");
        assert_eq!(decode("ISO-2022-CN", b"A\x1b$)A\x0e!!\x0f\xff").unwrap(), "A\u{3000}\u{00ff}");
        assert_eq!(decode("x-IBM933", &[0xc1, 0x0e, 0x0e, 0x0f, 0xc2]).unwrap(), "A\u{fffd}B");
        assert_eq!(decode("x-IBM933", &[0xc1, 0x0f, 0xc2]).unwrap(), "A\u{fffd}B");
        assert_eq!(decode("x-IBM933", &[0xc1, 0x0e]).unwrap(), "A");
    }

    #[test]
    fn malformed_legacy_sequences_preserve_consumption_and_eof() {
        assert_eq!(decode("ISO-2022-JP", b"A\x1bXBY").unwrap(), "A\u{fffd}BY");
        assert_eq!(decode("ISO-2022-JP", b"\x1b$(DZ").unwrap(), "\u{fffd}DZ");
        assert_eq!(decode("ISO-2022-JP-2", b"\x1b$(DZ").unwrap(), "\u{fffd}");
        assert_eq!(decode("ISO-2022-JP", b"A\x1b").unwrap(), "A\u{fffd}");
        assert_eq!(decode("ISO-2022-JP", b"A\x0e").unwrap(), "A");
        assert_eq!(decode("EUC-JP", b"\x8f AZ").unwrap(), "\u{fffd}Z");
        assert_eq!(decode("x-IBM33722", b"\x8f AZ").unwrap(), "\u{fffd}AZ");
        assert_eq!(decode("x-EUC-TW", b"\x8e\xa2 A!").unwrap(), "\u{fffd}!");
        assert_eq!(decode("x-IBM964", b"\x8e\xa2 A!").unwrap(), "\u{fffd}A!");
        assert_eq!(decode("GB18030", b"\x81 A").unwrap(), "\u{fffd} A");
        assert_eq!(decode("GB18030", b"\x81\x30 A").unwrap(), "\u{fffd}A");
        assert_eq!(decode("GB18030", b"\x81\x30\x81 A").unwrap(), "\u{fffd}A");
        assert_eq!(decode("GB18030", b"\x81\x30").unwrap(), "\u{fffd}");
    }

    #[test]
    fn iscii_context_and_japanese_detection_match_source() {
        assert_eq!(decode("x-ISCII91", &[0xa1]).unwrap(), "\u{0901}");
        assert_eq!(decode("x-ISCII91", &[0xa1, 0xe9]).unwrap(), "\u{0950}");
        assert_eq!(decode("x-ISCII91", &[0xe8, 0xe8, 0xe9, 0xef, b'A', b'B']).unwrap(), "\u{094d}\u{200c}\u{200d}\u{fffd}\u{fffd}B");
        assert_eq!(decode("x-JISAutoDetect", b"plain").unwrap(), "plain");
        assert_eq!(decode("x-JISAutoDetect", b"\x82\xa0\x82\xa2").unwrap(), "\u{3042}\u{3044}");
        assert_eq!(decode("x-JISAutoDetect", b"\xa4\xa2\xa4\xa4").unwrap(), "\u{3042}\u{3044}");
        assert_eq!(decode("x-JISAutoDetect", b"\x1b$B$\x22$$\x1b(B").unwrap(), "\u{3042}\u{3044}");
    }

    #[test]
    fn legacy_output_expansion_preserves_document_limit() {
        let mut input = vec![0xa4; MAX_DOCUMENT_BYTES / 3];
        let text = decode("ISO-8859-15", &input).unwrap();
        assert_eq!(text.len(), MAX_DOCUMENT_BYTES - 1);
        assert!(text.chars().all(|value| value == '\u{20ac}'));
        drop(text);
        input.push(0xa4);
        assert!(matches!(decode("ISO-8859-15", &input), Err(XmlError::ResourceLimit { resource: "decoded bytes", maximum: MAX_DOCUMENT_BYTES })));
    }
}
