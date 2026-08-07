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

#[derive(Clone, Debug)]
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
}

pub struct Parser {
    align: Alignment,
    style: Style,
    line: Vec<TextRun>,
    line_spacing: Option<u32>,
    codepage: Codepage,
    left_margin: i32,
    tabs: Vec<u8>,
    bar_module: u32,
    bar_height: u32,
    hri: u8,
    items: Vec<Item>,
    summary: Vec<String>,
    transmit: Vec<Vec<u8>>,
    n_ignore: u32,
    fed_blank: bool,
    page_mode: bool,
    page_height: usize,
    page_dir: u8,
    logo: Option<LogoItem>,
    density: u8,
    user_font: Vec<(u8, Vec<u8>)>,
    qr_data: Vec<u8>,
    qr_module: u32,
    qr_ecc: u8,
    pdf_data: Vec<u8>,
    pdf_module: u32,
    pdf_cols: Option<u8>,
    pdf_rows: Option<u8>,
    pdf_ecc: Option<u8>,
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
            bar_module: 2,
            bar_height: 50,
            hri: 0,
            items: Vec::new(),
            summary: Vec::new(),
            transmit: Vec::new(),
            n_ignore: 0,
            fed_blank: false,
            page_mode: false,
            page_height: 0,
            page_dir: 0,
            logo: None,
            density: 0,
            user_font: Vec::new(),
            qr_data: Vec::new(),
            qr_module: 3,
            qr_ecc: 49,
            pdf_data: Vec::new(),
            pdf_module: 2,
            pdf_cols: None,
            pdf_rows: None,
            pdf_ecc: None,
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
        p.user_font = memory.user_font.clone();
        p.density = memory.density;
        p.run(data);
        p.flush_line();
        ParsedDoc {
            items: p.items,
            summary: p.summary,
            transmit: p.transmit,
            memory: PrinterMemory {
                logo: p.logo,
                user_font: p.user_font,
                density: p.density,
            },
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
                0x00 | 0x11 | 0x13 | 0x04 | 0x05 | 0x06 | 0x15 | 0x10 | 0x14 | 0x0B | 0x0F => {
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
        self.bar_module = 2;
        self.bar_height = 50;
        self.hri = 0;
        self.page_mode = false;
        self.page_height = 0;
        self.page_dir = 0;
        self.qr_data.clear();
        self.qr_module = 3;
        self.qr_ecc = 49;
        self.pdf_data.clear();
        self.pdf_module = 2;
        self.pdf_cols = None;
        self.pdf_rows = None;
        self.pdf_ecc = None;
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
                    self.push(Item::Cut {
                        partial: n != 0 && n != 48,
                    });
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
                self.summary.push("ESC 7 (modo página) ignorado".to_string());
                i + 2
            }
            b'8' => {
                self.summary.push("ESC 8 (desactivación de página) ignorado".to_string());
                i + 2
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
                    self.push(Item::FeedDots(self.page_height as u32));
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
                self.push(Item::Cut { partial: false });
                i + 2
            }
            b'm' => {
                self.flush_line();
                self.push(Item::Cut { partial: true });
                i + 2
            }
            b'V' => {
                if let Some(n) = get(2) {
                    self.flush_line();
                    self.push(Item::Cut {
                        partial: n != 0 && n != 48,
                    });
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
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
                    self.summary.push("ESC Q (margen derecho) ignorado".to_string());
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'c' => {
                if let Some(n) = get(2) {
                    self.summary.push(format!("Selector ESC c n={n} ignorado"));
                    i + 3
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
            b'(' => {
                if let (Some(m), Some(nl), Some(nh)) = (get(2), get(3), get(4)) {
                    let len = nl as u32 + nh as u32 * 256;
                    if m == b'C' && get(5) == Some(0x06) {
                        // ESC ( C pL pH 06 n m t: zumbador (buzzer)
                        self.summary.push(format!(
                            "Buzzer activado (ESC ( C fn=0x06, {len} bytes)"
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
                    self.push(Item::Cut { partial });
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
                    if (65..=73).contains(&m) {
                        let mut j = i + 3;
                        while j < data.len() && data[j] != 0 {
                            j += 1;
                        }
                        if j < data.len() {
                            let bytes = data[i + 3..j].to_vec();
                            let (norm, warn) = crate::barcode::validate(m, &bytes);
                            if let Some(w) = warn {
                                self.summary.push(w);
                            }
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m,
                                data: norm,
                                hri: self.hri,
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
                            let (norm, warn) = crate::barcode::validate(m, &bytes);
                            if let Some(w) = warn {
                                self.summary.push(w);
                            }
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m,
                                data: norm,
                                hri: self.hri,
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
                if let Some(_n) = get(2) {
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
                    self.summary.push(format!("Modo página GS P ({w}x{h}) activado"));
                    i + 6
                } else {
                    usize::MAX
                }
            }
            b'T' => {
                if let Some(n) = get(2) {
                    self.page_dir = n;
                    if n != 0 {
                        self.summary
                            .push(format!("GS T dirección {n}: no soportada, se usa 0"));
                    }
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
                            self.logo = None;
                            self.summary.push("Logo NV borrado (GS ( L fn=48 a=49)".to_string());
                        } else {
                            self.summary.push(format!("GS ( L fn=48 a={a} ignorado"));
                        }
                    }
                    65 | 66 => {
                        self.push_print_logo(1, 1);
                    }
                    73 | 74 | 75 => {
                        let n = if self.logo.is_some() { 1 } else { 0 };
                        self.summary.push(format!("GS ( L fn={fn_} verificación: {n} logo(s)"));
                    }
                    _ => {
                        self.summary.push(format!("GS ( L fn={fn_:#x} ignorado"));
                    }
                }
            }
             0x45 => {
                if fn_ == 3 {
                    if let Some(n) = get(7) {
                        self.density = n;
                        self.summary.push(format!("Densidad de impresión GS ( E: {n}"));
                    }
                } else {
                    self.summary.push("GS ( E (densidad de impresión) ignorado".to_string());
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
        self.logo = Some(l.clone());
        self.flush_line();
        self.push(Item::StoreLogo(l));
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
                let mut stored = false;
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
                    if !stored && wb > 0 && h > 0 {
                        self.summary.push(format!(
                            "Logo NV almacenado via FS q ({wb} bytes x {h})"
                        ));
                        self.store_logo(data[j..j + dlen].to_vec(), wb * 8, h);
                        stored = true;
                    }
                    j += dlen;
                }
                j
            }
            b'p' => {
                if let Some(_n) = get(2) {
                    let m = get(3).unwrap_or(0);
                    let (sx, sy) = logo_scale(m);
                    self.push_print_logo(sx, sy);
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'&' | b'"' => {
                self.summary.push(format!("Kanji FS {} ignorado", cmd as char));
                i + 3
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
}
