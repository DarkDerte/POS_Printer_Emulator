pub fn demo_receipt() -> Vec<u8> {
    let mut d: Vec<u8> = Vec::new();

    push_esc(&mut d, b'@');
    push_esc(&mut d, b't');
    d.push(0);

    // Consultas de estado en tiempo real (el servidor responde al socket)
    d.extend_from_slice(&[0x10, 0x04, 0x01, 0x10, 0x04, 0x05, 0x10, 0x05, 0x1D, 0x72, 0x01]);

    // Código QR (GS ( k 49)
    let qr_text = b"TIENDA DEMO POS|TICKET 001|11,60 EUR";
    let qr_pl = 6 + qr_text.len();
    d.extend_from_slice(&[0x1D, 0x28, 0x6B]);
    d.push(qr_pl as u8);
    d.push(0);
    d.extend_from_slice(&[0x31, 0x43, 0x30]);
    d.extend_from_slice(&(qr_text.len() as u16).to_le_bytes());
    d.push(0);
    d.extend_from_slice(qr_text);
    d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x31, 0x50]);

    // PDF417 (GS ( k 48)
    let pdf_text = b"DEMO PDF417|1234567890|EUR 11.60";
    let pdf_pl = 6 + pdf_text.len();
    d.extend_from_slice(&[0x1D, 0x28, 0x6B]);
    d.push(pdf_pl as u8);
    d.push(0);
    d.extend_from_slice(&[0x30, 0x43, 0x30]);
    d.extend_from_slice(&(pdf_text.len() as u16).to_le_bytes());
    d.push(0);
    d.extend_from_slice(pdf_text);
    d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x30, 0x50]);

    push_esc(&mut d, b'a');
    d.push(1);
    push_esc(&mut d, b'E');
    d.push(1);
    d.extend_from_slice(b"TIENDA DEMO POS\n");
    push_esc(&mut d, b'E');
    d.push(0);
    d.extend_from_slice(b"Calle Principal 123\n");
    d.extend_from_slice(b"Ciudad, CP 28000\n");
    d.extend_from_slice(b"Tel: 555-1234\n");
    push_esc(&mut d, b'a');
    d.push(0);
    push_esc(&mut d, b'd');
    d.push(1);

    d.extend_from_slice(b"--------------------------------\n");
    d.extend_from_slice(b"LECHE 1L        x1     1.25\n");
    d.extend_from_slice(b"PAN INTEGRAL    x2     2.40\n");
    d.extend_from_slice(b"CAFE MOLIDO     x1     4.80\n");
    d.extend_from_slice(b"FRUTA VARIADA   x1     3.15\n");
    push_esc(&mut d, b'd');
    d.push(1);
    d.extend_from_slice(b"--------------------------------\n");

    // Logo NV definido con FS q e impreso con FS p
    let lw: u16 = 32;
    let lh: u16 = 16;
    d.extend_from_slice(&[0x1C, 0x71, 0x01]);
    d.extend_from_slice(&lw.to_le_bytes());
    d.extend_from_slice(&lh.to_le_bytes());
    let mut logo = vec![0u8; ((lw as usize + 7) / 8) * lh as usize];
    for y in 0..lh as usize {
        for x in 0..lw as usize {
            if (x / 4 + y) % 3 != 0 {
                logo[y * ((lw as usize + 7) / 8) + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    d.extend_from_slice(&logo);
    d.extend_from_slice(&[0x1C, 0x70, 0x00, 0x00]);
    push_esc(&mut d, b'd');
    d.push(1);

    // Inverso, cursiva y subrayado doble
    push_gs(&mut d, b'B');
    d.push(1);
    d.extend_from_slice(b"PROMOCION SEMANA 20%\n");
    push_gs(&mut d, b'B');
    d.push(0);
    push_esc(&mut d, b'4');
    d.extend_from_slice(b"* Oferta valida en caja *\n");
    push_esc(&mut d, b'5');
    push_esc(&mut d, b'-');
    d.push(2);
    d.extend_from_slice(b"CONDICIONES EN TIENDA\n");
    push_esc(&mut d, b'-');
    d.push(0);

    push_esc(&mut d, b'a');
    d.push(2);
    d.extend_from_slice(b"TOTAL A PAGAR\n");
    push_esc(&mut d, b'!');
    d.push(0x21);
    push_esc(&mut d, b'a');
    d.push(1);
    d.extend_from_slice(b"11,60 EUR\n");
    push_esc(&mut d, b'!');
    d.push(0);
    push_esc(&mut d, b'a');
    d.push(0);
    push_esc(&mut d, b'd');
    d.push(2);

    push_esc(&mut d, b'a');
    d.push(1);
    push_esc(&mut d, b'-');
    d.push(1);
    d.extend_from_slice(b"GRACIAS POR SU VISITA\n");
    push_esc(&mut d, b'-');
    d.push(0);
    push_esc(&mut d, b'd');
    d.push(2);

    push_gs(&mut d, b'w');
    d.push(3);
    push_gs(&mut d, b'h');
    d.push(60);
    push_gs(&mut d, b'H');
    d.push(2);
    push_esc(&mut d, b'a');
    d.push(1);
    push_gs(&mut d, b'k');
    d.push(67);
    d.extend_from_slice(b"1234567");
    d.push(0);
    // Code 128 (m=73) e ITF (m=70) con módulos reales
    push_gs(&mut d, b'k');
    d.push(73);
    d.extend_from_slice(b"ABC-128");
    d.push(0);
    push_gs(&mut d, b'k');
    d.push(70);
    d.extend_from_slice(b"123456");
    d.push(0);
    push_esc(&mut d, b'd');
    d.push(3);

    push_gs(&mut d, b'v');
    d.push(0x30);
    d.push(0x30);
    let rw: u16 = 96;
    let rh: u16 = 24;
    d.extend_from_slice(&rw.to_le_bytes());
    d.extend_from_slice(&rh.to_le_bytes());
    let row_bytes = (rw as usize + 7) / 8;
    let mut img = vec![0u8; row_bytes * rh as usize];
    for y in 0..rh as usize {
        for x in 0..rw as usize {
            if (x / 8 + y) % 3 == 0 {
                img[y * row_bytes + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    d.extend_from_slice(&img);

    push_esc(&mut d, b'd');
    d.push(4);

    // ASB (estado automático) + transmitir QR y PDF417 (GS ( k fn 81)
    d.extend_from_slice(&[0x1D, 0x61, 0x01]);
    d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x31, 0x51]);
    d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x30, 0x51]);

    // Logo raster NV con GS 8 L e impresión escalada GS / 49 (doble ancho)
    let rlw: u16 = 96;
    let rlh: u16 = 8;
    let rwb = (rlw as usize + 7) / 8; // ancho en bytes (12) -> 96 px
    let rn = rwb * rlh as usize;
    d.extend_from_slice(&[0x1D, 0x38, 0x4C]);
    d.push((rn & 0xFF) as u8);
    d.push((rn >> 8) as u8);
    d.extend_from_slice(&(rwb as u16).to_le_bytes());
    d.extend_from_slice(&(rlh as u16).to_le_bytes());
    let mut raster = vec![0u8; rn];
    for y in 0..rlh as usize {
        for x in 0..rlw as usize {
            if (x / 6 + y) % 2 == 0 {
                raster[y * rwb + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }
    d.extend_from_slice(&raster);
    d.extend_from_slice(&[0x1D, 0x38, 0x48, 0x00, 0x00]);
    d.extend_from_slice(&[0x1D, 0x2F, 0x31]);
    push_esc(&mut d, b'd');
    d.push(2);

    // Modo página GS P + texto + ESC FF
    d.extend_from_slice(&[0x1D, 0x50]);
    d.extend_from_slice(&(576u16).to_le_bytes());
    d.extend_from_slice(&(300u16).to_le_bytes());
    push_esc(&mut d, b'$');
    d.extend_from_slice(&(40u16).to_le_bytes());
    d.extend_from_slice(b"BLOQUE DE PAGINA\n");
    push_esc(&mut d, b'a');
    d.push(1);
    d.extend_from_slice(b"Etiqueta en modo pagina\n");
    push_esc(&mut d, b'!');
    d.push(0);
    d.push(0x0C);

    // Buzzer ESC ( C fn=06
    d.extend_from_slice(&[0x1B, 0x28, 0x43, 0x04, 0x00, 0x06, 0x03, 0x01, 0x00]);

    // Fuente descargada ESC & (1 char 'A', 12 bytes)
    d.extend_from_slice(&[0x1B, 0x26, 0x01, 0x41, 0x42]);
    d.extend_from_slice(&[0u8; 24]);

    push_gs(&mut d, b'V');
    d.push(49);

    d
}

fn push_esc(d: &mut Vec<u8>, cmd: u8) {
    d.push(0x1B);
    d.push(cmd);
}

fn push_gs(d: &mut Vec<u8>, cmd: u8) {
    d.push(0x1D);
    d.push(cmd);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_se_parsea_y_renderea() {
        let d = demo_receipt();
        let doc = crate::parser::Parser::parse(&d);
        let fonts = crate::render::Fonts::load();
        let page = crate::render::render(&doc.items, 576, &fonts);
        assert!(page.height > 50, "la demo debe producir contenido");
        assert!(
            doc.summary.iter().any(|s| s.contains("Buzzer")),
            "la demo debe activar el zumbador"
        );
        assert!(
            doc.summary.iter().any(|s| s.contains("Modo página GS P")),
            "la demo debe entrar en modo página"
        );
    }
}
