use crate::codepages::{self, Codepage};
use crate::state::PrinterMemory;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontSel {
    A,
    B,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Style {
    pub font: FontSel,
    pub bold: bool,
    pub double_strike: bool,
    pub underline: bool,
    pub underline2: bool,
    pub reverse: bool,
    pub italic: bool,
    pub char_spacing: u8,
    pub size_x: u8,
    pub size_y: u8,
    pub condensed: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            font: FontSel::A,
            bold: false,
            double_strike: false,
            underline: false,
            underline2: false,
            reverse: false,
            italic: false,
            char_spacing: 0,
            size_x: 1,
            size_y: 1,
            condensed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextRun {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug)]
pub struct TextItem {
    pub runs: Vec<TextRun>,
    pub align: Alignment,
}

#[derive(Clone, Debug)]
pub struct RasterItem {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub align: Alignment,
}

#[derive(Clone, Debug)]
pub struct BitImageItem {
    pub data: Vec<u8>,
    pub width: usize,
    pub bytes_per_col: usize,
    pub spread: usize,
    pub align: Alignment,
}

#[derive(Clone, Debug)]
pub struct BarcodeItem {
    #[allow(dead_code)]
    pub m: u8,
    pub data: Vec<u8>,
    pub height: u32,
    pub hri: u8,
    pub hri_font: u8,
    pub module: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Barcode2DKind {
    Qr,
    Pdf417,
}

#[derive(Clone, Debug)]
pub struct Barcode2DItem {
    pub kind: Barcode2DKind,
    pub data: Vec<u8>,
    pub module: u32,
    pub ecc: u8,
    pub cols: Option<u8>,
    pub rows: Option<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogoItem {
    pub data: Vec<u8>,
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug)]
pub struct PrintArea {
    pub left: f32,
    pub width: f32,
}

#[derive(Clone, Debug)]
pub enum Item {
    Text(TextItem),
    Raster(RasterItem),
    BitImage(BitImageItem),
    FeedLines(u32),
    FeedDots(u32),
    #[allow(dead_code)]
    Cut { partial: bool },
    Drawer { t: u8 },
    Barcode(BarcodeItem),
    Barcode2D(Barcode2DItem),
    SetLineSpacing(Option<u32>),
    SetPos(f32),
    MoveX(f32),
    SetLeftMargin(f32),
    SetPrintArea(PrintArea),
    PageStart { width: f32, height: f32, dir: u8 },
    PageEnd { height: f32 },
    StoreLogo(LogoItem),
    PrintLogo { scale_x: usize, scale_y: usize },
    Init,
}

#[derive(Clone, Debug)]
pub struct ParsedDoc {
    pub items: Vec<Item>,
    pub summary: Vec<String>,
    pub transmit: Vec<Vec<u8>>,
    pub memory: PrinterMemory,
    // Nº de veces que se debe imprimir el documento (GS # n).
    #[cfg_attr(not(test), allow(dead_code))]
    pub copies: u8,
    // Solicita una página de auto-test (GS ( E fn=5).
    pub auto_test: bool,
    // Nº total de timbres solicitados por el zumbador (ESC ( C fn=0x06).
    pub buzz: u32,
    // Estado de los botones del panel tras ESC 8 / ESC c 5.
    pub panel_enabled: bool,
    // Mayor tiempo de pulso de cajón solicitado (ESC p / GS p), en ms.
    pub drawer_pulse_ms: Option<u16>,
}

pub struct Parser {
    align: Alignment,
    style: Style,
    line: Vec<TextRun>,
    line_spacing: Option<u32>,
    codepage: Codepage,
    left_margin: i32,
    tabs: Vec<u8>,
    skip_perforation: u8,
    page_lines: u16,
    user_font_active: bool,
    bar_module: u32,
    bar_height: u32,
    hri: u8,
    hri_font: u8,
    items: Vec<Item>,
    summary: Vec<String>,
    transmit: Vec<Vec<u8>>,
    n_ignore: u32,
    fed_blank: bool,
    kanji: bool,
    page_mode: bool,
    page_height: usize,
    page_dir: u8,
    logo: Option<LogoItem>,
    logos: Vec<LogoItem>,
    density: u8,
    user_font: Vec<(u8, Vec<u8>)>,
    user_kanji: Vec<(u16, Vec<u8>)>,
    qr_data: Vec<u8>,
    qr_module: u32,
    qr_ecc: u8,
    pdf_data: Vec<u8>,
    pdf_module: u32,
    pdf_cols: Option<u8>,
    pdf_rows: Option<u8>,
    pdf_ecc: Option<u8>,
    model: crate::model::PrinterModel,
    copies: u8,
    auto_test: bool,
    buzz: u32,
    panel_enabled: bool,
    right_margin: Option<i32>,
    macro_bytes: Option<Vec<u8>>,
    macro_depth: u8,
    // Ajustes de fábrica (DIP) aplicados como valores por defecto.
    auto_cut: bool,
    hri_default: u8,
}

impl Default for Parser {
    fn default() -> Self {
        Parser {
            align: Alignment::Left,
            style: Style::default(),
            line: Vec::new(),
            line_spacing: None,
            codepage: Codepage::Cp437,
            left_margin: 0,
            tabs: Vec::new(),
            skip_perforation: 0,
            page_lines: 0,
            user_font_active: false,
            bar_module: 2,
            bar_height: 50,
            hri: 0,
            hri_font: 0,
            items: Vec::new(),
            summary: Vec::new(),
            transmit: Vec::new(),
            n_ignore: 0,
            fed_blank: false,
            kanji: false,
            page_mode: false,
            page_height: 0,
            page_dir: 0,
            logo: None,
            logos: Vec::new(),
            density: 0,
            user_font: Vec::new(),
            user_kanji: Vec::new(),
            qr_data: Vec::new(),
            qr_module: 3,
            qr_ecc: 49,
            pdf_data: Vec::new(),
            pdf_module: 2,
            pdf_cols: None,
            pdf_rows: None,
            pdf_ecc: None,
            model: crate::model::PrinterModel::EpsonTmT88V,
            copies: 1,
            auto_test: false,
            buzz: 0,
            panel_enabled: true,
            right_margin: None,
            macro_bytes: None,
            macro_depth: 0,
            auto_cut: true,
            hri_default: 0,
        }
    }
}

impl Parser {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn parse(data: &[u8]) -> ParsedDoc {
        Self::parse_with(data, &PrinterMemory::default())
    }

    pub fn parse_with(data: &[u8], memory: &PrinterMemory) -> ParsedDoc {
        let mut p = Parser::default();
        p.logo = memory.logo.clone();
        p.logos = memory.logos.clone();
        p.user_font = memory.user_font.clone();
        p.user_kanji = memory.user_kanji.clone();
        p.density = memory.density;
        p.macro_bytes = memory.macro_bytes.clone();
        p.run(data);
        p.flush_line();
        p.finish()
    }

    /// Igual que `parse_with` pero fijando el perfil de modelo.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn parse_with_model(
        data: &[u8],
        memory: &PrinterMemory,
        model: crate::model::PrinterModel,
    ) -> ParsedDoc {
        Self::parse_with_model_cp(data, memory, model, 0)
    }

    /// Igual que `parse_with_model` pero partiendo del codepage inicial del DIP.
    pub fn parse_with_model_cp(
        data: &[u8],
        memory: &PrinterMemory,
        model: crate::model::PrinterModel,
        initial_cp: u8,
    ) -> ParsedDoc {
        let mut p = Parser::default();
        p.model = model;
        p.codepage = codepages::table(initial_cp);
        p.logo = memory.logo.clone();
        p.logos = memory.logos.clone();
        p.user_font = memory.user_font.clone();
        p.user_kanji = memory.user_kanji.clone();
        p.density = memory.density;
        p.macro_bytes = memory.macro_bytes.clone();
        p.run(data);
        p.flush_line();
        p.finish()
    }

    /// Igual que `parse_with_model_cp` pero aplicando los ajustes DIP de fábrica:
    /// la posición HRI por defecto (hri_below) y si hay cuchilla automática.
    pub fn parse_with_dip(
        data: &[u8],
        memory: &PrinterMemory,
        model: crate::model::PrinterModel,
        initial_cp: u8,
        hri_below: bool,
        auto_cut: bool,
    ) -> ParsedDoc {
        let mut p = Parser::default();
        p.model = model;
        p.codepage = codepages::table(initial_cp);
        p.logo = memory.logo.clone();
        p.logos = memory.logos.clone();
        p.user_font = memory.user_font.clone();
        p.user_kanji = memory.user_kanji.clone();
        p.density = memory.density;
        p.macro_bytes = memory.macro_bytes.clone();
        p.auto_cut = auto_cut;
        p.hri_default = if hri_below { 2 } else { 0 };
        p.hri = p.hri_default;
        p.run(data);
        p.flush_line();
        p.finish()
    }

    // Construye el ParsedDoc final aplicando copias, resumen y estado mecánico.
    fn finish(self) -> ParsedDoc {
        let drawer_pulse_ms = self
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Drawer { t } => Some(*t as u16 * 2),
                _ => None,
            })
            .max();
        let mut items = self.items;
        if self.copies > 1 {
            let base = items;
            let mut out = Vec::with_capacity(base.len() * self.copies as usize);
            for _ in 0..self.copies {
                out.extend(base.iter().cloned());
            }
            items = out;
        }
        ParsedDoc {
            items,
            summary: self.summary,
            transmit: self.transmit,
            memory: PrinterMemory {
                logo: self.logo,
                logos: self.logos,
                user_font: self.user_font,
                user_kanji: self.user_kanji,
                density: self.density,
                macro_bytes: self.macro_bytes,
            },
            copies: self.copies,
            auto_test: self.auto_test,
            buzz: self.buzz,
            panel_enabled: self.panel_enabled,
            drawer_pulse_ms,
        }
    }

    fn run(&mut self, data: &[u8]) {
        let mut i = 0usize;
        while i < data.len() {
            if self.n_ignore > 0 {
                self.n_ignore -= 1;
                i += 1;
                continue;
            }
            let b = data[i];
            match b {
                0x1B => {
                    let ni = self.handle_esc(data, i);
                    if ni == usize::MAX {
                        break;
                    }
                    i = ni;
                }
                0x1D => {
                    let ni = self.handle_gs(data, i);
                    if ni == usize::MAX {
                        break;
                    }
                    i = ni;
                }
                0x1C => {
                    let ni = self.handle_fs(data, i);
                    if ni == usize::MAX {
                        break;
                    }
                    i = ni;
                }
                0x0A | 0x0C => {
                    self.flush_line();
                    self.push(Item::FeedLines(1));
                    if b == 0x0C {
                        self.summary.push("Salto de página (FF)".to_string());
                    }
                    i += 1;
                }
                0x0D => {
                    self.flush_line();
                    i += 1;
                }
                0x09 => {
                    self.handle_tab();
                    i += 1;
                }
                0x0E => {
                    // ESC SO: doble ancho
                    self.flush_line();
                    self.style.size_x = 2;
                    i += 1;
                }
                0x0F => {
                    // ESC SI: modo condensado
                    self.flush_line();
                    self.style.condensed = true;
                    i += 1;
                }
                0x14 => {
                    // ESC DC4: cancela doble ancho
                    self.flush_line();
                    self.style.size_x = 1;
                    i += 1;
                }
                0x00 | 0x11 | 0x13 | 0x04 | 0x05 | 0x06 | 0x15 | 0x10 | 0x0B => {
                    i += 1;
                }
                b if (0x20..=0x7E).contains(&b) || b >= 0x80 => {
                    if b >= 0xC0 {
                        let len = utf8_len(b);
                        if i + len <= data.len()
                            && data[i + 1..i + len].iter().all(|&c| c & 0xC0 == 0x80)
                        {
                            if let Ok(s) = std::str::from_utf8(&data[i..i + len]) {
                                self.push_utf8(s);
                                i += len;
                                continue;
                            }
                        }
                    }
                    if self.kanji && codepages::kanji_lead(b) {
                        if let Some(&t) = data.get(i + 1) {
                            if codepages::kanji_trail(t) {
                                if let Some(s) = codepages::decode_kanji_pair(b, t) {
                                    self.push_utf8(&s);
                                    i += 2;
                                    continue;
                                }
                            }
                        }
                    } else if codepages::is_double_byte(self.codepage)
                        && codepages::is_lead(self.codepage, b)
                    {
                        if let Some(&t) = data.get(i + 1) {
                            if codepages::is_trail(self.codepage, t) {
                                if let Some(s) =
                                    codepages::decode_pair(self.codepage, b, t)
                                {
                                    self.push_utf8(&s);
                                    i += 2;
                                    continue;
                                }
                            }
                        }
                    }
                    self.push_char(b);
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    fn push(&mut self, item: Item) {
        match &item {
            Item::Text(_) => self.fed_blank = false,
            Item::FeedLines(_) | Item::FeedDots(_) => {}
            _ => self.fed_blank = false,
        }
        self.items.push(item);
    }

    fn push_char(&mut self, b: u8) {
        let ch = codepages::decode(self.codepage, b);
        self.push_char_str(ch);
    }

    fn push_char_str(&mut self, ch: char) {
        match self.line.last_mut() {
            Some(last) if last.style == self.style => last.text.push(ch),
            _ => self.line.push(TextRun {
                text: ch.to_string(),
                style: self.style,
            }),
        }
    }

    fn push_utf8(&mut self, s: &str) {
        for ch in s.chars() {
            self.push_char_str(ch);
        }
    }

    fn handle_tab(&mut self) {
        // advance to next tab stop (default 8 chars)
        let total: usize = self.line.iter().map(|r| r.text.chars().count()).sum();
        let next = if let Some(&t) = self.tabs.iter().find(|&&t| (t as usize) > total) {
            t as usize
        } else {
            ((total / 8) + 1) * 8
        };
        let pad = next.saturating_sub(total);
        for _ in 0..pad {
            self.push_char(0x20);
        }
    }

    fn flush_line(&mut self) {
        if self.line.is_empty() {
            return;
        }
        let runs = std::mem::take(&mut self.line);
        self.push(Item::Text(TextItem {
            runs,
            align: self.align,
        }));
    }

    fn reset_defaults(&mut self) {
        self.flush_line();
        self.align = Alignment::Left;
        self.style = Style::default();
        self.line_spacing = None;
        self.codepage = Codepage::Cp437;
        self.left_margin = 0;
        self.tabs.clear();
        self.skip_perforation = 0;
        self.page_lines = 0;
        self.user_font_active = false;
        self.bar_module = 2;
        self.bar_height = 50;
        self.hri = self.hri_default;
        self.hri_font = 0;
        self.page_mode = false;
        self.page_height = 0;
        self.page_dir = 0;
        self.kanji = false;
        self.qr_data.clear();
        self.qr_module = 3;
        self.qr_ecc = 49;
        self.pdf_data.clear();
        self.pdf_module = 2;
        self.pdf_cols = None;
        self.pdf_rows = None;
        self.pdf_ecc = None;
        self.copies = 1;
        self.right_margin = None;
        self.panel_enabled = true;
    }

    /// Emite una orden de corte; si el DIP de auto-corte está desactivado (sin
    /// cuchilla instalada) solo se registra el avance sin dibujar la línea.
    fn push_cut(&mut self, partial: bool) {
        if self.auto_cut {
            self.push(Item::Cut { partial });
        } else {
            self.summary.push(format!(
                "Corte {} ignorado: cuchilla automática desactivada (DIP)",
                if partial { "parcial" } else { "total" }
            ));
            self.push(Item::FeedDots(12));
        }
    }

    fn handle_esc(&mut self, data: &[u8], i: usize) -> usize {
        let get = |off: usize| data.get(i + off).copied();
        let cmd = match get(1) {
            Some(c) => c,
            None => return usize::MAX,
        };
        match cmd {
            b'@' => {
                self.reset_defaults();
                self.push(Item::Init);
                i + 2
            }
            b'!' => {
                if let Some(n) = get(2) {
                    self.style.font = if n & 0x01 != 0 { FontSel::B } else { FontSel::A };
                    self.style.bold = n & 0x08 != 0;
                    self.style.size_y = if n & 0x10 != 0 { 2 } else { 1 };
                    self.style.size_x = if n & 0x20 != 0 { 2 } else { 1 };
                    self.style.reverse = n & 0x40 != 0;
                    self.style.underline = n & 0x80 != 0;
                    self.style.underline2 = n & 0x80 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'E' => {
                if let Some(n) = get(2) {
                    self.style.bold = n & 0x01 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'G' => {
                if let Some(n) = get(2) {
                    self.style.double_strike = n & 0x01 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'-' => {
                if let Some(n) = get(2) {
                    self.style.underline = n & 0x01 != 0 || n & 0x02 != 0;
                    self.style.underline2 = n & 0x02 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'4' => {
                self.style.italic = true;
                i + 2
            }
            b'5' => {
                self.style.italic = false;
                i + 2
            }
            b' ' => {
                if let Some(n) = get(2) {
                    self.style.char_spacing = n;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'$' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    let x = (a as f32) + (b as f32) * 256.0;
                    self.flush_line();
                    self.push(Item::SetPos(x));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'\\' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    let mut dx = a as i32 + (b as i32) * 256;
                    if dx >= 2048 {
                        dx -= 4096;
                    }
                    self.flush_line();
                    self.push(Item::MoveX(dx as f32));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'v' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    self.push_cut(n != 0 && n != 48);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'=' => {
                self.summary.push("Modo host ESC = ignorado".to_string());
                i + 2
            }
            b'7' => {
                // ESC 7: selecciona la fuente descargada por el usuario
                self.user_font_active = true;
                self.summary.push("ESC 7: fuente de usuario seleccionada".to_string());
                i + 2
            }
            b'6' => {
                // ESC 6: selecciona la fuente interna (default)
                self.user_font_active = false;
                self.summary.push("ESC 6: fuente interna seleccionada".to_string());
                i + 2
            }
            b'\x14' => {
                // ESC DC4: cancela el doble ancho
                self.flush_line();
                self.style.size_x = 1;
                i + 2
            }
            b'N' => {
                // ESC N n: punto de salto de perforación
                if let Some(n) = get(2) {
                    self.skip_perforation = n;
                    self.summary.push(format!("ESC N: salto de perforación {n} líneas"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'C' => {
                // ESC C n: longitud de página en líneas
                if let Some(n) = get(2) {
                    self.page_lines = n as u16;
                    self.summary.push(format!("ESC C: longitud de página {n} líneas"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'8' => {
                // ESC 8 n: activa/desactiva los botones del panel
                if let Some(n) = get(2) {
                    self.panel_enabled = n != 0;
                    self.summary.push(format!(
                        "ESC 8: botones de panel {}",
                        if n != 0 { "activados" } else { "desactivados" }
                    ));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'a' => {
                if let Some(n) = get(2) {
                    self.align = match n {
                        1 | 49 => Alignment::Center,
                        2 | 50 => Alignment::Right,
                        _ => Alignment::Left,
                    };
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'd' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    self.push(Item::FeedLines(n as u32));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'J' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    self.push(Item::FeedDots(n as u32));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'2' => {
                self.line_spacing = None;
                self.push(Item::SetLineSpacing(None));
                i + 2
            }
            b'3' => {
                if let Some(n) = get(2) {
                    self.line_spacing = Some(n as u32);
                    self.push(Item::SetLineSpacing(Some(n as u32)));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            0x0C => {
                self.flush_line();
                if self.page_mode {
                    self.summary.push("Modo página impreso (ESC FF)".to_string());
                    self.push(Item::PageEnd {
                        height: self.page_height as f32,
                    });
                    self.page_mode = false;
                } else {
                    self.push(Item::FeedLines(1));
                }
                i + 2
            }
            b't' => {
                if let Some(n) = get(2) {
                    self.codepage = codepages::table(n);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'R' => {
                if let Some(_n) = get(2) {
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'r' => {
                if let Some(_n) = get(2) {
                    self.summary
                        .push("ESC r (selección de color/cinta) ignorado".to_string());
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'i' => {
                self.flush_line();
                self.push(Item::FeedDots(6));
                self.push_cut(false);
                i + 2
            }
            b'm' => {
                self.flush_line();
                self.push_cut(true);
                i + 2
            }
            b'V' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    self.push_cut(n != 0 && n != 48);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'p' => {
                if let Some(m) = get(2) {
                    let t1 = get(3).unwrap_or(50);
                    let _t2 = get(4).unwrap_or(50);
                    self.flush_line();
                    let t = if m & 0x01 != 0 { _t2 } else { t1 };
                    self.push(Item::Drawer { t });
                    i + 5
                } else {
                    usize::MAX
                }
            }
            b'*' => {
                if let (Some(m), Some(nl), Some(nh)) = (get(2), get(3), get(4)) {
                    let width = nl as usize + nh as usize * 256;
                    let bytes_per_col = if matches!(m, 20 | 21 | 33) { 3 } else { 1 };
                    let spread = if matches!(m, 1 | 21 | 32 | 33) { 2 } else { 1 };
                    let data_len = width * bytes_per_col;
                    if i + 5 + data_len <= data.len() {
                        let start = i + 5;
                        let slice = &data[start..start + data_len];
                        self.flush_line();
                        self.push(Item::BitImage(BitImageItem {
                            data: slice.to_vec(),
                            width,
                            bytes_per_col,
                            spread,
                            align: self.align,
                        }));
                        i + 5 + data_len
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                }
            }
            b'%' => {
                if let Some(n) = get(2) {
                    self.style.font = if n == 1 { FontSel::B } else { FontSel::A };
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'D' => {
                self.tabs.clear();
                let mut j = i + 2;
                while j < data.len() && data[j] != 0 {
                    self.tabs.push(data[j]);
                    j += 1;
                }
                if j < data.len() {
                    j + 1
                } else {
                    usize::MAX
                }
            }
            b'l' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    self.left_margin = (a as i32) + (b as i32) * 256;
                    self.flush_line();
                    self.push(Item::SetLeftMargin(self.left_margin as f32));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'L' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    self.left_margin = (a as i32) + (b as i32) * 256;
                    self.flush_line();
                    self.push(Item::SetLeftMargin(self.left_margin as f32));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'W' => {
                if let (Some(_x), Some(_xh), Some(_y), Some(_yh), Some(dx), Some(dxh), Some(_dy), Some(_dyh)) =
                    (get(2), get(3), get(4), get(5), get(6), get(7), get(8), get(9))
                {
                    let left = (get(2).unwrap() as f32) + (get(3).unwrap() as f32) * 256.0;
                    let width = (dx as f32) + (dxh as f32) * 256.0;
                    self.flush_line();
                    self.push(Item::SetPrintArea(PrintArea { left, width }));
                    i + 10
                } else {
                    usize::MAX
                }
            }
            b'Q' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    self.right_margin = Some((a as i32) + (b as i32) * 256);
                    self.flush_line();
                    let width = ((self.right_margin.unwrap() as f32) - (self.left_margin as f32))
                        .max(0.0);
                    self.push(Item::SetPrintArea(PrintArea {
                        left: self.left_margin as f32,
                        width,
                    }));
                    self.summary
                        .push(format!("ESC Q: margen derecho {} (área {width:.0} px)", self.right_margin.unwrap()));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'c' => {
                if let Some(n) = get(2) {
                    if n == 5 {
                        // ESC c 5 n: activa/desactiva los botones del panel
                        let valor = get(3).unwrap_or(0);
                        self.panel_enabled = valor != 0;
                        self.summary.push(format!(
                            "ESC c 5: botones de panel {}",
                            if valor != 0 { "activados" } else { "desactivados" }
                        ));
                        i + 4
                    } else {
                        match n {
                            3 => self.summary.push(
                                "ESC c 3: sensor de papel para detener la impresión".to_string(),
                            ),
                            4 => self.summary.push(
                                "ESC c 4: sensor de papel que emite señal de fin de papel".to_string(),
                            ),
                            6 => self.summary.push(
                                "ESC c 6: sensor de papel para detener impresión (2)".to_string(),
                            ),
                            _ => self.summary.push(format!("Selector ESC c n={n}")),
                        }
                        i + 3
                    }
                } else {
                    usize::MAX
                }
            }
            b'&' => {
                if let (Some(n), Some(c1), Some(c2)) = (get(2), get(3), get(4)) {
                    let count = (c2 as usize).saturating_sub(c1 as usize).saturating_add(1) * n as usize;
                    let data_len = count * 12;
                    let total = i + 5 + data_len;
                    if total <= data.len() {
                        let ds = i + 5;
                        for c in 0..count {
                            let code = c1.wrapping_add(c as u8);
                            let start = ds + c * 12;
                            self.user_font.push((code, data[start..start + 12].to_vec()));
                        }
                        self.summary.push(format!(
                            "Fuente descargada ESC & ({} chars x 12 bytes) almacenada",
                            count
                        ));
                        total
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                }
            }
            b'u' => {
                // ESC u n: estado de periféricos (cajón). Lo responde el servidor
                // en tiempo real porque depende del estado físico actual.
                if let Some(n) = get(2) {
                    self.summary
                        .push(format!("ESC u {n}: estado de periférico transmitido"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'(' => {
                if let (Some(m), Some(nl), Some(nh)) = (get(2), get(3), get(4)) {
                    let len = nl as u32 + nh as u32 * 256;
                    if m == b'C' && get(5) == Some(0x06) {
                        // ESC ( C pL pH 06 n m t: zumbador. n = nº de timbres.
                        let n = get(6).unwrap_or(1);
                        let cyc = get(7).unwrap_or(0);
                        let t = get(8).unwrap_or(0);
                        self.buzz = self.buzz.max(n as u32);
                        self.summary.push(format!(
                            "Buzzer ESC ( C fn=0x06: {n} timbre(s), ciclo {cyc}, t={t} ({len} bytes)"
                        ));
                    } else {
                        self.summary.push(format!(
                            "Comando multi-byte ESC ( {m} ({len} bytes) ignorado"
                        ));
                    }
                    self.n_ignore = len;
                    i + 5
                } else {
                    usize::MAX
                }
            }
            b'K' | b'Y' | b'Z' | b'X' | b'U' | b'T' => {
                if let (Some(nl), Some(nh)) = (get(2), get(3)) {
                    let len = nl as u32 + nh as u32 * 256;
                    self.summary.push(format!(
                        "Gráfico ESC {cmd} ({len} bytes) ignorado"
                    ));
                    self.n_ignore = len;
                    i + 4
                } else {
                    usize::MAX
                }
            }
            _ => {
                self.summary.push(format!("Comando ESC {:#04x} no soportado", cmd));
                i + 2
            }
        }
    }

    fn handle_gs(&mut self, data: &[u8], i: usize) -> usize {
        let get = |off: usize| data.get(i + off).copied();
        let cmd = match get(1) {
            Some(c) => c,
            None => return usize::MAX,
        };
        match cmd {
            b'!' => {
                if let Some(n) = get(2) {
                    let sx = (n & 0x0F).max(1);
                    let sy = ((n >> 4) & 0x0F).max(1);
                    self.style.size_x = sx;
                    self.style.size_y = sy;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'_' => {
                if let Some(n) = get(2) {
                    self.style.underline = n & 0x01 != 0 || n & 0x02 != 0;
                    self.style.underline2 = n & 0x02 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'B' => {
                if let Some(n) = get(2) {
                    self.style.reverse = n & 0x01 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'b' => {
                if let Some(n) = get(2) {
                    self.style.reverse = n == 1;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b' ' => {
                if let Some(n) = get(2) {
                    self.style.char_spacing = n;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'$' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    let x = (a as f32) + (b as f32) * 256.0;
                    self.flush_line();
                    self.push(Item::SetPos(x));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'\\' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    let mut dx = a as i32 + (b as i32) * 256;
                    if dx >= 2048 {
                        dx -= 4096;
                    }
                    self.flush_line();
                    self.push(Item::MoveX(dx as f32));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'V' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    let partial = n != 0 && n != 48;
                    self.push_cut(partial);
                    if n == 66 || n == 67 {
                        if let Some(feed) = get(3) {
                            self.push(Item::FeedLines(feed as u32));
                            i + 4
                        } else {
                            usize::MAX
                        }
                    } else {
                        i + 3
                    }
                } else {
                    usize::MAX
                }
            }
            b'/' => {
                if let Some(n) = get(2) {
                    let (sx, sy) = logo_scale(n);
                    self.push_print_logo(sx, sy);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'a' => {
                if let Some(n) = get(2) {
                    if n == 1 {
                        self.summary.push("ASB activado (estado automático)".to_string());
                    } else {
                        self.summary.push("ASB desactivado".to_string());
                    }
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'v' => {
                if get(2) == Some(0x30) {
                    if let (Some(m), Some(xl), Some(xh), Some(yl), Some(yh)) =
                        (get(3), get(4), get(5), get(6), get(7))
                    {
                        let _m = m;
                        let width = xl as usize + xh as usize * 256;
                        let height = yl as usize + yh as usize * 256;
                        let row_bytes = (width + 7) / 8;
                        let data_len = row_bytes * height;
                        if i + 8 + data_len <= data.len() {
                            let start = i + 8;
                            self.flush_line();
                            self.push(Item::Raster(RasterItem {
                                data: data[start..start + data_len].to_vec(),
                                width,
                                height,
                                align: self.align,
                            }));
                            i + 8 + data_len
                        } else {
                            usize::MAX
                        }
                    } else {
                        usize::MAX
                    }
                } else {
                    i + 2
                }
            }
            b'k' => {
                if let Some(m) = get(2) {
                    let norm_m = match m {
                        0 => 65,
                        1 => 66,
                        2 => 67,
                        3 => 68,
                        4 => 69,
                        5 => 70,
                        6 => 71,
                        other => other,
                    };
                    if (65..=73).contains(&norm_m) {
                        let mut j = i + 3;
                        while j < data.len() && data[j] != 0 {
                            j += 1;
                        }
                        if j < data.len() {
                            let bytes = data[i + 3..j].to_vec();
                            let (norm, warn) = crate::barcode::validate(norm_m, &bytes);
                            if let Some(w) = warn {
                                self.summary.push(w);
                            }
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m: norm_m,
                                data: norm,
                                hri: self.hri,
                                hri_font: self.hri_font,
                                module: self.bar_module,
                                height: self.bar_height,
                            }));
                            j + 1
                        } else {
                            usize::MAX
                        }
                    } else if let Some(n) = get(3) {
                        let len = n as usize;
                        if i + 4 + len <= data.len() {
                            let bytes = data[i + 4..i + 4 + len].to_vec();
                            let (norm, warn) = crate::barcode::validate(norm_m, &bytes);
                            if let Some(w) = warn {
                                self.summary.push(w);
                            }
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m: norm_m,
                                data: norm,
                                hri: self.hri,
                                hri_font: self.hri_font,
                                module: self.bar_module,
                                height: self.bar_height,
                            }));
                            i + 4 + len
                        } else {
                            usize::MAX
                        }
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                }
            }
            b'H' => {
                if let Some(n) = get(2) {
                    self.hri = n;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'f' => {
                if let Some(n) = get(2) {
                    self.hri_font = n & 0x01;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'w' => {
                if let Some(n) = get(2) {
                    self.bar_module = (n as u32).max(1);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'h' => {
                if let Some(n) = get(2) {
                    self.bar_height = (n as u32).max(8);
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'r' => {
                if let Some(n) = get(2) {
                    self.summary.push(format!("Consulta de estado GS r (n={n})"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'#' => {
                // GS # n: nº de copias del siguiente documento
                if let Some(n) = get(2) {
                    self.copies = n.max(1);
                    self.summary.push(format!("GS #: {n} copia(s)"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b':' => {
                // GS : pL pH m n d1..dk: define la macro
                if let (Some(p_l), Some(p_h)) = (get(2), get(3)) {
                    let len = p_l as usize + p_h as usize * 256;
                    let m = get(4).unwrap_or(0);
                    let total = i + 4 + len;
                    if total > data.len() {
                        usize::MAX
                    } else if m == 1 {
                        self.macro_bytes = Some(data[i + 6..i + 4 + len].to_vec());
                        self.summary.push(format!(
                            "Macro definida (GS :): {} bytes almacenados",
                            len.saturating_sub(2)
                        ));
                        total
                    } else {
                        self.summary.push(format!("GS : modo m={m} ignorado"));
                        total
                    }
                } else {
                    usize::MAX
                }
            }
            b'^' => {
                // GS ^ r t m: ejecuta la macro r veces
                if let (Some(r), Some(t)) = (get(2), get(3)) {
                    let mut n = 0;
                    for _ in 0..r.max(1) {
                        self.run_macro();
                        n += 1;
                    }
                    self.summary.push(format!(
                        "Macro ejecutada (GS ^): {n} vez/veces (espera t={t})"
                    ));
                    i + 5
                } else {
                    usize::MAX
                }
            }
            b'I' => {
                // GS I n: identificación de la impresora (ID del fabricante/modelo)
                let _n = get(2);
                self.transmit.push(self.model.id_string().as_bytes().to_vec());
                self.summary.push(format!("GS I: ID \"{}\" transmitido", self.model.id_string()));
                i + 3
            }
            b'P' => {
                if let (Some(xl), Some(xh), Some(yl), Some(yh)) =
                    (get(2), get(3), get(4), get(5))
                {
                    let w = xl as usize + xh as usize * 256;
                    let h = yl as usize + yh as usize * 256;
                    self.flush_line();
                    self.page_mode = true;
                    self.page_height = h;
                    self.push(Item::SetPrintArea(PrintArea {
                        left: self.left_margin as f32,
                        width: w as f32,
                    }));
                    self.push(Item::PageStart {
                        width: w as f32,
                        height: h as f32,
                        dir: self.page_dir,
                    });
                    self.summary.push(format!(
                        "Modo página GS P ({w}x{h}) activado (dirección {})",
                        self.page_dir
                    ));
                    i + 6
                } else {
                    usize::MAX
                }
            }
            b'T' => {
                if let Some(n) = get(2) {
                    self.page_dir = n & 0x03;
                    self.summary
                        .push(format!("GS T dirección {}", self.page_dir));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'8' => {
                return self.handle_gs_8(data, i);
            }
            b'L' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    self.left_margin = (a as i32) + (b as i32) * 256;
                    self.flush_line();
                    self.push(Item::SetLeftMargin(self.left_margin as f32));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'W' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    let width = (a as f32) + (b as f32) * 256.0;
                    self.flush_line();
                    self.push(Item::SetPrintArea(PrintArea {
                        left: self.left_margin as f32,
                        width,
                    }));
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'p' => {
                if let Some(m) = get(2) {
                    let t1 = get(3).unwrap_or(50);
                    let _t2 = get(4).unwrap_or(50);
                    self.flush_line();
                    let t = if m & 0x01 != 0 { _t2 } else { t1 };
                    self.push(Item::Drawer { t });
                    i + 5
                } else {
                    usize::MAX
                }
            }
            b'(' => {
                return self.handle_gs_paren(data, i);
            }
            _ => {
                self.summary.push(format!("Comando GS {:#04x} no soportado", cmd));
                i + 2
            }
        }
    }

    fn handle_gs_paren(&mut self, data: &[u8], i: usize) -> usize {
        let get = |off: usize| data.get(i + off).copied();
        let (Some(p_l), Some(p_h)) = (get(3), get(4)) else {
            return usize::MAX;
        };
        let len = p_l as usize + p_h as usize * 256;
        let total = i + 5 + len;
        if total > data.len() {
            return usize::MAX;
        }
        let Some(cn) = get(5) else {
            return usize::MAX;
        };
        let Some(fn_) = get(6) else {
            return usize::MAX;
        };
        let n16 = |o: usize| -> usize {
            let a = data.get(i + o).copied().unwrap_or(0) as usize;
            let b = data.get(i + o + 1).copied().unwrap_or(0) as usize;
            a + b * 256
        };
        let n24 = |o: usize| -> usize {
            let a = data.get(i + o).copied().unwrap_or(0) as usize;
            let b = data.get(i + o + 1).copied().unwrap_or(0) as usize;
            let c = data.get(i + o + 2).copied().unwrap_or(0) as usize;
            a + b * 256 + c * 65536
        };
        match cn {
            49 => {
                match fn_ {
                    49 => {}
                    50 => {
                        if let Some(n) = get(7) {
                            self.qr_module = (n as u32).max(1);
                        }
                    }
                    51 => {
                        if let Some(n) = get(7) {
                            self.qr_ecc = n;
                        }
                    }
                    67 | 0xA5 => {
                        let dlen = n24(8);
                        let start = i + 11;
                        if start + dlen <= total {
                            self.qr_data = data[start..start + dlen].to_vec();
                        }
                        if fn_ == 0xA5 {
                            self.push_2d(Barcode2DKind::Qr);
                        }
                    }
                    80 => {
                        self.push_2d(Barcode2DKind::Qr);
                    }
                    81 | 0xA7 => {
                        if !self.qr_data.is_empty() {
                            self.transmit.push(self.qr_data.clone());
                            self.summary.push(format!(
                                "QR transmitido ({len} bytes -> {} de datos)",
                                self.qr_data.len()
                            ));
                        } else {
                            self.summary.push("QR transmitir: sin datos".to_string());
                        }
                    }
                    _ => {
                        self.summary.push(format!("GS ( k QR fn={fn_:#x} ignorado"));
                    }
                }
            }
            48 => {
                match fn_ {
                    48 => {
                        self.pdf_cols = Some((n16(7).clamp(1, 30)) as u8);
                    }
                    49 => {
                        self.pdf_rows = Some((n16(7).clamp(3, 90)) as u8);
                    }
                    50 => {
                        if let Some(n) = get(7) {
                            self.pdf_ecc = Some(n);
                        }
                    }
                    51 => {
                        if let Some(n) = get(7) {
                            self.pdf_module = (n as u32).max(1);
                        }
                    }
                    67 => {
                        let dlen = n24(8);
                        let start = i + 11;
                        if start + dlen <= total {
                            self.pdf_data = data[start..start + dlen].to_vec();
                        }
                    }
                    80 => {
                        self.push_2d(Barcode2DKind::Pdf417);
                    }
                    81 => {
                        if !self.pdf_data.is_empty() {
                            self.transmit.push(self.pdf_data.clone());
                            self.summary.push(format!(
                                "PDF417 transmitido ({len} bytes -> {} de datos)",
                                self.pdf_data.len()
                            ));
                        } else {
                            self.summary.push("PDF417 transmitir: sin datos".to_string());
                        }
                    }
                    _ => {
                        self.summary.push(format!("GS ( k PDF417 fn={fn_:#x} ignorado"));
                    }
                }
            }
            0x4C => {
                // GS ( L : definición/impresión de imágenes NV (logotipos)
                match fn_ {
                    48 => {
                        let a = get(7).unwrap_or(0);
                        let k = get(8).unwrap_or(0);
                        if a == 48 {
                            // a=48: definir k logotipos: por cada uno rL rH xL xH yL yH datos
                            let mut j = i + 9;
                            let mut stored = 0;
                            for _ in 0..k.max(1) {
                                if j + 6 > total {
                                    break;
                                }
                                let r = data[j] as usize + data[j + 1] as usize * 256;
                                let x_bytes = data[j + 2] as usize + data[j + 3] as usize * 256;
                                let height = data[j + 4] as usize + data[j + 5] as usize * 256;
                                let ds = j + 6;
                                if x_bytes > 0 && height > 0 && r >= 4 && ds + x_bytes * height <= total {
                                    self.summary.push(format!(
                                        "Logo NV {stored} almacenado ({x_bytes} bytes x {height})"
                                    ));
                                    self.store_logo(data[ds..ds + x_bytes * height].to_vec(), x_bytes * 8, height);
                                    stored += 1;
                                }
                                j = ds + r;
                                if j > total {
                                    break;
                                }
                            }
                            if stored == 0 {
                                self.summary.push("GS ( L fn=48: datos de logo inválidos".to_string());
                            }
                        } else if a == 49 {
                            self.clear_logos();
                            self.summary.push("Logos NV borrados (GS ( L fn=48 a=49)".to_string());
                        } else {
                            self.summary.push(format!("GS ( L fn=48 a={a} ignorado"));
                        }
                    }
                    49 => {
                        // GS ( L fn=49 n1: nº de gráficos NV en la animación
                        if let Some(n) = get(7) {
                            self.summary
                                .push(format!("Animación: {n} gráficos NV por ciclo"));
                        }
                    }
                    50 => {
                        // GS ( L fn=50 n1: fotogramas por segundo de la animación
                        if let Some(n) = get(7) {
                            self.summary
                                .push(format!("Animación: {n} fotogramas/segundo"));
                        }
                    }
                    51 => {
                        // GS ( L fn=51 xL xH: posición inicial de la animación
                        let x = n16(7);
                        self.summary
                            .push(format!("Animación: posición inicial x={x}"));
                    }
                    52 => {
                        // GS ( L fn=52 a: definir un gráfico NV para animación
                        let a = get(7).unwrap_or(0);
                        if a == 48 {
                            let r = n16(8);
                            let x_bytes = n16(10);
                            let height = n16(12);
                            let ds = i + 14;
                            if x_bytes > 0 && height > 0 && r >= 4 && ds + x_bytes * height <= total {
                                self.store_logo(data[ds..ds + x_bytes * height].to_vec(), x_bytes * 8, height);
                                self.summary.push(format!(
                                    "Gráfico de animación almacenado ({x_bytes} bytes x {height})"
                                ));
                            }
                        } else if a == 49 {
                            self.clear_logos();
                            self.summary.push("Gráficos de animación borrados".to_string());
                        }
                    }
                    53 => {
                        // GS ( L fn=53 k: nº de gráficos que se definirán
                        if let Some(k) = get(7) {
                            self.summary.push(format!("Animación: se definirán {k} gráficos"));
                        }
                    }
                    65 | 66 => {
                        if fn_ == 66 {
                            self.push_print_all_logos();
                        } else {
                            self.push_print_logo_num(1, 1, 1);
                        }
                    }
                    73 | 74 | 75 => {
                        let n = self.logos.len().min(32) as u8;
                        self.transmit.push(vec![n]);
                        self.summary.push(format!(
                            "GS ( L fn={fn_} verificación: {n} logo(s) -> transmitido"
                        ));
                    }
                    _ => {
                        self.summary.push(format!("GS ( L fn={fn_:#x} ignorado"));
                    }
                }
            }
             0x4B => {
                // GS ( K : identificación de la impresora
                match fn_ {
                    65 => {
                        self.transmit.push(self.model.id_string().as_bytes().to_vec());
                        self.summary.push("GS ( K fn=65: modelo transmitido".to_string());
                    }
                    66 => {
                        self.transmit.push(self.model.firmware().as_bytes().to_vec());
                        self.summary.push("GS ( K fn=66: firmware transmitido".to_string());
                    }
                    67 => {
                        self.transmit.push(self.model.serial().as_bytes().to_vec());
                        self.summary.push("GS ( K fn=67: serie transmitida".to_string());
                    }
                    _ => {
                        self.summary.push(format!("GS ( K fn={fn_:#x} ignorado"));
                    }
                }
            }
             0x45 => {
                // GS ( E : control de impresión (densidad, auto-test)
                match fn_ {
                    2 => {
                        // fn=2: transmitir densidad de impresión
                        self.transmit.push(vec![self.density]);
                        self.summary
                            .push(format!("GS ( E fn=2: densidad {} transmitida", self.density));
                    }
                    3 => {
                        // fn=3: fijar densidad de impresión
                        if let Some(n) = get(7) {
                            self.density = n;
                            self.summary.push(format!("Densidad de impresión GS ( E: {n}"));
                        }
                    }
                    5 => {
                        // fn=5: imprimir página de auto-test
                        self.auto_test = true;
                        self.summary.push("GS ( E fn=5: auto-test solicitado".to_string());
                    }
                    _ => {
                        self.summary.push(format!("GS ( E fn={fn_:#x} ignorado"));
                    }
                }
            }
             0x41 => {
                // GS ( A : definir caracteres descargados por el usuario
                if fn_ == 48 {
                    let n2 = get(8).unwrap_or(0);
                    let a = get(9).unwrap_or(0);
                    let ds = i + 10;
                    let data_len = n2 as usize * 12;
                    if ds + data_len <= total {
                        let mut count = 0;
                        for c in 0..n2 as usize {
                            let code = a.wrapping_add(c as u8);
                            let start = ds + c * 12;
                            self.user_font.push((code, data[start..start + 12].to_vec()));
                            count += 1;
                        }
                        self.summary.push(format!(
                            "Fuente descargada GS ( A: {count} chars almacenados"
                        ));
                    } else {
                        self.summary.push("GS ( A: datos de fuente incompletos".to_string());
                    }
                } else {
                    self.summary.push(format!("GS ( A fn={fn_:#x} ignorado"));
                }
            }
            _ => {
                self.summary.push(format!(
                    "Comando multi-byte GS ( {cn} ({len} bytes) ignorado"
                ));
            }
        }
        total
    }

    // Ejecuta la macro definida con GS : (bytes ESC/POS re-parseados en el
    // contexto actual, con guarda de profundidad para evitar recursión).
    fn run_macro(&mut self) {
        if self.macro_depth >= 4 {
            self.summary.push("Macro: recursión máxima alcanzada".to_string());
            return;
        }
        let Some(bytes) = self.macro_bytes.clone() else {
            self.summary.push("Macro (GS ^): no hay macro definida".to_string());
            return;
        };
        self.macro_depth += 1;
        self.run(&bytes);
        self.macro_depth -= 1;
    }

    fn push_2d(&mut self, kind: Barcode2DKind) {
        let (data, module, ecc, cols, rows) = match kind {
            Barcode2DKind::Qr => (self.qr_data.clone(), self.qr_module, self.qr_ecc, None, None),
            Barcode2DKind::Pdf417 => (
                self.pdf_data.clone(),
                self.pdf_module,
                self.pdf_ecc.unwrap_or(8),
                self.pdf_cols,
                self.pdf_rows,
            ),
        };
        if data.is_empty() {
            self.summary.push(format!("{kind:?}: sin datos almacenados"));
            return;
        }
        self.flush_line();
        self.push(Item::Barcode2D(Barcode2DItem {
            kind,
            data,
            module,
            ecc,
            cols,
            rows,
        }));
    }

    fn store_logo(&mut self, data: Vec<u8>, width: usize, height: usize) {
        let l = LogoItem { data, width, height };
        if self.logos.len() < 32 {
            self.logos.push(l.clone());
        } else {
            self.summary.push("Capacidad NV de logos agotada (32 máx)".to_string());
        }
        self.logo = Some(l.clone());
        self.flush_line();
        self.push(Item::StoreLogo(l));
    }

    fn clear_logos(&mut self) {
        self.logo = None;
        self.logos.clear();
    }

    // Imprime el logo almacenado (en memoria o en la memoria NV persistente).
    fn push_print_logo(&mut self, scale_x: usize, scale_y: usize) {
        if let Some(l) = self.logo.clone() {
            self.flush_line();
            self.push(Item::StoreLogo(l.clone()));
            self.push(Item::PrintLogo { scale_x, scale_y });
        } else {
            self.summary
                .push("Impresión de logo: no hay logo NV almacenado".to_string());
        }
    }

    // Imprime el logo número `num` (1-based) de los almacenados en NV.
    fn push_print_logo_num(&mut self, num: usize, scale_x: usize, scale_y: usize) {
        let Some(l) = self.logos.get(num.saturating_sub(1)).cloned() else {
            self.summary.push(format!(
                "Impresión de logo #{num}: no existe ({} almacenados)",
                self.logos.len()
            ));
            return;
        };
        self.logo = Some(l.clone());
        self.flush_line();
        self.push(Item::StoreLogo(l));
        self.push(Item::PrintLogo { scale_x, scale_y });
    }

    // Imprime todos los logos almacenados en secuencia (animación).
    fn push_print_all_logos(&mut self) {
        if self.logos.is_empty() {
            self.summary
                .push("Animación: no hay logos NV almacenados".to_string());
            return;
        }
        self.flush_line();
        for l in self.logos.iter().cloned().collect::<Vec<_>>() {
            self.push(Item::StoreLogo(l.clone()));
            self.push(Item::PrintLogo { scale_x: 1, scale_y: 1 });
        }
    }

    fn handle_fs(&mut self, data: &[u8], i: usize) -> usize {
        let get = |off: usize| data.get(i + off).copied();
        let cmd = match get(1) {
            Some(c) => c,
            None => return usize::MAX,
        };
        match cmd {
            b'q' => {
                // FS q n: define NV bit images
                let Some(n) = get(2) else {
                    return usize::MAX;
                };
                let mut j = i + 3;
                let mut stored = 0;
                let count = n.max(1) as usize;
                for _ in 0..count {
                    if j + 4 > data.len() {
                        break;
                    }
                    let wb = data[j] as usize + data[j + 1] as usize * 256;
                    let h = data[j + 2] as usize + data[j + 3] as usize * 256;
                    let dlen = wb * h;
                    j += 4;
                    if j + dlen > data.len() {
                        break;
                    }
                    if wb > 0 && h > 0 {
                        self.summary.push(format!(
                            "Logo NV {} almacenado via FS q ({wb} bytes x {h})",
                            stored + 1
                        ));
                        self.store_logo(data[j..j + dlen].to_vec(), wb * 8, h);
                        stored += 1;
                    }
                    j += dlen;
                }
                j
            }
            b'p' => {
                // FS p n m: imprime el logo nº m (1-based) con escala según m
                if let (Some(_n), Some(m)) = (get(2), get(3)) {
                    let (sx, sy) = logo_scale(m);
                    if m == 0 {
                        self.push_print_logo(sx, sy);
                    } else {
                        self.push_print_logo_num(m as usize, sx, sy);
                    }
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'&' => {
                // FS & : selecciona modo kanji (Shift-JIS)
                self.kanji = true;
                self.summary.push("Modo kanji activado (FS &)".to_string());
                i + 2
            }
            b'.' => {
                // FS . : cancela modo kanji
                self.kanji = false;
                self.summary.push("Modo kanji cancelado (FS .)".to_string());
                i + 2
            }
            b'2' => {
                // FS 2 C1 C2 d1..d32: define kanji de usuario
                if let (Some(c1), Some(c2)) = (get(2), get(3)) {
                    let start = i + 4;
                    if start + 32 <= data.len() {
                        let code = c1 as u16 + (c2 as u16) * 256;
                        self.user_kanji
                            .push((code, data[start..start + 32].to_vec()));
                        self.summary.push(format!(
                            "Kanji de usuario FS 2 {:#06x} (32 bytes) almacenado",
                            code
                        ));
                        i + 36
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                }
            }
            b'"' => {
                // FS " C1 C2 d1..d32: define kanji de usuario (alternativa a FS 2)
                if let (Some(c1), Some(c2)) = (get(2), get(3)) {
                    let start = i + 4;
                    if start + 32 <= data.len() {
                        let code = c1 as u16 + (c2 as u16) * 256;
                        self.user_kanji
                            .push((code, data[start..start + 32].to_vec()));
                        self.summary.push(format!(
                            "Kanji de usuario FS \" {:#06x} (32 bytes) almacenado",
                            code
                        ));
                        i + 36
                    } else {
                        usize::MAX
                    }
                } else {
                    usize::MAX
                }
            }
            b'!' => {
                // FS ! n: modo impresión kanji (mismo formato que GS !)
                if let Some(n) = get(2) {
                    let sx = (n & 0x0F).max(1);
                    let sy = ((n >> 4) & 0x0F).max(1);
                    self.style.size_x = sx;
                    self.style.size_y = sy;
                    self.summary
                        .push(format!("Modo kanji FS ! n={n:#04x}"));
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'-' => {
                // FS - n: subrayado en modo kanji
                if let Some(n) = get(2) {
                    self.style.underline = n & 0x01 != 0 || n & 0x02 != 0;
                    self.style.underline2 = n & 0x02 != 0;
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'/' => {
                // FS / n: espaciado vertical del texto kanji
                if let Some(_n) = get(2) {
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'S' => {
                // FS S n1 n2: espacio entre caracteres kanji (n1 izq, n2 der)
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'$' => {
                // FS $ nL nH: espacio lateral de caracteres kanji
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'W' => {
                // FS W nL nH: ancho del área de impresión kanji
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
                    i + 4
                } else {
                    usize::MAX
                }
            }
            _ => {
                self.summary.push(format!("Comando FS {:#04x} no soportado", cmd));
                i + 2
            }
        }
    }

    // GS 8 L (0x1D 0x38 0x4C) y GS 8 H (0x1D 0x38 0x48): logotipos raster NV
    fn handle_gs_8(&mut self, data: &[u8], i: usize) -> usize {
        let get = |off: usize| data.get(i + off).copied();
        match get(2) {
            Some(b'L') => {
                if let (Some(n1), Some(n2), Some(xl), Some(xh), Some(yl), Some(yh)) =
                    (get(3), get(4), get(5), get(6), get(7), get(8))
                {
                    let n = n1 as usize + n2 as usize * 256;
                    let width_bytes = xl as usize + xh as usize * 256;
                    let height = yl as usize + yh as usize * 256;
                    let total = i + 9 + n;
                    if total <= data.len() && width_bytes * height <= n {
                        self.summary.push(format!(
                            "Logo raster NV almacenado via GS 8 L ({} bytes x {})",
                            width_bytes, height
                        ));
                        let start = i + 9;
                        self.store_logo(
                            data[start..start + width_bytes * height].to_vec(),
                            width_bytes * 8,
                            height,
                        );
                    }
                    total
                } else {
                    usize::MAX
                }
            }
            Some(b'H') => {
                if let Some(m) = get(3) {
                    if m == 1 {
                        self.summary.push(
                            "GS 8 H: logo en buffer de impresión (se imprime al imprimir)".to_string(),
                        );
                    }
                    self.push_print_logo(1, 1);
                    i + 5
                } else {
                    usize::MAX
                }
            }
            _ => {
                self.summary.push("Comando GS 8 desconocido".to_string());
                i + 2
            }
        }
    }
}

fn utf8_len(b: u8) -> usize {
    if b >= 0xF0 {
        4
    } else if b >= 0xE0 {
        3
    } else if b >= 0xC0 {
        2
    } else {
        1
    }
}

fn logo_scale(m: u8) -> (usize, usize) {
    match m {
        49 => (2, 1),
        50 => (1, 2),
        51 => (2, 2),
        _ => (1, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_esc(d: &mut Vec<u8>, cmd: u8) {
        d.push(0x1B);
        d.push(cmd);
    }

    fn push_gs(d: &mut Vec<u8>, cmd: u8) {
        d.push(0x1D);
        d.push(cmd);
    }

    #[test]
    fn estilo_bits_esc_exclamacion() {
        let mut d = Vec::new();
        push_esc(&mut d, b'!');
        d.push(0x50); // font A + size_y2 (0x10) + reverse (0x40)
        d.extend_from_slice(b"X\n");
        let doc = Parser::parse(&d);
        let Item::Text(t) = &doc.items[0] else {
            panic!("esperaba texto");
        };
        assert!(t.runs[0].style.reverse);
        assert_eq!(t.runs[0].style.size_y, 2);
    }

    #[test]
    fn cursiva_espaciado_y_subrayado() {
        let mut d = Vec::new();
        push_esc(&mut d, b'4');
        push_esc(&mut d, b' ');
        d.push(4);
        d.extend_from_slice(b"A\n");
        push_esc(&mut d, b'5');
        push_esc(&mut d, b'-');
        d.push(2);
        d.extend_from_slice(b"B\n");
        let doc = Parser::parse(&d);
        let texts: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 2);
        assert!(texts[0].runs[0].style.italic);
        assert_eq!(texts[0].runs[0].style.char_spacing, 4);
        assert!(!texts[1].runs[0].style.italic);
        assert!(texts[1].runs[0].style.underline && texts[1].runs[0].style.underline2);
    }

    #[test]
    fn posicionamiento_y_margenes() {
        let mut d = Vec::new();
        push_esc(&mut d, b'$');
        d.extend_from_slice(&40u16.to_le_bytes());
        push_esc(&mut d, b'l');
        d.extend_from_slice(&(20u16).to_le_bytes());
        push_gs(&mut d, b'L');
        d.extend_from_slice(&(30u16).to_le_bytes());
        push_gs(&mut d, b'W');
        d.extend_from_slice(&(300u16).to_le_bytes());
        d.push(0x0A);
        let doc = Parser::parse(&d);
        let has_pos = doc.items.iter().any(|it| matches!(it, Item::SetPos(x) if *x == 40.0));
        assert!(has_pos, "esperaba Item::SetPos(40)");
        let has_margin = doc.items.iter().any(|it| matches!(it, Item::SetLeftMargin(x) if *x == 30.0));
        assert!(has_margin, "esperaba Item::SetLeftMargin(30)");
        let has_area = doc
            .items
            .iter()
            .any(|it| matches!(it, Item::SetPrintArea(p) if p.left == 30.0 && p.width == 300.0));
        assert!(has_area, "esperaba Item::SetPrintArea");
    }

    #[test]
    fn corte_gs_v() {
        let mut d = Vec::new();
        push_gs(&mut d, b'V');
        d.push(1);
        let doc = Parser::parse(&d);
        let has_cut = doc.items.iter().any(|it| matches!(it, Item::Cut { partial: true }));
        assert!(has_cut);
    }

    #[test]
    fn qr_store_y_print() {
        let mut d = Vec::new();
        let qr = b"test-qr";
        let pl = 6 + qr.len();
        d.extend_from_slice(&[0x1D, 0x28, 0x6B]);
        d.push(pl as u8);
        d.push(0);
        d.extend_from_slice(&[0x31, 0x43, 0x30]);
        d.extend_from_slice(&(qr.len() as u16).to_le_bytes());
        d.push(0);
        d.extend_from_slice(qr);
        d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x31, 0x50]);
        let doc = Parser::parse(&d);
        let qr_items: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Barcode2D(b) if b.kind == Barcode2DKind::Qr => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(qr_items.len(), 1);
        assert_eq!(qr_items[0].data, qr);
    }

    #[test]
    fn pdf417_store_y_print() {
        let mut d = Vec::new();
        let pdf = b"test-pdf417";
        let pl = 6 + pdf.len();
        d.extend_from_slice(&[0x1D, 0x28, 0x6B]);
        d.push(pl as u8);
        d.push(0);
        d.extend_from_slice(&[0x30, 0x43, 0x30]);
        d.extend_from_slice(&(pdf.len() as u16).to_le_bytes());
        d.push(0);
        d.extend_from_slice(pdf);
        d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x30, 0x50]);
        let doc = Parser::parse(&d);
        let pdf_items: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Barcode2D(b) if b.kind == Barcode2DKind::Pdf417 => Some(b),
                _ => None,
            })
            .collect();
        assert_eq!(pdf_items.len(), 1);
        assert_eq!(pdf_items[0].data, pdf);
    }

    #[test]
    fn utf8_multibyte() {
        let mut d = Vec::new();
        d.extend_from_slice("Café € 40".as_bytes());
        d.push(0x0A);
        let doc = Parser::parse(&d);
        let Item::Text(t) = &doc.items[0] else {
            panic!("esperaba texto");
        };
        assert_eq!(t.runs[0].text, "Café € 40");
    }

    #[test]
    fn utf8_invalido_cae_a_codepage() {
        // 0xC9 0xC4 en CP437 = '╔' '─'; 0xC4 no es byte de continuación UTF-8
        let doc = Parser::parse(&[0xC9, 0xC4, 0x0A]);
        let texts: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].runs[0].text, "╔─");
    }

    #[test]
    fn gs_a_asb() {
        let mut d = Vec::new();
        push_gs(&mut d, b'a');
        d.push(1);
        d.push(0x0A);
        let doc = Parser::parse(&d);
        assert!(doc.summary.iter().any(|s| s.contains("ASB")));
    }

    #[test]
    fn gs_8_l_almacena_raster_y_gs_8_h_imprime() {
        let mut d = Vec::new();
        const RN: usize = 2 * 4; // 2 bytes/row x 4 rows
        d.extend_from_slice(&[0x1D, 0x38, 0x4C]);
        d.push(RN as u8);
        d.push(0);
        d.extend_from_slice(&[2u8, 0, 4, 0]); // width_bytes=2 (16 px), height=4
        d.extend_from_slice(&[0u8; RN]);
        d.extend_from_slice(&[0x1D, 0x38, 0x48, 0x00, 0x00]);
        let doc = Parser::parse(&d);
        assert!(
            doc.items
                .iter()
                .any(|it| matches!(it, Item::StoreLogo(l) if l.width == 16 && l.height == 4)),
            "esperaba Item::StoreLogo"
        );
        assert!(
            doc.items
                .iter()
                .any(|it| matches!(it, Item::PrintLogo { scale_x: 1, scale_y: 1 })),
            "esperaba Item::PrintLogo"
        );
    }

    #[test]
    fn gs_slash_escalas_de_logo() {
        let mut d = Vec::new();
        // Primero se define un logo NV (GS 8 L) y luego se imprime escalado con GS /
        const RN: usize = 2 * 4;
        d.extend_from_slice(&[0x1D, 0x38, 0x4C]);
        d.push(RN as u8);
        d.push(0);
        d.extend_from_slice(&[2u8, 0, 4, 0]);
        d.extend_from_slice(&[0u8; RN]);
        push_gs(&mut d, b'/');
        d.push(49);
        push_gs(&mut d, b'/');
        d.push(51);
        let doc = Parser::parse(&d);
        let scales: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::PrintLogo { scale_x, scale_y } => Some((*scale_x, *scale_y)),
                _ => None,
            })
            .collect();
        assert_eq!(scales, vec![(2, 1), (2, 2)]);
    }

    #[test]
    fn modo_pagina_gs_p_y_esc_ff() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x50]);
        d.extend_from_slice(&(576u16).to_le_bytes());
        d.extend_from_slice(&(300u16).to_le_bytes());
        d.extend_from_slice(b"PAGINA\n");
        d.extend_from_slice(&[0x1B, 0x0C]);
        let doc = Parser::parse(&d);
        assert!(
            doc.summary.iter().any(|s| s.contains("Modo página GS P")),
            "esperaba entrada de modo página"
        );
        assert!(
            doc.summary.iter().any(|s| s.contains("Modo página impreso")),
            "esperaba salida de modo página"
        );
        let has_area = doc
            .items
            .iter()
            .any(|it| matches!(it, Item::SetPrintArea(p) if p.width == 576.0));
        assert!(has_area, "esperaba Item::SetPrintArea del modo página");
    }

    #[test]
    fn direccion_pagina_gs_t() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x54, 0x02]); // GS T dir 2
        d.extend_from_slice(&[0x1D, 0x50]);
        d.extend_from_slice(&(576u16).to_le_bytes());
        d.extend_from_slice(&(300u16).to_le_bytes());
        d.extend_from_slice(b"X\n");
        d.extend_from_slice(&[0x1B, 0x0C]);
        let doc = Parser::parse(&d);
        assert!(
            doc.items
                .iter()
                .any(|it| matches!(it, Item::PageStart { dir: 2, .. })),
            "esperaba Item::PageStart con dirección 2"
        );
        assert!(
            doc.items
                .iter()
                .any(|it| matches!(it, Item::PageEnd { .. })),
            "esperaba Item::PageEnd"
        );
    }

    #[test]
    fn transmitir_qr_y_pdf417_fn_81() {
        let mut d = Vec::new();
        let qr = b"tx-data";
        let pl = 6 + qr.len();
        d.extend_from_slice(&[0x1D, 0x28, 0x6B]);
        d.push(pl as u8);
        d.push(0);
        d.extend_from_slice(&[0x31, 0x43, 0x30]);
        d.extend_from_slice(&(qr.len() as u16).to_le_bytes());
        d.push(0);
        d.extend_from_slice(qr);
        d.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x31, 0x51]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.transmit, vec![qr.to_vec()]);
    }

    #[test]
    fn buzzer_y_fuente_descargada() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1B, 0x28, 0x43, 0x04, 0x00, 0x06, 0x03, 0x01, 0x00]);
        d.extend_from_slice(&[0x1B, 0x26, 0x01, 0x41, 0x42]);
        d.extend_from_slice(&[0u8; 24]);
        let doc = Parser::parse(&d);
        assert!(doc.summary.iter().any(|s| s.contains("Buzzer")));
        assert!(doc.summary.iter().any(|s| s.contains("Fuente descargada")));
    }

    #[test]
    fn kanji_fs_amp_y_fs_punto() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1C, 0x26]); // FS & -> modo kanji
        d.extend_from_slice(&[0x88, 0x9F]); // "亜" en Shift-JIS
        d.push(0x0A);
        d.extend_from_slice(&[0x1C, 0x2E]); // FS . -> cancela kanji
        d.extend_from_slice(&[0x88, 0x9F, 0x0A]); // ahora es texto CP437
        let doc = Parser::parse(&d);
        let texts: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t.runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>()),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0].concat(), "亜");
        assert_eq!(texts[1].concat(), "êƒ"); // 0x88/0x9F CP437 sin modo kanji
        assert!(doc.summary.iter().any(|s| s.contains("Modo kanji activado")));
        assert!(doc.summary.iter().any(|s| s.contains("Modo kanji cancelado")));
    }

    #[test]
    fn codepage_multibyte_esc_t_81() {
        let mut d = Vec::new();
        push_esc(&mut d, b't');
        d.push(81); // Shift-JIS
        d.extend_from_slice(&[0x82, 0xA0]); // "あ" en Shift-JIS
        d.extend_from_slice(&[0x88, 0x9F]); // "亜"
        d.push(0x0A);
        let doc = Parser::parse(&d);
        let texts: Vec<_> = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t.runs.iter().map(|r| r.text.clone()).collect::<Vec<_>>()),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0].concat(), "あ亜");
    }

    #[test]
    fn codepage_1251_cirilico() {
        let mut d = Vec::new();
        push_esc(&mut d, b't');
        d.push(85); // WPC1251
        d.extend_from_slice(&[0xC0, 0xE0, 0x0A]); // "Аа"
        let doc = Parser::parse(&d);
        let Item::Text(t) = &doc.items[0] else {
            panic!("esperaba texto");
        };
        assert_eq!(t.runs[0].text, "Аа");
    }

    #[test]
    fn fs_2_define_kanji_usuario() {
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1C, 0x32]); // FS 2
        d.extend_from_slice(&[0x88, 0x9F]);
        d.extend_from_slice(&[0u8; 32]);
        let doc = Parser::parse(&d);
        assert!(doc.summary.iter().any(|s| s.contains("Kanji de usuario")));
        assert_eq!(doc.memory.user_kanji, vec![(0x9F88, vec![0u8; 32])]);
    }

    #[test]
    fn multi_logo_gs_l_y_fs_p_numero() {
        let mut d = Vec::new();
        // Definir k=2 logos vía GS ( L fn=48 a=48; r = nº de bytes de datos
        let mut seq = Vec::new();
        for (w, h, fill) in [(2u16, 2u16, 0xA5u8), (2, 4, 0x5A)] {
            let r = w * h;
            seq.extend_from_slice(&r.to_le_bytes());
            seq.extend_from_slice(&w.to_le_bytes());
            seq.extend_from_slice(&h.to_le_bytes());
            seq.extend_from_slice(&vec![fill; r as usize]);
        }
        d.extend_from_slice(&[0x1D, 0x28, 0x4C]);
        d.push(0); // pL (se rellena abajo)
        d.push(0); // pH
        d.extend_from_slice(&[0x4C, 0x30, 0x30, 2]); // cn=L fn=48 a=48 k=2
        d.extend_from_slice(&seq);
        let p_l = (seq.len() + 4) as u8; // cn fn a k
        d[3] = p_l;
        // FS p n=1 m=2: imprime el logo 2
        d.extend_from_slice(&[0x1C, 0x70, 0x01, 0x02]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.memory.logos.len(), 2);
        assert_eq!(doc.memory.logos[1].height, 4);
        let prints = doc
            .items
            .iter()
            .filter(|it| matches!(it, Item::PrintLogo { .. }))
            .count();
        assert_eq!(prints, 1, "esperaba una sola impresión de logo");
        let last_store = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::StoreLogo(l) => Some(l.height),
                _ => None,
            })
            .last()
            .unwrap();
        assert_eq!(last_store, 4, "el último StoreLogo debe ser el logo 2");
    }

    #[test]
    fn gs_l_fn73_transmite_conteo() {
        let mut d = Vec::new();
        let mut seq = Vec::new();
        seq.extend_from_slice(&4u16.to_le_bytes()); // r = datos
        seq.extend_from_slice(&2u16.to_le_bytes());
        seq.extend_from_slice(&2u16.to_le_bytes());
        seq.extend_from_slice(&[0u8; 4]);
        d.extend_from_slice(&[0x1D, 0x28, 0x4C]);
        d.push(0);
        d.push(0);
        d.extend_from_slice(&[0x4C, 0x30, 0x30, 1]);
        d.extend_from_slice(&seq);
        let p_l = (seq.len() + 4) as u8;
        d[3] = p_l;
        // GS ( L fn=73: transmite el número de logos
        d.extend_from_slice(&[0x1D, 0x28, 0x4C, 0x01, 0x00, 0x4C, 0x49]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.memory.logos.len(), 1);
        assert_eq!(doc.transmit, vec![vec![1u8]]);
    }

    #[test]
    fn identificacion_gs_i_y_gs_k_por_modelo() {
        use crate::model::PrinterModel;
        // GS I n: devuelve el ID del modelo
        let doc = Parser::parse_with_model(
            &[0x1D, 0x49, 0x01],
            &PrinterMemory::default(),
            PrinterModel::XprinterXp80,
        );
        assert_eq!(doc.transmit, vec![b"XP-T80".to_vec()]);
        // GS ( K fn=65/66/67: modelo, firmware y serie
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x28, 0x4B, 0x02, 0x00, 0x4B, 0x41]); // fn=65 modelo
        d.extend_from_slice(&[0x1D, 0x28, 0x4B, 0x02, 0x00, 0x4B, 0x42]); // fn=66 firmware
        d.extend_from_slice(&[0x1D, 0x28, 0x4B, 0x02, 0x00, 0x4B, 0x43]); // fn=67 serie
        let doc = Parser::parse_with_model(&d, &PrinterMemory::default(), PrinterModel::CitizenCtS310);
        assert_eq!(
            doc.transmit,
            vec![b"CT-S310".to_vec(), b"1.05".to_vec(), b"CTS31012345".to_vec()]
        );
    }

    #[test]
    fn hri_posicion_y_fuente_gs_h_f() {
        // GS H n (posición) + GS f n (fuente) + GS k m datos
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x48, 0x03]); // HRI encima y debajo
        d.extend_from_slice(&[0x1D, 0x66, 0x01]); // HRI fuente B
        d.extend_from_slice(&[0x1D, 0x6B, 0x02, b'1', b'2', 0x00]); // Code39 "12"
        let doc = Parser::parse(&d);
        let b = doc
            .items
            .iter()
            .find_map(|it| match it {
                Item::Barcode(b) => Some(b),
                _ => None,
            })
            .expect("esperaba un código de barras");
        assert_eq!(b.hri, 3);
        assert_eq!(b.hri_font, 1);
        assert_eq!(b.data, b"12");
    }

    #[test]
    fn copias_gs_num() {
        // GS # 3: el siguiente documento se imprime 3 veces
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x23, 0x03]);
        d.extend_from_slice(b"AB");
        d.push(b'\n');
        let doc = Parser::parse(&d);
        assert_eq!(doc.copies, 3);
        let texts = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t.runs.iter().map(|r| r.text.as_str()).collect::<String>()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["AB".to_string(); 3]);
    }

    #[test]
    fn macro_definir_y_ejecutar() {
        // GS : define una macro; GS ^ la ejecuta en el contexto actual
        let mut d = Vec::new();
        // Definir macro = [0x1D 0x40 "MACRO" 0x0A]  (pL pH m n + datos)
        let content: Vec<u8> = [0x1D, 0x40].into_iter().chain(b"MACRO\n".iter().copied()).collect();
        let len = (content.len() + 2) as u16; // m(1) + n(1) + contenido
        d.extend_from_slice(&[0x1D, 0x3A]);
        d.extend_from_slice(&len.to_le_bytes());
        d.extend_from_slice(&[0x01, 0x00]);
        d.extend_from_slice(&content);
        // Ejecutar: GS ^ r t m
        d.extend_from_slice(&[0x1D, 0x5E, 0x01, 0x00, 0x01]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.memory.macro_bytes, Some(content));
        let texts = doc
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Text(t) => Some(t.runs.iter().map(|r| r.text.as_str()).collect::<String>()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["MACRO".to_string()], "la macro debe imprimirse al ejecutarla");
    }

    #[test]
    fn esc_8_y_esc_c5_panel() {
        let d = vec![0x1B, b'8', 0x00];
        let doc = Parser::parse(&d);
        assert!(!doc.panel_enabled);
        let d = vec![0x1B, b'c', 0x05, 0x00];
        let doc = Parser::parse(&d);
        assert!(!doc.panel_enabled);
        let d = vec![0x1B, b'8', 0x01];
        let doc = Parser::parse(&d);
        assert!(doc.panel_enabled);
    }

    #[test]
    fn esc_q_margen_derecho() {
        // ESC l margen izq 10 + ESC Q margen der 100 -> área de 90 px
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1B, b'l', 10, 0]);
        d.extend_from_slice(&[0x1B, b'Q', 100, 0]);
        let doc = Parser::parse(&d);
        let area = doc
            .items
            .iter()
            .find_map(|it| match it {
                Item::SetPrintArea(a) => Some(a.clone()),
                _ => None,
            })
            .expect("ESC Q debe fijar el área de impresión");
        assert_eq!(area.left as i32, 10);
        assert!((area.width - 90.0).abs() < 0.001);
    }

    #[test]
    fn gs_e_densidad_transmite_y_autotest() {
        // fn=2 transmite densidad, fn=3 fija, fn=5 pide auto-test
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1D, 0x28, 0x45, 0x02, 0x00, 0x45, 0x02]);
        d.extend_from_slice(&[0x1D, 0x28, 0x45, 0x03, 0x00, 0x45, 0x03, 0x2A]);
        d.extend_from_slice(&[0x1D, 0x28, 0x45, 0x02, 0x00, 0x45, 0x02]);
        d.extend_from_slice(&[0x1D, 0x28, 0x45, 0x02, 0x00, 0x45, 0x05]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.memory.density, 42);
        assert!(doc.auto_test);
        // fn=2 transmitió la densidad inicial (0) y la nueva (42)
        assert_eq!(doc.transmit, vec![vec![0], vec![42]]);
    }

    #[test]
    fn buzzer_esc_c_fn06_cuenta_timbres() {
        // ESC ( C pL pH 06 n m t
        let mut d = Vec::new();
        d.extend_from_slice(&[0x1B, 0x28, 0x43, 0x04, 0x00, 0x06, 0x03, 0x01, 0x00]);
        let doc = Parser::parse(&d);
        assert_eq!(doc.buzz, 3);
    }

    #[test]
    fn drawer_pulse_recogido() {
        let d = vec![0x1B, b'p', 0x00, 0x32, 0x32]; // ESC p 0 n1=50 n2=50
        let doc = Parser::parse(&d);
        assert_eq!(doc.drawer_pulse_ms, Some(100), "n1=50 -> pulso de 100 ms");
    }

    #[test]
    fn esc_u_registra_consulta_periferico() {
        let d = vec![0x1B, b'u', 0x01];
        let doc = Parser::parse(&d);
        assert!(doc
            .summary
            .iter()
            .any(|l| l.contains("ESC u") && l.contains("periférico")));
    }

    #[test]
    fn dip_hri_abajo_por_defecto_y_corte_sin_cuchilla() {
        // HRI abajo por defecto: los códigos de barras usan posición 2 sin GS H.
        let d = vec![0x1D, b'k', 0x02, b'1', b'2', b'3', b'4', b'5', 0x00];
        let doc = Parser::parse_with_dip(&d, &PrinterMemory::default(), crate::model::PrinterModel::EpsonTmT88V, 0, true, true);
        let bc = doc
            .items
            .iter()
            .find_map(|it| match it {
                Item::Barcode(b) => Some(b),
                _ => None,
            })
            .expect("debe emitirse un código de barras");
        assert_eq!(bc.hri, 2, "con DIP HRI abajo, la posición por defecto es debajo");

        // Sin cuchilla (auto_cut off): GS V no produce Item::Cut.
        let d = vec![0x1D, b'V', 65];
        let doc = Parser::parse_with_dip(&d, &PrinterMemory::default(), crate::model::PrinterModel::EpsonTmT88V, 0, true, false);
        assert!(
            !doc.items.iter().any(|it| matches!(it, Item::Cut { .. })),
            "sin cuchilla no debe dibujarse la línea de corte"
        );
        assert!(doc.summary.iter().any(|l| l.contains("cuchilla automática")));

        // Con cuchilla activa sí se corta.
        let d = vec![0x1D, b'V', 65];
        let doc = Parser::parse_with_dip(&d, &PrinterMemory::default(), crate::model::PrinterModel::EpsonTmT88V, 0, true, true);
        assert!(doc.items.iter().any(|it| matches!(it, Item::Cut { .. })));
    }
}
