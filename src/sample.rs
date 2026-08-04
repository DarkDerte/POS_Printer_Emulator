pub fn demo_receipt() -> Vec<u8> {
    let mut d: Vec<u8> = Vec::new();

    push_esc(&mut d, b'@');
    push_esc(&mut d, b't');
    d.push(0);

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
    push_esc(&mut d, b'i');

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
