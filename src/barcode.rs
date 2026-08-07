// Códigos de barras 1D: dígitos de control y patrones de módulos reales (EAN/UPC).

const L_CODES: [[bool; 7]; 10] = [
    [false, false, false, true, true, false, true],
    [false, false, true, true, false, false, true],
    [false, false, true, false, false, true, true],
    [false, true, true, true, true, false, true],
    [false, true, false, false, false, true, true],
    [false, true, true, false, false, false, true],
    [false, true, false, true, true, true, true],
    [false, true, true, true, false, true, true],
    [false, true, true, false, true, true, true],
    [false, false, false, true, false, true, true],
];

// Patrones de paridad del primer dígito EAN-13: 0=L, 1=G
const PARITY: [[u8; 6]; 10] = [
    [0, 0, 0, 0, 0, 0],
    [0, 0, 1, 0, 1, 1],
    [0, 0, 1, 1, 0, 1],
    [0, 0, 1, 1, 1, 0],
    [0, 1, 0, 0, 1, 1],
    [0, 1, 1, 0, 0, 1],
    [0, 1, 1, 1, 0, 0],
    [0, 1, 0, 1, 0, 1],
    [0, 1, 0, 1, 1, 0],
    [0, 1, 1, 0, 1, 0],
];

fn l_code(d: usize) -> &'static [bool; 7] {
    &L_CODES[d]
}

fn g_code(d: usize) -> [bool; 7] {
    let mut g = [false; 7];
    for i in 0..7 {
        g[i] = L_CODES[d][6 - i];
    }
    g
}

fn c_code(d: usize) -> [bool; 7] {
    let l = L_CODES[d];
    let mut c = [false; 7];
    for i in 0..7 {
        c[i] = l[6 - i];
    }
    c
}

// Dígito de control EAN-13 / EAN-8 / UPC-A (pesos 3 y 1 desde la derecha).
pub fn check_digit(digits: &[u8]) -> u8 {
    let mut sum: u32 = 0;
    let n = digits.len();
    for (i, &d) in digits.iter().enumerate() {
        let weight = if (n - 1 - i) % 2 == 0 { 3 } else { 1 };
        sum += ((d - b'0') as u32) * weight;
    }
    ((10 - (sum % 10)) % 10) as u8
}

fn digits_from(data: &[u8]) -> Option<Vec<u8>> {
    data.iter()
        .map(|&b| {
            if (b'0'..=b'9').contains(&b) {
                Some(b - b'0')
            } else {
                None
            }
        })
        .collect()
}

// EAN-13: recibe 13 dígitos (con dígito de control) y produce 95 módulos.
pub fn ean13_modules(digits: &[u8]) -> Option<Vec<bool>> {
    let d = digits_from(digits)?;
    if d.len() != 13 {
        return None;
    }
    let parity = PARITY[d[0] as usize];
    let mut out = Vec::with_capacity(95);
    out.extend_from_slice(&[true, false, true]); // guarda inicio
    for (i, &dig) in d[1..7].iter().enumerate() {
        if parity[i] == 0 {
            out.extend_from_slice(l_code(dig as usize));
        } else {
            out.extend_from_slice(&g_code(dig as usize));
        }
    }
    out.extend_from_slice(&[false, true, false, true, false]); // separador central
    for &dig in &d[7..13] {
        out.extend_from_slice(&c_code(dig as usize));
    }
    out.extend_from_slice(&[true, false, true]); // guarda final
    Some(out)
}

// EAN-8: recibe 8 dígitos (con dígito de control) y produce 67 módulos.
pub fn ean8_modules(digits: &[u8]) -> Option<Vec<bool>> {
    let d = digits_from(digits)?;
    if d.len() != 8 {
        return None;
    }
    let mut out = Vec::with_capacity(67);
    out.extend_from_slice(&[true, false, true]);
    for &dig in &d[0..4] {
        out.extend_from_slice(l_code(dig as usize));
    }
    out.extend_from_slice(&[false, true, false, true, false]);
    for &dig in &d[4..8] {
        out.extend_from_slice(&c_code(dig as usize));
    }
    out.extend_from_slice(&[true, false, true]);
    Some(out)
}

// UPC-A (12 dígitos) = EAN-13 con primer dígito 0.
pub fn upca_as_ean13(digits: &[u8]) -> Option<Vec<u8>> {
    let d = digits_from(digits)?;
    if d.len() != 12 {
        return None;
    }
    let mut ean = Vec::with_capacity(13);
    ean.push(0);
    ean.extend_from_slice(&d);
    Some(ean)
}

// Normaliza y valida datos de GS k; devuelve (datos, aviso opcional).
pub fn validate(m: u8, data: &[u8]) -> (Vec<u8>, Option<String>) {
    // Los códigos 1D admiten dos familias de m: 0..=6 (longitud explícita) y
    // 65..=73 (terminado en NUL). Se unifican para validar igual.
    let norm_m = match m {
        0 | 65 => 65, // UPC-A
        2 | 67 => 67, // EAN-13
        3 | 68 => 68, // EAN-8
        4 | 69 => 69, // Code 39
        5 | 70 => 70, // ITF
        6 | 71 => 71, // Codabar
        other => other,
    };
    let digits = |d: &[u8]| d.iter().all(|&b| (b'0'..=b'9').contains(&b));
    match norm_m {
        65 => {
            // UPC-A: 11 o 12 dígitos (se añade el dígito de control si falta)
            if digits(data) && (data.len() == 11 || data.len() == 12) {
                let mut d = data.to_vec();
                if d.len() == 11 {
                    let cd = check_digit(&d);
                    d.push(b'0' + cd);
                    (d, Some("UPC-A: dígito de control calculado".to_string()))
                } else {
                    let cd = check_digit(&d[..11]);
                    let msg = if d[11] == b'0' + cd {
                        None
                    } else {
                        Some(format!(
                            "UPC-A: dígito de control inválido (era {}, debería ser {cd})",
                            d[11] as char
                        ))
                    };
                    (d, msg)
                }
            } else {
                (data.to_vec(), Some("UPC-A: se requieren 11/12 dígitos".to_string()))
            }
        }
        67 => {
            if digits(data) && (data.len() == 12 || data.len() == 13) {
                let mut d = data.to_vec();
                if d.len() == 12 {
                    let cd = check_digit(&d);
                    d.push(b'0' + cd);
                    (d, Some("EAN-13: dígito de control calculado".to_string()))
                } else {
                    let cd = check_digit(&d[..12]);
                    let msg = if d[12] == b'0' + cd {
                        None
                    } else {
                        Some(format!(
                            "EAN-13: dígito de control inválido (era {}, debería ser {cd})",
                            d[12] as char
                        ))
                    };
                    (d, msg)
                }
            } else {
                (data.to_vec(), Some("EAN-13: se requieren 12/13 dígitos".to_string()))
            }
        }
        68 => {
            if digits(data) && (data.len() == 7 || data.len() == 8) {
                let mut d = data.to_vec();
                if d.len() == 7 {
                    let cd = check_digit(&d);
                    d.push(b'0' + cd);
                    (d, Some("EAN-8: dígito de control calculado".to_string()))
                } else {
                    let cd = check_digit(&d[..7]);
                    let msg = if d[7] == b'0' + cd {
                        None
                    } else {
                        Some(format!(
                            "EAN-8: dígito de control inválido (era {}, debería ser {cd})",
                            d[7] as char
                        ))
                    };
                    (d, msg)
                }
            } else {
                (data.to_vec(), Some("EAN-8: se requieren 7/8 dígitos".to_string()))
            }
        }
        69 => {
            // Code 39: A-Z, 0-9 y - . espacio $ / + % *
            let ok = data.iter().all(|&b| {
                (b'A'..=b'Z').contains(&b)
                    || (b'0'..=b'9').contains(&b)
                    || matches!(b, b'-' | b'.' | b' ' | b'$' | b'/' | b'+' | b'%' | b'*')
            });
            if ok {
                (data.to_vec(), None)
            } else {
                (data.to_vec(), Some("Code 39: carácter inválido".to_string()))
            }
        }
        70 => {
            // ITF: número par de dígitos
            if digits(data) && data.len() % 2 == 0 {
                (data.to_vec(), None)
            } else {
                (data.to_vec(), Some("ITF: se requiere número par de dígitos".to_string()))
            }
        }
        71 => {
            // Codabar: 0-9 y - $ : / . +
            let ok = data.iter().all(|&b| {
                (b'0'..=b'9').contains(&b) || matches!(b, b'-' | b'$' | b':' | b'/' | b'.' | b'+')
            });
            if ok {
                (data.to_vec(), None)
            } else {
                (data.to_vec(), Some("Codabar: carácter inválido".to_string()))
            }
        }
        72 => {
            // Code 93: mismo juego de caracteres que Code 39
            let ok = data.iter().all(|&b| {
                (b'A'..=b'Z').contains(&b)
                    || (b'0'..=b'9').contains(&b)
                    || matches!(b, b'-' | b'.' | b' ' | b'$' | b'/' | b'+' | b'%')
            });
            if ok {
                (data.to_vec(), None)
            } else {
                (data.to_vec(), Some("Code 93: carácter inválido".to_string()))
            }
        }
        73 => {
            // Code 128: bytes 0x00..=0x7F
            if data.iter().all(|&b| b <= 0x7F) {
                (data.to_vec(), None)
            } else {
                (data.to_vec(), Some("Code 128: byte > 0x7F no soportado".to_string()))
            }
        }
        _ => (data.to_vec(), None),
    }
}

// Tabla de patrones del Code 128 (valores 0..=105, 11 módulos; parada 13).
const CODE128: [&str; 106] = [
    "11011001100", "11001101100", "11001100110", "10010011000", "10010001100",
    "10001001100", "10011001000", "10011000100", "10001100100", "11001001000",
    "11001000100", "11000100100", "10110011100", "10011011100", "10011001110",
    "10111001100", "10011101100", "10011100110", "11001110010", "11001011100",
    "11001001110", "11011100100", "11001110100", "11101101110", "11101001100",
    "11100101100", "11100100110", "11101100100", "11100110100", "11100110010",
    "11011011000", "11011000110", "11000110110", "10100011000", "10001011000",
    "10001000110", "10110001000", "10001101000", "10001100010", "11010001000",
    "11000101000", "11000100010", "10110111000", "10110001110", "10001101110",
    "10111011000", "10111000110", "10001110110", "11101110110", "11010001110",
    "11000101110", "11011101000", "11011100010", "11011101110", "11101011000",
    "11101000110", "11100010110", "11101101000", "11101100010", "11100011010",
    "11101111010", "11001000010", "11110001010", "10100110000", "10100001100",
    "10010110000", "10010000110", "10000101100", "10000100110", "10110010000",
    "10110000100", "10011010000", "10011000010", "10000110100", "10000110010",
    "11000010010", "11001010000", "11110111010", "11000010100", "10001111010",
    "10100111100", "10010111100", "10010011110", "10111100100", "10011110100",
    "10011110010", "11110100100", "11110010100", "11110010010", "11011011110",
    "11011110110", "11110110110", "10101111000", "10100011110", "10001011110",
    "10111101000", "10111100010", "11110101000", "11110100010", "10111011110",
    "10111101110", "11101011110", "11110101110", "11010000100", "11010010000",
    "11010011100",
];

// Code 128: selecciona el subjuego más compacto (C para dígitos pares, A/B
// para texto), calcula la suma de control y devuelve los módulos.
pub fn code128_modules(data: &[u8]) -> Option<Vec<bool>> {
    if data.is_empty() {
        return None;
    }
    let code_c = data.len() % 2 == 0 && data.iter().all(|b| b.is_ascii_digit());
    let mut values: Vec<usize> = Vec::new();
    if code_c {
        values.push(105); // Start C
        let mut i = 0;
        while i < data.len() {
            let v = (data[i] - b'0') as usize * 10 + (data[i + 1] - b'0') as usize;
            values.push(v);
            i += 2;
        }
    } else {
        let use_a = data.iter().all(|&b| b < 0x20 || (0x20..=0x5F).contains(&b));
        values.push(if use_a { 103 } else { 104 }); // Start A / Start B
        for &b in data {
            let v = if use_a {
                if b < 0x20 {
                    b as usize + 64
                } else {
                    b as usize - 32
                }
            } else {
                if b < 0x20 {
                    return None;
                }
                b as usize - 32
            };
            values.push(v);
        }
    }
    let mut sum = values[0];
    for (i, &v) in values.iter().enumerate().skip(1) {
        sum = (sum + v * i) % 103;
    }
    values.push(sum);
    let mut out = Vec::with_capacity(values.len() * 11 + 13);
    for v in values {
        for c in CODE128[v].bytes() {
            out.push(c == b'1');
        }
    }
    for c in "1100011101011".bytes() {
        out.push(c == b'1');
    }
    Some(out)
}

// Tabla de anchos de ITF (1 = ancho, 0 = estrecho), 5 elementos por dígito.
const ITF_DIGITS: [[u8; 5]; 10] = [
    [0, 0, 1, 1, 0],
    [1, 0, 0, 0, 1],
    [0, 1, 0, 0, 1],
    [1, 1, 0, 0, 0],
    [0, 0, 1, 0, 1],
    [1, 0, 1, 0, 0],
    [0, 1, 1, 0, 0],
    [0, 0, 0, 1, 1],
    [1, 0, 0, 1, 0],
    [0, 1, 0, 1, 0],
];

// ITF (Interleaved 2 of 5): los dígitos se agrupan en pares; el primero
// codifica barras y el segundo espacios.
pub fn itf_modules(data: &[u8]) -> Option<Vec<bool>> {
    let d = digits_from(data)?;
    if d.len() < 2 || d.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::new();
    out.extend_from_slice(&[true, false, true, false]); // start
    for pair in d.chunks(2) {
        let bars = ITF_DIGITS[pair[0] as usize];
        let spaces = ITF_DIGITS[pair[1] as usize];
        for i in 0..5 {
            let bw = if bars[i] == 1 { 3 } else { 1 };
            for _ in 0..bw {
                out.push(true);
            }
            let sw = if spaces[i] == 1 { 3 } else { 1 };
            for _ in 0..sw {
                out.push(false);
            }
        }
    }
    // stop: barra ancha, espacio estrecho, barra estrecha
    out.extend_from_slice(&[true, true, true, false, true]);
    Some(out)
}

// Tabla de patrones del Code 39 (9 elementos, 1 = ancho, 0 = estrecho).
const CODE39: [&str; 44] = [
    "000110100", "100100001", "001100001", "101100000", "000110001", "100110000",
    "001110000", "000100101", "100100100", "001100100", "100001001", "001001001",
    "101001000", "000011001", "100011000", "001011000", "000001101", "100001100",
    "001001100", "000011100", "100000011", "001000011", "101000010", "000010011",
    "100010010", "001010010", "000000111", "100000110", "001000110", "000010110",
    "110000001", "011000001", "111000000", "010010001", "110010000", "011010000",
    "010000101", "110000100", "011000100", "010101000", "010100010", "010001010",
    "000101010", "010010100",
];

fn code39_index(c: u8) -> Option<usize> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as usize),
        b'A'..=b'Z' => Some((c - b'A') as usize + 10),
        b'-' => Some(36),
        b'.' => Some(37),
        b' ' => Some(38),
        b'$' => Some(39),
        b'/' => Some(40),
        b'+' => Some(41),
        b'%' => Some(42),
        b'*' => Some(43),
        _ => None,
    }
}

// Code 39: símbolo completo con asteriscos de inicio/parada.
pub fn code39_modules(data: &[u8]) -> Option<Vec<bool>> {
    let mut out = Vec::new();
    for c in std::iter::once(&b'*').chain(data.iter()).chain(std::iter::once(&b'*')) {
        let idx = code39_index(*c)?;
        for ch in CODE39[idx].bytes() {
            // cada elemento: barra/espacio alterno de 1 o 3 módulos
            let wide = ch == b'1';
            let w = if wide { 3 } else { 1 };
            for _ in 0..w {
                out.push(true);
            }
            out.push(false);
        }
    }
    out.pop(); // quitar el espacio final tras la última barra
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ean13_control_y_modulos() {
        assert_eq!(check_digit(b"400638133393"), 1);
        let m = ean13_modules(b"4006381333931").unwrap();
        assert_eq!(m.len(), 95);
    }

    #[test]
    fn upc_a_con_control_automatico() {
        let (d, warn) = validate(65, b"01234567890");
        assert_eq!(d, b"012345678905");
        assert!(warn.is_some());
    }

    #[test]
    fn code128_digitos_y_texto() {
        let d = code128_modules(b"123456").unwrap();
        assert_eq!(d.len(), 11 + 3 * 11 + 11 + 13); // start C + 3 pares + checksum + stop
        let t = code128_modules(b"ABC").unwrap();
        assert!(!t.is_empty());
        assert_eq!(code128_modules(b""), None);
    }

    #[test]
    fn itf_pares_y_modulos() {
        assert!(itf_modules(b"1234").is_some());
        assert_eq!(itf_modules(b"123"), None);
        assert_eq!(itf_modules(b"12A4"), None);
    }

    #[test]
    fn code39_modulos() {
        let m = code39_modules(b"ABC-123").unwrap();
        assert!(!m.is_empty());
        assert_eq!(code39_modules(b"a"), None);
        assert_eq!(code39_index(b'Z'), Some(35));
    }
}
