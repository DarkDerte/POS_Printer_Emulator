use qrcode::{Color, EcLevel, QrCode};

/// Bitmap (sin zona de silencio) de un código 2D generado.
pub struct Bitmap2D {
    pub width: usize,
    pub height: usize,
    /// Fila principal (row-major), `true` = módulo oscuro.
    pub dark: Vec<bool>,
}

const ECC_COUNT: [usize; 9] = [2, 4, 8, 16, 32, 64, 128, 256, 512];
const PDF417_MAX_CODEWORDS: usize = 30 * 90;

fn rows_for(count: usize, cols: u8, level: u8) -> usize {
    let n = count + ECC_COUNT[level as usize];
    (n + cols as usize - 1) / cols as usize
}

/// Genera un QR. `ecc`: 0/48=L, 1/49=M, 2/50=Q, 3/51=H (cualquier otro valor = M).
pub fn qr(data: &[u8], ecc: u8) -> Option<Bitmap2D> {
    if data.is_empty() {
        return None;
    }
    let level = match ecc {
        0 | 48 => EcLevel::L,
        2 | 50 => EcLevel::Q,
        3 | 51 => EcLevel::H,
        _ => EcLevel::M,
    };
    let code = QrCode::with_error_correction_level(data, level).ok()?;
    let w = code.width();
    let dark: Vec<bool> = code.to_colors().iter().map(|&c| c == Color::Dark).collect();
    Some(Bitmap2D {
        width: w,
        height: w,
        dark,
    })
}

/// Genera un PDF417. `cols`/`rows`/`ecc` opcionales (se autodimensiona si faltan).
pub fn pdf417(
    data: &[u8],
    cols: Option<u8>,
    rows: Option<u8>,
    ecc: Option<u8>,
) -> Option<Bitmap2D> {
    if data.is_empty() {
        return None;
    }
    let mut scratch = vec![0u16; PDF417_MAX_CODEWORDS];
    let count = pdf417::PDF417Encoder::new(&mut scratch, false)
        .append_bytes(data)
        .count();
    if count == 0 || count > PDF417_MAX_CODEWORDS {
        return None;
    }
    let cols = cols.unwrap_or(30).clamp(1, 30);
    let level = if let Some(l) = ecc {
        l.clamp(0, 8)
    } else {
        let mut l = 8u8;
        while l > 0 && rows_for(count, cols, l) > 90 {
            l -= 1;
        }
        l
    };
    let rows = if let Some(r) = rows {
        r.clamp(3, 90)
    } else {
        rows_for(count, cols, level).clamp(3, 90) as u8
    };
    let capacity = rows as usize * cols as usize;
    if capacity < count + ECC_COUNT[level as usize] {
        return None;
    }
    let mut codewords = vec![0u16; capacity];
    let enc = pdf417::PDF417Encoder::new(&mut codewords, false).append_bytes(data);
    enc.seal(level);
    let width = 17 * cols as usize + 69;
    let height = rows as usize;
    let mut dark = vec![false; width * height];
    let pdf = pdf417::PDF417::new(&codewords, rows, cols, level);
    pdf.render(dark.as_mut_slice());
    Some(Bitmap2D {
        width,
        height,
        dark,
    })
}
