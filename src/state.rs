use std::collections::VecDeque;
use std::sync::Mutex;

use crate::model::PrinterModel;
use crate::render::RenderedPage;

pub use crate::memory::PrinterMemory;
pub use crate::sim::{DipSwitches, SimState};

pub struct Job {
    pub id: u64,
    pub when: String,
    pub peer: String,
    pub size: usize,
    pub raw: Vec<u8>,
    pub page: Option<RenderedPage>,
    pub summary: Vec<String>,
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
    pub model: PrinterModel,
    pub power_on: bool,
    pub hex_dump: bool,
    pub instant_print: bool,
    pub print_speed_mm_s: u32,
    // Mecánica de flujo real: cola de trabajos pendientes de imprimir.
    pub print_queue: VecDeque<u64>,
    pub printing: bool,
    // Botón PAUSE del panel (pausa la impresión en curso).
    pub paused: bool,
    // Botones del panel habilitados (ESC 8 / ESC c 5).
    pub panel_enabled: bool,
    // DIP switches / memoria de configuración de fábrica.
    pub dip: DipSwitches,
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
            sim: SimState::new(),
            memory: PrinterMemory::default(),
            model: PrinterModel::EpsonTmT88V,
            power_on: true,
            hex_dump: false,
            instant_print: false,
            print_speed_mm_s: 100,
            print_queue: VecDeque::new(),
            printing: false,
            paused: false,
            panel_enabled: true,
            dip: DipSwitches::default(),
        }
    }

    pub fn log(&mut self, msg: String) {
        self.logs.push_back(format!("{} {}", now_hm(), msg));
        if self.logs.len() > 500 {
            self.logs.pop_front();
        }
    }

    // ¿La impresora está lista para imprimir (no hay condición de offline)?
    pub fn is_printable(&self) -> bool {
        self.power_on
            && !self.paused
            && !self.sim.is_paper_out()
            && !self.sim.cover_open
            && !self.sim.error
            && !self.sim.sleeping
            && !self.sim.cutter_jam
            && !self.sim.offline
    }

    // Estado del buffer de recepción para DLE EOT 10..19.
    pub fn buffer_fill(&self) -> u8 {
        self.sim.buffer_state(self.printing || !self.print_queue.is_empty())
    }

    // Restablecimiento de fábrica: DIP por defecto, memoria NV vacía y rollo lleno.
    pub fn factory_reset(&mut self) {
        self.dip = DipSwitches::factory_defaults();
        self.memory = PrinterMemory::default();
        self.sim.reload();
        self.panel_enabled = true;
        self.paused = false;
        self.print_queue.clear();
        self.log("Restablecimiento de fábrica aplicado (DIP + NV + rollo)".to_string());
    }
}

pub fn now_hm() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

pub type Shared = std::sync::Arc<Mutex<SharedState>>;
