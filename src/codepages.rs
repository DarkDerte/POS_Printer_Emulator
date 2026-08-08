#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Codepage {
    Cp437,
    Cp850,
    Cp858,
    Cp866,
    Cp1252,
    Cp1250,
    Cp1251,
    Cp1253,
    Cp1256,
    Cp874,
    Cp932, // Shift-JIS
    Cp936, // GBK
    Cp949, // EUC-KR (windows-949)
    Cp950, // Big5
}

static CP437_HIGH: [char; 128] = [
    '\u{00c7}', '\u{00fc}', '\u{00e9}', '\u{00e2}', '\u{00e4}', '\u{00e0}', '\u{00e5}', '\u{00e7}',
    '\u{00ea}', '\u{00eb}', '\u{00e8}', '\u{00ef}', '\u{00ee}', '\u{00ec}', '\u{00c4}', '\u{00c5}',
    '\u{00c9}', '\u{00e6}', '\u{00c6}', '\u{00f4}', '\u{00f6}', '\u{00f2}', '\u{00fb}', '\u{00f9}',
    '\u{00ff}', '\u{00d6}', '\u{00dc}', '\u{00a2}', '\u{00a3}', '\u{00a5}', '\u{20a7}', '\u{0192}',
    '\u{00e1}', '\u{00ed}', '\u{00f3}', '\u{00fa}', '\u{00f1}', '\u{00d1}', '\u{00aa}', '\u{00ba}',
    '\u{00bf}', '\u{2310}', '\u{00ac}', '\u{00bd}', '\u{00bc}', '\u{00a1}', '\u{00ab}', '\u{00bb}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{2561}', '\u{2562}', '\u{2556}',
    '\u{2555}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255d}', '\u{255c}', '\u{255b}', '\u{2510}',
    '\u{2514}', '\u{2534}', '\u{252c}', '\u{251c}', '\u{2500}', '\u{253c}', '\u{255e}', '\u{255f}',
    '\u{255a}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256c}', '\u{2567}',
    '\u{2568}', '\u{2564}', '\u{2565}', '\u{2559}', '\u{2558}', '\u{2552}', '\u{2553}', '\u{256b}',
    '\u{256a}', '\u{2518}', '\u{250c}', '\u{2588}', '\u{2584}', '\u{258c}', '\u{2590}', '\u{2580}',
    '\u{03b1}', '\u{00df}', '\u{0393}', '\u{03c0}', '\u{03a3}', '\u{03c3}', '\u{00b5}', '\u{03c4}',
    '\u{03a6}', '\u{0398}', '\u{03a9}', '\u{03b4}', '\u{221e}', '\u{03c6}', '\u{03b5}', '\u{2229}',
    '\u{2261}', '\u{00b1}', '\u{2265}', '\u{2264}', '\u{2320}', '\u{2321}', '\u{00f7}', '\u{2248}',
    '\u{00b0}', '\u{2219}', '\u{00b7}', '\u{221a}', '\u{207f}', '\u{00b2}', '\u{25a0}', '\u{00a0}',
];

static CP850_HIGH: [char; 128] = [
    '\u{00c7}', '\u{00fc}', '\u{00e9}', '\u{00e2}', '\u{00e4}', '\u{00e0}', '\u{00e5}', '\u{00e7}',
    '\u{00ea}', '\u{00eb}', '\u{00e8}', '\u{00ef}', '\u{00ee}', '\u{00ec}', '\u{00c4}', '\u{00c5}',
    '\u{00c9}', '\u{00e6}', '\u{00c6}', '\u{00f4}', '\u{00f6}', '\u{00f2}', '\u{00fb}', '\u{00f9}',
    '\u{00ff}', '\u{00d6}', '\u{00dc}', '\u{00f8}', '\u{00a3}', '\u{00d8}', '\u{00d7}', '\u{0192}',
    '\u{00e1}', '\u{00ed}', '\u{00f3}', '\u{00fa}', '\u{00f1}', '\u{00d1}', '\u{00aa}', '\u{00ba}',
    '\u{00bf}', '\u{00ae}', '\u{00ac}', '\u{00bd}', '\u{00bc}', '\u{00a1}', '\u{00ab}', '\u{00bb}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{00c1}', '\u{00c2}', '\u{00c0}',
    '\u{00a9}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255d}', '\u{00a2}', '\u{00a5}', '\u{2510}',
    '\u{2514}', '\u{2534}', '\u{252c}', '\u{251c}', '\u{2500}', '\u{253c}', '\u{00e3}', '\u{00c3}',
    '\u{255a}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256c}', '\u{00a4}',
    '\u{00f0}', '\u{00d0}', '\u{00ca}', '\u{00cb}', '\u{00c8}', '\u{0131}', '\u{00cd}', '\u{00ce}',
    '\u{00cf}', '\u{2518}', '\u{250c}', '\u{2588}', '\u{2584}', '\u{00a6}', '\u{00cc}', '\u{2580}',
    '\u{00d3}', '\u{00df}', '\u{00d4}', '\u{00d2}', '\u{00f5}', '\u{00d5}', '\u{00b5}', '\u{00fe}',
    '\u{00de}', '\u{00da}', '\u{00db}', '\u{00d9}', '\u{00fd}', '\u{00dd}', '\u{00af}', '\u{00b4}',
    '\u{00ad}', '\u{00b1}', '\u{2017}', '\u{00be}', '\u{00b6}', '\u{00a7}', '\u{00f7}', '\u{00b8}',
    '\u{00b0}', '\u{00a8}', '\u{00b7}', '\u{00b9}', '\u{00b3}', '\u{00b2}', '\u{25a0}', '\u{00a0}',
];

static CP866_HIGH: [char; 128] = [
    '\u{0410}', '\u{0411}', '\u{0412}', '\u{0413}', '\u{0414}', '\u{0415}', '\u{0416}', '\u{0417}',
    '\u{0418}', '\u{0419}', '\u{041a}', '\u{041b}', '\u{041c}', '\u{041d}', '\u{041e}', '\u{041f}',
    '\u{0420}', '\u{0421}', '\u{0422}', '\u{0423}', '\u{0424}', '\u{0425}', '\u{0426}', '\u{0427}',
    '\u{0428}', '\u{0429}', '\u{042a}', '\u{042b}', '\u{042c}', '\u{042d}', '\u{042e}', '\u{042f}',
    '\u{0430}', '\u{0431}', '\u{0432}', '\u{0433}', '\u{0434}', '\u{0435}', '\u{0436}', '\u{0437}',
    '\u{0438}', '\u{0439}', '\u{043a}', '\u{043b}', '\u{043c}', '\u{043d}', '\u{043e}', '\u{043f}',
    '\u{2591}', '\u{2592}', '\u{2593}', '\u{2502}', '\u{2524}', '\u{2561}', '\u{2562}', '\u{2556}',
    '\u{2555}', '\u{2563}', '\u{2551}', '\u{2557}', '\u{255d}', '\u{255c}', '\u{255b}', '\u{2510}',
    '\u{2514}', '\u{2534}', '\u{252c}', '\u{251c}', '\u{2500}', '\u{253c}', '\u{255e}', '\u{255f}',
    '\u{255a}', '\u{2554}', '\u{2569}', '\u{2566}', '\u{2560}', '\u{2550}', '\u{256c}', '\u{2567}',
    '\u{2568}', '\u{2564}', '\u{2565}', '\u{2559}', '\u{2558}', '\u{2552}', '\u{2553}', '\u{256b}',
    '\u{256a}', '\u{2518}', '\u{250c}', '\u{2588}', '\u{2584}', '\u{258c}', '\u{2590}', '\u{2580}',
    '\u{0440}', '\u{0441}', '\u{0442}', '\u{0443}', '\u{0444}', '\u{0445}', '\u{0446}', '\u{0447}',
    '\u{0448}', '\u{0449}', '\u{044a}', '\u{044b}', '\u{044c}', '\u{044d}', '\u{044e}', '\u{044f}',
    '\u{0401}', '\u{0451}', '\u{0404}', '\u{0454}', '\u{0407}', '\u{0457}', '\u{040e}', '\u{045e}',
    '\u{00b0}', '\u{2219}', '\u{00b7}', '\u{221a}', '\u{2116}', '\u{00a4}', '\u{25a0}', '\u{00a0}',
];

static CP1252_HIGH: [char; 128] = [
    '\u{20ac}', '\u{fffd}', '\u{201a}', '\u{0192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02c6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{fffd}', '\u{017d}', '\u{fffd}',
    '\u{fffd}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02dc}', '\u{2122}', '\u{0161}', '\u{203a}', '\u{0153}', '\u{fffd}', '\u{017e}', '\u{0178}',
    '\u{00a0}', '\u{00a1}', '\u{00a2}', '\u{00a3}', '\u{00a4}', '\u{00a5}', '\u{00a6}', '\u{00a7}',
    '\u{00a8}', '\u{00a9}', '\u{00aa}', '\u{00ab}', '\u{00ac}', '\u{00ad}', '\u{00ae}', '\u{00af}',
    '\u{00b0}', '\u{00b1}', '\u{00b2}', '\u{00b3}', '\u{00b4}', '\u{00b5}', '\u{00b6}', '\u{00b7}',
    '\u{00b8}', '\u{00b9}', '\u{00ba}', '\u{00bb}', '\u{00bc}', '\u{00bd}', '\u{00be}', '\u{00bf}',
    '\u{00c0}', '\u{00c1}', '\u{00c2}', '\u{00c3}', '\u{00c4}', '\u{00c5}', '\u{00c6}', '\u{00c7}',
    '\u{00c8}', '\u{00c9}', '\u{00ca}', '\u{00cb}', '\u{00cc}', '\u{00cd}', '\u{00ce}', '\u{00cf}',
    '\u{00d0}', '\u{00d1}', '\u{00d2}', '\u{00d3}', '\u{00d4}', '\u{00d5}', '\u{00d6}', '\u{00d7}',
    '\u{00d8}', '\u{00d9}', '\u{00da}', '\u{00db}', '\u{00dc}', '\u{00dd}', '\u{00de}', '\u{00df}',
    '\u{00e0}', '\u{00e1}', '\u{00e2}', '\u{00e3}', '\u{00e4}', '\u{00e5}', '\u{00e6}', '\u{00e7}',
    '\u{00e8}', '\u{00e9}', '\u{00ea}', '\u{00eb}', '\u{00ec}', '\u{00ed}', '\u{00ee}', '\u{00ef}',
    '\u{00f0}', '\u{00f1}', '\u{00f2}', '\u{00f3}', '\u{00f4}', '\u{00f5}', '\u{00f6}', '\u{00f7}',
    '\u{00f8}', '\u{00f9}', '\u{00fa}', '\u{00fb}', '\u{00fc}', '\u{00fd}', '\u{00fe}', '\u{00ff}',
];

pub fn table(n: u8) -> Codepage {
    match n {
        2 => Codepage::Cp850,
        11 => Codepage::Cp866,
        13 | 255 => Codepage::Cp858,
        14 | 21 => Codepage::Cp874,
        16 => Codepage::Cp1252,
        18 | 22 => Codepage::Cp1250,
        19 | 23 => Codepage::Cp1251,
        20 | 24 | 25 | 26 | 27 | 28 | 29 => Codepage::Cp437,
        80 => Codepage::Cp874,
        81 => Codepage::Cp932,
        82 => Codepage::Cp936,
        83 => Codepage::Cp949,
        84 => Codepage::Cp950,
        85 => Codepage::Cp1251,
        86 => Codepage::Cp1250,
        87 => Codepage::Cp1253,
        88 => Codepage::Cp1256,
        _ => Codepage::Cp437,
    }
}

// Los codepages multibyte (JIS/GBK/EUC-KR/Big5) se decodifican por pares.
pub fn is_double_byte(cp: Codepage) -> bool {
    matches!(cp, Codepage::Cp932 | Codepage::Cp936 | Codepage::Cp949 | Codepage::Cp950)
}

pub fn is_lead(cp: Codepage, b: u8) -> bool {
    match cp {
        Codepage::Cp932 => (0x81..=0x9F).contains(&b) || (0xE0..=0xEF).contains(&b),
        Codepage::Cp936 => (0x81..=0xFE).contains(&b),
        Codepage::Cp949 => (0x81..=0xFE).contains(&b),
        Codepage::Cp950 => (0x81..=0xFE).contains(&b),
        _ => false,
    }
}

pub fn is_trail(cp: Codepage, b: u8) -> bool {
    match cp {
        Codepage::Cp932 => (0x40..=0x7E).contains(&b) || (0x80..=0xFC).contains(&b),
        Codepage::Cp936 => (0x40..=0xFE).contains(&b) && b != 0x7F,
        Codepage::Cp949 => (0x41..=0xFE).contains(&b),
        Codepage::Cp950 => (0x40..=0x7E).contains(&b) || (0xA1..=0xFE).contains(&b),
        _ => false,
    }
}

fn decode_rs(enc: &'static encoding_rs::Encoding, b: u8) -> char {
    let buf = [b];
    let (s, _, _) = enc.decode(&buf);
    s.chars().next().unwrap_or('\u{FFFD}')
}

fn decode_pair_rs(enc: &'static encoding_rs::Encoding, lead: u8, trail: u8) -> Option<String> {
    let buf = [lead, trail];
    let (s, _, had) = enc.decode(&buf);
    if had || s.contains('\u{FFFD}') {
        return None;
    }
    Some(s.into_owned())
}

// Decodifica un par en el codepage multibyte activo.
pub fn decode_pair(cp: Codepage, lead: u8, trail: u8) -> Option<String> {
    match cp {
        Codepage::Cp932 => decode_pair_rs(encoding_rs::SHIFT_JIS, lead, trail),
        Codepage::Cp936 => decode_pair_rs(encoding_rs::GBK, lead, trail),
        Codepage::Cp949 => decode_pair_rs(encoding_rs::EUC_KR, lead, trail),
        Codepage::Cp950 => decode_pair_rs(encoding_rs::BIG5, lead, trail),
        _ => None,
    }
}

// Modo kanji (FS & / FS 2): los pares se interpretan como Shift-JIS.
pub fn kanji_lead(b: u8) -> bool {
    (0x81..=0x9F).contains(&b) || (0xE0..=0xEF).contains(&b)
}

pub fn kanji_trail(b: u8) -> bool {
    (0x40..=0x7E).contains(&b) || (0x80..=0xFC).contains(&b)
}

pub fn decode_kanji_pair(lead: u8, trail: u8) -> Option<String> {
    decode_pair_rs(encoding_rs::SHIFT_JIS, lead, trail)
}

pub fn decode(cp: Codepage, b: u8) -> char {
    if b < 0x80 {
        return b as char;
    }
    let idx = (b - 0x80) as usize;
    match cp {
        Codepage::Cp437 => CP437_HIGH[idx],
        Codepage::Cp850 => CP850_HIGH[idx],
        Codepage::Cp866 => CP866_HIGH[idx],
        Codepage::Cp1252 => CP1252_HIGH[idx],
        Codepage::Cp858 => {
            if b == 0xD5 {
                '\u{20ac}'
            } else {
                CP437_HIGH[idx]
            }
        }
        Codepage::Cp1250 => decode_rs(encoding_rs::WINDOWS_1250, b),
        Codepage::Cp1251 => decode_rs(encoding_rs::WINDOWS_1251, b),
        Codepage::Cp1253 => decode_rs(encoding_rs::WINDOWS_1253, b),
        Codepage::Cp1256 => decode_rs(encoding_rs::WINDOWS_1256, b),
        Codepage::Cp874 => decode_rs(encoding_rs::WINDOWS_874, b),
        Codepage::Cp932 | Codepage::Cp936 | Codepage::Cp949 | Codepage::Cp950 => {
            // byte suelto en contexto multibyte: carácter de reemplazo
            '\u{FFFD}'
        }
    }
}
