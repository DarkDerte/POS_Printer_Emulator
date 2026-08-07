use std::collections::VecDeque;
use std::sync::Mutex;

use crate::parser::LogoItem;
use crate::render::RenderedPage;

// Memoria de la impresora que persiste entre trabajos (como la NV/flash real):
// logos NV, fuentes descargadas y densidad de impresión.
#[derive(Clone, Debug, Default)]
pub struct PrinterMemory {
    pub logo: Option<LogoItem>,
    pub user_font: Vec<(u8, Vec<u8>)>,
    pub density: u8,
}

pub struct Job {
    pub id: u64,
    pub when: String,
    pub peer: String,
    pub size: usize,
    pub raw: Vec<u8>,
    pub page: Option<RenderedPage>,
    pub summary: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SimState {
    pub paper_out: bool,
    pub near_end: bool,
    pub cover_open: bool,
    pub error: bool,
    pub drawer_open: bool,
}

pub struct SharedState {
    pub jobs: Vec<Job>,
    pub logs: VecDeque<String>,
    pub total_bytes: u64,
    pub next_id: u64,
    pub paper_width: usize,
    pub server_error: Option<String>,
    pub sim: SimState,
    pub memory: PrinterMemory,
}

impl SharedState {
    pub fn new() -> Self {
        SharedState {
            jobs: Vec::new(),
            logs: VecDeque::new(),
            total_bytes: 0,
            next_id: 1,
            paper_width: 576,
            server_error: None,
            sim: SimState::default(),
            memory: PrinterMemory::default(),
        }
    }

    pub fn log(&mut self, msg: String) {
        self.logs.push_back(format!("{} {}", now_hm(), msg));
        if self.logs.len() > 500 {
            self.logs.pop_front();
        }
    }
}

pub fn now_hm() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

pub type Shared = std::sync::Arc<Mutex<SharedState>>;

// Los layouts de bits siguen (aproximadamente) los de ESC/POS de EPSON:
//   DLE EOT 1 / DLE ENQ / GS r 1 / ASB byte1:
//     bit0 offline, bit1 tapa cerrada, bit2 sin error, bit4 papel presente, bit5 cajon cerrado
//   DLE EOT 4: bit3 error de cabezal/recuperable
//   DLE EOT 5 / GS r 49/50: 0x00 presente, 0x0C casi fin, 0x18 fin de papel
//   GS r 2 / ASB byte2: bit0 offline
//   ASB byte3: papel (0x00 presente, 0x04 casi fin, 0x0C fin)
//   ASB byte4: bit0 cajon abierto (kick)
impl SimState {
    pub fn printer_status(&self) -> u8 {
        let mut s = 0x02 | 0x04 | 0x10 | 0x20;
        if self.cover_open {
            s &= !0x02;
        }
        if self.error {
            s &= !0x04;
            s |= 0x01;
        }
        if self.paper_out {
            s &= !0x10;
        }
        if self.drawer_open {
            s &= !0x20;
        }
        s
    }

    pub fn error_status(&self) -> u8 {
        if self.error {
            0x08
        } else {
            0x00
        }
    }

    pub fn recovery_status(&self) -> u8 {
        if self.error {
            0x01
        } else {
            0x00
        }
    }

    pub fn paper_sensor(&self) -> u8 {
        if self.paper_out {
            0x18
        } else if self.near_end {
            0x0C
        } else {
            0x00
        }
    }

    pub fn gs_r1(&self) -> u8 {
        let mut s = 0x08 | 0x10;
        if self.error {
            s &= !0x08;
        }
        if self.paper_out {
            s &= !0x10;
        }
        s
    }

    pub fn gs_r2(&self) -> u8 {
        if self.error || self.cover_open {
            0x01
        } else {
            0x00
        }
    }

    pub fn asb(&self) -> [u8; 4] {
        [
            self.printer_status(),
            self.gs_r2(),
            if self.paper_out {
                0x0C
            } else if self.near_end {
                0x04
            } else {
                0x00
            },
            if self.drawer_open {
                0x01
            } else {
                0x00
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estado_online_por_defecto() {
        let s = SimState::default();
        assert_eq!(s.printer_status(), 0x36);
        assert_eq!(s.gs_r1(), 0x18);
        assert_eq!(s.gs_r2(), 0x00);
        assert_eq!(s.paper_sensor(), 0x00);
        assert_eq!(s.asb(), [0x36, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn estado_con_papel_agotado() {
        let mut s = SimState::default();
        s.paper_out = true;
        assert_eq!(s.printer_status() & 0x10, 0);
        assert_eq!(s.paper_sensor(), 0x18);
        assert_eq!(s.gs_r1() & 0x10, 0);
        assert_eq!(s.asb()[2], 0x0C);
    }

    #[test]
    fn estado_con_error_y_tapa_abierta() {
        let mut s = SimState::default();
        s.error = true;
        s.cover_open = true;
        assert_eq!(s.printer_status() & 0x01, 0x01);
        assert_eq!(s.printer_status() & 0x02, 0);
        assert_eq!(s.error_status(), 0x08);
        assert_eq!(s.gs_r2(), 0x01);
        assert_eq!(s.asb()[1], 0x01);
    }
}
