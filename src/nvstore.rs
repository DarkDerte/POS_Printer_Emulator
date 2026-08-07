// Persistencia NV real: guarda logos, fuentes descargadas y densidad en un
// fichero binario para que sobrevivan entre sesiones (como la flash de la
// impresora). Formato propio, sin dependencias.
//
//   "POSNV1" (6)
//   logo:    1 byte presente, u32le width, u32le height, u32le len, bytes
//   fonts:   u32le count, por fuente: 1 byte carácter, u32le len, bytes
//   density: 1 byte
use std::io;
use std::path::PathBuf;

use crate::parser::LogoItem;
use crate::state::PrinterMemory;

const MAGIC: &[u8; 6] = b"POSNV1";

pub fn nv_path() -> PathBuf {
    if let Some(p) = std::env::var_os("POS_NV_PATH") {
        return PathBuf::from(p);
    }
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("pos_printer_emulator"));
    base.join("pos_printer_emulator")
        .join("nv.bin")
}

pub fn save(memory: &PrinterMemory) -> io::Result<()> {
    save_at(memory, &nv_path())
}

pub fn save_at(memory: &PrinterMemory, path: &PathBuf) -> io::Result<()> {
    let mut out = Vec::with_capacity(128 + memory.logo.as_ref().map_or(0, |l| l.data.len()));
    out.extend_from_slice(MAGIC);
    match &memory.logo {
        Some(l) => {
            out.push(1);
            out.extend_from_slice(&(l.width as u32).to_le_bytes());
            out.extend_from_slice(&(l.height as u32).to_le_bytes());
            out.extend_from_slice(&(l.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&l.data);
        }
        None => out.push(0),
    }
    out.extend_from_slice(&(memory.user_font.len() as u32).to_le_bytes());
    for (ch, bytes) in &memory.user_font {
        out.push(*ch);
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(bytes);
    }
    out.push(memory.density);

    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(path, out)
}

pub fn load() -> Option<PrinterMemory> {
    load_at(&nv_path())
}

pub fn load_at(path: &PathBuf) -> Option<PrinterMemory> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() < 7 || &bytes[..6] != MAGIC {
        return None;
    }
    let mut tail = bytes;
    tail.drain(..6);
    let mut it = tail.into_iter();
    let u32_le = |it: &mut std::vec::IntoIter<u8>| -> Option<u32> {
        let mut b = [0u8; 4];
        for slot in &mut b {
            *slot = it.next()?;
        }
        Some(u32::from_le_bytes(b))
    };
    let logo = match it.next()? {
        0 => None,
        _ => {
            let width = u32_le(&mut it)? as usize;
            let height = u32_le(&mut it)? as usize;
            let len = u32_le(&mut it)? as usize;
            let mut data = vec![0u8; len];
            for b in &mut data {
                *b = it.next()?;
            }
            Some(LogoItem { data, width, height })
        }
    };
    let nf = u32_le(&mut it)? as usize;
    let mut user_font = Vec::with_capacity(nf);
    for _ in 0..nf {
        let ch = it.next()?;
        let len = u32_le(&mut it)? as usize;
        let mut data = vec![0u8; len];
        for b in &mut data {
            *b = it.next()?;
        }
        user_font.push((ch, data));
    }
    let density = it.next()?;
    Some(PrinterMemory {
        logo,
        user_font,
        density,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_con_logo_fuente_y_densidad() {
        let dir = std::env::temp_dir().join(format!("pos_nv_test_r_{}", std::process::id()));
        let path = dir.join("nv.bin");
        let _ = std::fs::remove_file(&path);
        let mem = PrinterMemory {
            logo: Some(LogoItem {
                data: vec![0xAA, 0x55, 0xFF],
                width: 16,
                height: 8,
            }),
            user_font: vec![(b'A', vec![1, 2, 3, 4, 5, 6, 7, 8])],
            density: 42,
        };
        assert!(save_at(&mem, &path).is_ok());
        let loaded = load_at(&path).unwrap();
        assert_eq!(loaded.logo.unwrap().data, vec![0xAA, 0x55, 0xFF]);
        assert_eq!(loaded.user_font, vec![(b'A', vec![1, 2, 3, 4, 5, 6, 7, 8])]);
        assert_eq!(loaded.density, 42);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn vacio_y_corrupto() {
        let dir = std::env::temp_dir().join(format!("pos_nv_test_v_{}", std::process::id()));
        let path = dir.join("nv.bin");
        let _ = std::fs::remove_file(&path);
        assert!(save_at(&PrinterMemory::default(), &path).is_ok());
        assert!(load_at(&path).unwrap().logo.is_none());
        std::fs::write(&path, b"NO-MAGIC").unwrap();
        assert!(load_at(&path).is_none());
        let _ = std::fs::remove_file(&path);
    }
}
