// Estado mecánico de la impresora: sensores, papel, cajones, energía.
//
// Los layouts de bits siguen (aproximadamente) los de ESC/POS de EPSON:
//   DLE EOT 1 / DLE ENQ / GS r 1 / ASB byte1:
//     bit0 offline, bit1 tapa cerrada, bit2 sin error, bit4 papel presente, bit5 cajon cerrado
//   DLE EOT 4: bit3 error de cabezal/recuperable
//   DLE EOT 5 / GS r 49/50: 0x00 presente, 0x0C casi fin, 0x18 fin de papel
//   GS r 2 / ASB byte2: bit0 offline
//   ASB byte3: papel (0x00 presente, 0x04 casi fin, 0x0C fin)
//   ASB byte4: bit0 cajon abierto (kick)

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimState {
    pub paper_out: bool,
    pub near_end: bool,
    pub cover_open: bool,
    pub error: bool,
    pub drawer_open: bool,
    pub drawer2_open: bool,
    pub sleeping: bool,
    pub cutter_jam: bool,
    pub offline: bool,
    pub paper_mm: f32,
}

impl Default for SimState {
    fn default() -> Self {
        SimState {
            paper_out: false,
            near_end: false,
            cover_open: false,
            error: false,
            drawer_open: false,
            drawer2_open: false,
            sleeping: false,
            cutter_jam: false,
            offline: false,
            paper_mm: Self::ROLL_MM,
        }
    }
}

// "DIP switches" / interruptores de memoria: config de fábrica del modelo,
// como los interruptores físicos de las térmicas reales.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DipSwitches {
    // Código de codepage inicial (código de ESC t; 0 = CP437).
    pub initial_codepage: u8,
    // Corte automático al final de página.
    pub auto_cut: bool,
    // Posición HRI por defecto (true = debajo del código).
    pub hri_below: bool,
    // Anchura de papel por defecto en mm (58/80/112).
    pub paper_width_mm: u16,
    // Zumbador activado/desactivado.
    pub buzzer_enabled: bool,
}

impl Default for DipSwitches {
    fn default() -> Self {
        DipSwitches {
            initial_codepage: 0,
            auto_cut: true,
            hri_below: true,
            paper_width_mm: 80,
            buzzer_enabled: true,
        }
    }
}

impl DipSwitches {
    // Restablecimiento de fábrica: configuración por defecto.
    pub fn factory_defaults() -> Self {
        DipSwitches::default()
    }

    pub fn paper_width_px(&self) -> usize {
        match self.paper_width_mm {
            58 => 384,
            112 => 832,
            _ => 576,
        }
    }
}

impl SimState {
    // Rollo inicial (30 m) y umbral de "papel próximo al fin" (1,5 m).
    pub const ROLL_MM: f32 = 30_000.0;
    pub const NEAR_END_MM: f32 = 1_500.0;

    pub fn new() -> Self {
        SimState::default()
    }

    // Consume papel (en mm). Al agotarse, marca el sensor de papel.
    pub fn consume_paper(&mut self, mm: f32) {
        self.paper_mm = (self.paper_mm - mm.max(0.0)).max(0.0);
        if self.paper_mm <= 0.0 {
            self.paper_out = true;
        }
    }

    // Recarga el rollo (como pulsar el botón FEED en una real).
    pub fn reload(&mut self) {
        self.paper_mm = Self::ROLL_MM;
        self.paper_out = false;
        self.near_end = false;
    }

    pub fn is_paper_out(&self) -> bool {
        self.paper_out || self.paper_mm <= 0.0
    }

    pub fn is_near_end(&self) -> bool {
        self.near_end || (self.paper_mm > 0.0 && self.paper_mm < Self::NEAR_END_MM)
    }

    pub fn printer_status(&self) -> u8 {
        let mut s = 0x02 | 0x04 | 0x10 | 0x20;
        if self.cover_open {
            s &= !0x02;
        }
        if self.error {
            s &= !0x04;
            s |= 0x01;
        }
        if self.sleeping || self.offline {
            s |= 0x01; // offline mientras duerme o en modo offline
        }
        if self.is_paper_out() {
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
        } else if self.cover_open || self.cutter_jam {
            0x02
        } else {
            0x00
        }
    }

    // DLE EOT 9: sensor de recuperación (tapa/cuchilla).
    pub fn recovery_sensor(&self) -> u8 {
        if self.error {
            0x08
        } else if self.cover_open {
            0x02
        } else if self.cutter_jam {
            0x01
        } else {
            0x00
        }
    }

    // DLE EOT 7: sensor de fin de rollo (papel presente/agotado).
    pub fn roll_end_sensor(&self) -> u8 {
        if self.is_paper_out() {
            0x18
        } else {
            0x00
        }
    }

    // DLE EOT 8: sensor de papel próximo al final del rollo.
    pub fn near_end_sensor(&self) -> u8 {
        if self.is_paper_out() {
            0x18
        } else if self.is_near_end() {
            0x0C
        } else {
            0x00
        }
    }

    pub fn paper_sensor(&self) -> u8 {
        if self.is_paper_out() {
            0x18
        } else if self.is_near_end() {
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
        if self.is_paper_out() {
            s &= !0x10;
        }
        s
    }

    pub fn gs_r2(&self) -> u8 {
        if self.error || self.cover_open || self.sleeping || self.offline {
            0x01
        } else {
            0x00
        }
    }

    // Estado del buffer de recepción (BSB). 0x00 = vacío, 0x10 = ocupado/parcial.
    pub fn buffer_state(&self, busy: bool) -> u8 {
        if busy {
            0x10
        } else {
            0x00
        }
    }

    pub fn asb(&self) -> [u8; 4] {
        [
            self.printer_status(),
            self.gs_r2(),
            if self.is_paper_out() {
                0x0C
            } else if self.is_near_end() {
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

    #[test]
    fn sensores_de_papel_y_cuchilla() {
        let s = SimState::default();
        assert_eq!(s.roll_end_sensor(), 0x00);
        assert_eq!(s.near_end_sensor(), 0x00);
        assert_eq!(s.recovery_sensor(), 0x00);

        let mut casi = SimState::default();
        casi.paper_mm = 1000.0; // por debajo del umbral de 1,5 m
        assert_eq!(casi.roll_end_sensor(), 0x00);
        assert_eq!(casi.near_end_sensor(), 0x0C);
        assert!(casi.is_near_end());

        let mut agotado = SimState::default();
        agotado.paper_mm = 0.0;
        assert_eq!(agotado.roll_end_sensor(), 0x18);
        assert_eq!(agotado.near_end_sensor(), 0x18);
        assert!(agotado.is_paper_out());

        let mut atascada = SimState::default();
        atascada.cutter_jam = true;
        assert_eq!(atascada.recovery_sensor(), 0x01);
        assert_eq!(atascada.recovery_status(), 0x02);
    }

    #[test]
    fn consumo_y_recarga_de_papel() {
        let mut s = SimState::default();
        let antes = s.paper_mm;
        s.consume_paper(100.0);
        assert!((s.paper_mm - (antes - 100.0)).abs() < 0.001);
        assert!(!s.is_paper_out());
        // Consumo que agota el rollo
        s.consume_paper(s.paper_mm + 1.0);
        assert!(s.is_paper_out());
        // Recargar restaura el sensor
        s.reload();
        assert!(!s.is_paper_out());
        assert_eq!(s.paper_mm, SimState::ROLL_MM);
    }

    #[test]
    fn sleep_es_offline_y_drawer2_no_afecta_status() {
        let mut s = SimState::default();
        s.sleeping = true;
        assert_eq!(s.printer_status() & 0x01, 0x01);
        assert_eq!(s.gs_r2(), 0x01);
        // El cajón 2 no altera los bytes de estado estándar (solo el 1)
        s.sleeping = false;
        s.drawer2_open = true;
        assert_eq!(s.printer_status(), 0x36);
        assert_eq!(s.asb()[3], 0x00);
        s.drawer_open = true;
        assert_eq!(s.asb()[3], 0x01);
    }

    #[test]
    fn offline_por_gs_o() {
        let mut s = SimState::default();
        s.offline = true;
        assert_eq!(s.printer_status() & 0x01, 0x01);
        assert_eq!(s.gs_r2(), 0x01);
        assert_eq!(s.printer_status() & 0x04, 0x04, "sin error de cabezal");
    }

    #[test]
    fn buffer_state_refleja_busy() {
        let s = SimState::default();
        assert_eq!(s.buffer_state(false), 0x00);
        assert_eq!(s.buffer_state(true), 0x10);
    }
}
