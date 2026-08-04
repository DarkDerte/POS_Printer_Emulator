use crate::codepages::{self, Codepage};

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
    SetLineSpacing(Option<u32>),
    Init,
}

#[derive(Clone, Debug)]
pub struct ParsedDoc {
    pub items: Vec<Item>,
    pub summary: Vec<String>,
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
    n_ignore: u32,
    fed_blank: bool,
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
            n_ignore: 0,
            fed_blank: false,
        }
    }
}

impl Parser {
    pub fn parse(data: &[u8]) -> ParsedDoc {
        let mut p = Parser::default();
        p.run(data);
        p.flush_line();
        ParsedDoc {
            items: p.items,
            summary: p.summary,
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
        match self.line.last_mut() {
            Some(last) if last.style == self.style => last.text.push(ch),
            _ => self.line.push(TextRun {
                text: ch.to_string(),
                style: self.style,
            }),
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
                    self.style.underline = n & 0x80 != 0;
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
                    self.style.underline = n & 0x01 != 0;
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
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'Q' => {
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'c' => {
                if let Some(_n) = get(2) {
                    i + 3
                } else {
                    usize::MAX
                }
            }
            b'(' => {
                if let (Some(m), Some(nl), Some(nh)) = (get(2), get(3), get(4)) {
                    let len = nl as u32 + nh as u32 * 256;
                    self.summary.push(format!(
                        "Comando multi-byte ESC ( {m} ({len} bytes) ignorado"
                    ));
                    self.n_ignore = len;
                    i + 5
                } else {
                    usize::MAX
                }
            }
            b'K' | b'L' | b'Y' | b'Z' | b'X' | b'W' | b'U' | b'T' => {
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
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m,
                                data: bytes,
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
                            self.flush_line();
                            self.push(Item::Barcode(BarcodeItem {
                                m,
                                data: bytes,
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
                self.summary.push("Modo página GS P activado (ignorado)".to_string());
                i + 2
            }
            b'L' => {
                if let (Some(a), Some(b)) = (get(2), get(3)) {
                    self.left_margin = (a as i32) + (b as i32) * 256;
                    i + 4
                } else {
                    usize::MAX
                }
            }
            b'W' => {
                if let (Some(_a), Some(_b)) = (get(2), get(3)) {
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
                if let (Some(m), Some(nl), Some(nh)) = (get(2), get(3), get(4)) {
                    let len = nl as u32 + nh as u32 * 256;
                    self.summary.push(format!(
                        "Comando multi-byte GS ( {m} ({len} bytes) ignorado"
                    ));
                    self.n_ignore = len;
                    i + 5
                } else {
                    usize::MAX
                }
            }
            _ => {
                self.summary.push(format!("Comando GS {:#04x} no soportado", cmd));
                i + 2
            }
        }
    }
}
