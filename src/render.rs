use std::path::Path;

use ab_glyph::{point, Font, FontVec, PxScale, ScaleFont};

use crate::parser::{
    Alignment, Barcode2DItem, Barcode2DKind, FontSel, Item, LogoItem, TextItem, TextRun,
};

pub struct Fonts {
    pub regular: Option<FontVec>,
    pub bold: Option<FontVec>,
}

impl Fonts {
    pub fn load() -> Self {
        let candidates: &[(&str, &str)] = &[
            ("consola.ttf", "consolab.ttf"),
            ("lucon.ttf", ""),
            ("cour.ttf", "courbd.ttf"),
            ("cascadiamono.ttf", "cascadiamonob.ttf"),
            ("arial.ttf", "arialbd.ttf"),
            ("segoeui.ttf", "segoeuib.ttf"),
            ("tahoma.ttf", "tahomabd.ttf"),
            ("times.ttf", "timesbd.ttf"),
        ];
        let base = Path::new("C:/Windows/Fonts");
        for (reg, bld) in candidates {
            let rp = base.join(reg);
            let Ok(bytes) = std::fs::read(&rp) else {
                continue;
            };
            let Ok(font) = FontVec::try_from_vec(bytes) else {
                continue;
            };
            let bold = if bld.is_empty() {
                None
            } else {
                std::fs::read(base.join(bld))
                    .ok()
                    .and_then(|b| FontVec::try_from_vec(b).ok())
            };
            return Fonts {
                regular: Some(font),
                bold,
            };
        }
        Fonts {
            regular: None,
            bold: None,
        }
    }
}

pub struct RenderedPage {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

pub struct Renderer<'a> {
    cv: Canvas,
    fonts: &'a Fonts,
    line_spacing: Option<f32>,
    left_margin: f32,
    print_width: Option<f32>,
    logo: Option<LogoItem>,
}

struct Canvas {
    w: usize,
    buf: Vec<u8>,
    used: usize,
    x: f32,
    y: f32,
    line_height: f32,
    clip_right: i32,
}

impl Canvas {
    fn new(w: usize) -> Self {
        Canvas {
            w,
            buf: Vec::new(),
            used: 0,
            x: 0.0,
            y: 0.0,
            line_height: 24.0,
            clip_right: i32::MAX,
        }
    }

    fn ensure(&mut self, h: usize) {
        let need = (h + 16).saturating_mul(self.w).saturating_mul(4);
        if need > self.buf.len() {
            let grow = (self.buf.len().max(1) * 2).max(need);
            self.buf.resize(grow, 0xFF);
        }
    }

    fn set(&mut self, x: i32, y: i32, cov: f32) {
        if x < 0 || y < 0 || x >= self.clip_right {
            return;
        }
        let xi = x as usize;
        let yi = y as usize;
        if xi >= self.w {
            return;
        }
        self.ensure(yi + 1);
        if yi + 1 > self.used {
            self.used = yi + 1;
        }
        let idx = (yi * self.w + xi) * 4;
        let cur = self.buf[idx];
        let f = 1.0 - cov;
        let nv = (cur as f32 * f) as u8;
        if nv < cur {
            self.buf[idx] = nv;
            self.buf[idx + 1] = nv;
            self.buf[idx + 2] = nv;
        }
    }

    fn erase(&mut self, x: i32, y: i32, cov: f32) {
        if x < 0 || y < 0 || x >= self.clip_right {
            return;
        }
        let xi = x as usize;
        let yi = y as usize;
        if xi >= self.w {
            return;
        }
        self.ensure(yi + 1);
        if yi + 1 > self.used {
            self.used = yi + 1;
        }
        let idx = (yi * self.w + xi) * 4;
        let cur = self.buf[idx];
        let nv = (cur as f32 + (255.0 - cur as f32) * cov) as u8;
        if nv > cur {
            self.buf[idx] = nv;
            self.buf[idx + 1] = nv;
            self.buf[idx + 2] = nv;
        }
    }

    fn fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, 1.0);
            }
        }
    }

    fn erase_fill(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        for y in y0..y1 {
            for x in x0..x1 {
                self.erase(x, y, 1.0);
            }
        }
    }
}

pub fn render(items: &[Item], width: usize, fonts: &Fonts) -> RenderedPage {
    let mut r = Renderer {
        cv: Canvas::new(width),
        fonts,
        line_spacing: None,
        left_margin: 0.0,
        print_width: None,
        logo: None,
    };
    r.cv.ensure(1024);
    for item in items {
        r.render_item(item);
    }
    let used = r.cv.used.max(r.cv.y as usize).max(1);
    r.cv.buf.truncate(used * width * 4);
    RenderedPage {
        width,
        height: used,
        rgba: r.cv.buf,
    }
}

impl<'a> Renderer<'a> {
    fn render_item(&mut self, item: &Item) {
        match item {
            Item::Text(t) => self.render_text(t),
            Item::Raster(ri) => self.render_raster(ri),
            Item::BitImage(bi) => self.render_bitimage(bi),
            Item::FeedLines(n) => {
                self.cv.y += self.cv.line_height * (*n as f32);
                self.cv.ensure(self.cv.y as usize + 64);
                self.cv.used = self.cv.used.max(self.cv.y as usize);
            }
            Item::FeedDots(n) => {
                self.cv.y += *n as f32;
                self.cv.ensure(self.cv.y as usize + 64);
                self.cv.used = self.cv.used.max(self.cv.y as usize);
            }
            Item::Cut { partial: _ } => {
                let y = self.cv.y as i32 + 4;
                let mut x: i32 = 0;
                while (x as usize) < self.cv.w {
                    self.cv.fill(x, y, ((x + 4) as usize).min(self.cv.w) as i32, y + 2);
                    x += 10;
                }                self.cv.y += 12.0;
                self.cv.used = self.cv.used.max(self.cv.y as usize);
            }
            Item::Drawer { t } => {
                self.cv.y += 8.0;
                self.draw_text_plain(&format!("*** APERTURA CAJÓN (pulso {t}ms) ***"), true);
                self.cv.y += 4.0;
            }
            Item::Barcode(b) => self.render_barcode(b),
            Item::Barcode2D(b) => self.render_2d(b),
            Item::SetLineSpacing(sp) => {
                self.line_spacing = sp.map(|n| n as f32);
                if let Some(n) = sp {
                    self.cv.line_height = *n as f32;
                }
            }
            Item::SetPos(x) => {
                self.cv.x = (self.left_margin + x).max(self.left_margin);
            }
            Item::MoveX(dx) => {
                self.cv.x = (self.cv.x + dx).max(self.left_margin);
            }
            Item::SetLeftMargin(x) => {
                self.left_margin = x.max(0.0);
                self.cv.x = self.left_margin;
            }
            Item::SetPrintArea(pa) => {
                self.left_margin = pa.left.max(0.0);
                self.print_width = Some(pa.width.max(0.0));
                self.cv.clip_right = (pa.left + pa.width).max(0.0) as i32;
                self.cv.x = self.left_margin;
            }
            Item::StoreLogo(l) => {
                self.logo = Some(l.clone());
            }
            Item::PrintLogo { scale_x, scale_y } => {
                if let Some(l) = &self.logo.clone() {
                    self.render_logo(l, *scale_x, *scale_y);
                }
            }
            Item::Init => {
                self.cv.y += 8.0;
                self.left_margin = 0.0;
                self.print_width = None;
                self.cv.clip_right = i32::MAX;
                self.cv.x = 0.0;
            }
        }
    }

    fn render_text(&mut self, item: &TextItem) {
        let avail = self.print_width.unwrap_or(self.cv.w as f32 - self.left_margin);
        let lines = self.wrap_lines(item, avail);
        if lines.is_empty() {
            return;
        }
        let item_h = item
            .runs
            .iter()
            .map(|r| self.sy_for(r))
            .fold(0.0f32, f32::max);
        let advance = self.line_spacing.unwrap_or(item_h).max(4.0);
        if let Some(f) = &self.fonts.regular {
            let asc = self.ascent_px(item_h);
            for (line, width) in &lines {
                let baseline = self.cv.y + asc;
                let ox = self.line_ox(item.align, *width, avail);
                let mut x = ox;
                for (ri, text) in line {
                    let mut run = item.runs[*ri].clone();
                    run.text = text.clone();
                    self.draw_run(f, &run, &mut x, baseline);
                }
                self.cv.y += advance;
            }
        } else {
            for (line, width) in &lines {
                let ox = self.line_ox(item.align, *width, avail);
                let mut x = ox;
                for (ri, text) in line {
                    let mut run = item.runs[*ri].clone();
                    run.text = text.clone();
                    self.draw_run_fallback(&run, &mut x);
                }
                self.cv.y += advance;
            }
        }
        self.cv.line_height = advance;
        self.cv.x = self.left_margin;
        self.cv.ensure(self.cv.y as usize + 64);
        self.cv.used = self.cv.used.max(self.cv.y as usize);
    }

    fn line_ox(&self, align: Alignment, width: f32, avail: f32) -> f32 {
        match align {
            Alignment::Center => self.left_margin + ((avail - width) / 2.0).max(0.0),
            Alignment::Right => self.left_margin + (avail - width).max(0.0),
            Alignment::Left => self.cv.x,
        }
    }

    fn char_advances(&self, run: &TextRun) -> Vec<f32> {
        // Paso fijo tipo térmica: cada carácter ocupa una celda de ancho sx
        let (sx, _sy) = self.scaled_scale(run);
        let spacing = run.style.char_spacing as f32;
        run.text.chars().map(|_| sx + spacing).collect()
    }

    fn wrap_lines(&self, item: &TextItem, avail: f32) -> Vec<(Vec<(usize, String)>, f32)> {
        let mut out: Vec<(Vec<(usize, String)>, f32)> = Vec::new();
        let mut cur: Vec<(usize, String)> = Vec::new();
        let mut cur_w = 0.0f32;
        for (ri, run) in item.runs.iter().enumerate() {
            let advs = self.char_advances(run);
            let chars: Vec<char> = run.text.chars().collect();
            let mut seg = String::new();
            for (ci, &ch) in chars.iter().enumerate() {
                let a = advs[ci];
                if !cur.is_empty() && cur_w + a > avail {
                    cur.push((ri, std::mem::take(&mut seg)));
                    out.push((std::mem::take(&mut cur), cur_w));
                    cur_w = 0.0;
                }
                seg.push(ch);
                cur_w += a;
            }
            if !seg.is_empty() {
                cur.push((ri, seg));
            }
        }
        if !cur.is_empty() {
            out.push((cur, cur_w));
        }
        out
    }

    fn sy_for(&self, run: &TextRun) -> f32 {
        let base = match run.style.font {
            FontSel::A => 24.0,
            FontSel::B => 17.0,
        };
        base * run.style.size_y.max(1) as f32
    }

    fn sx_aim(&self, run: &TextRun) -> f32 {
        let base = match run.style.font {
            FontSel::A => 12.0,
            FontSel::B => 9.0,
        };
        base * run.style.size_x.max(1) as f32
    }

    fn ascent_px(&self, h: f32) -> f32 {
        if let Some(f) = &self.fonts.regular {
            let sf = f.as_scaled(PxScale { x: h, y: h });
            sf.ascent()
        } else {
            h * 0.8
        }
    }

    fn scaled_scale(&self, run: &TextRun) -> (f32, f32) {
        let sy = self.sy_for(run);
        let sx_aim = self.sx_aim(run);
        if let Some(f) = &self.fonts.regular {
            let sf = f.as_scaled(PxScale { x: sy, y: sy });
            let adv = sf.h_advance(sf.glyph_id('0'));
            let sx = if adv > 0.0 { sy * sx_aim / adv } else { sx_aim };
            (sx, sy)
        } else {
            (sx_aim, sy)
        }
    }

    fn measure_run(&self, run: &TextRun) -> f32 {
        let (sx, _sy) = self.scaled_scale(run);
        let n = run.text.chars().count() as f32;
        let spacing = run.style.char_spacing as f32;
        (n * sx) + (n * spacing)
    }

    fn draw_run(&mut self, f: &FontVec, run: &TextRun, x: &mut f32, baseline: f32) {
        let (sx, sy) = self.scaled_scale(run);
        let fake_bold = run.style.bold && self.fonts.bold.is_none();
        let font = if run.style.bold {
            self.fonts.bold.as_ref().unwrap_or(f)
        } else {
            f
        };
        let scaled = font.as_scaled(PxScale { x: sx, y: sy });
        let run_x0 = *x;
        let run_w = self.measure_run(run);
        let spacing = run.style.char_spacing as f32;
        if run.style.reverse {
            let asc = scaled.ascent();
            let desc = scaled.descent();
            let y0 = (baseline - asc - 1.0) as i32;
            let y1 = (baseline - desc + 2.0) as i32;
            let x1 = (run_x0 + run_w) as i32;
            self.cv.fill(run_x0 as i32, y0, x1, y1);
        }
        for ch in run.text.chars() {
            let gid = scaled.glyph_id(ch);
            let glyph = gid.with_scale_and_position(
                PxScale { x: sx, y: sy },
                point(*x, baseline),
            );
            if let Some(outlined) = scaled.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                let gx = bounds.min.x;
                let gy = bounds.min.y;
                outlined.draw(|dx, dy, cov| {
                    let shear = if run.style.italic {
                        (dy as f32 - sy * 0.5) * 0.3
                    } else {
                        0.0
                    };
                    let px = (gx + dx as f32 + shear) as i32;
                    let py = (gy + dy as f32) as i32;
                    if run.style.reverse {
                        self.cv.erase(px, py, cov);
                        if fake_bold {
                            self.cv.erase(px + 1, py, cov);
                        }
                        if run.style.double_strike {
                            self.cv.erase(px, py + 1, cov);
                        }
                    } else {
                        self.cv.set(px, py, cov);
                        if fake_bold {
                            self.cv.set(px + 1, py, cov);
                        }
                        if run.style.double_strike {
                            self.cv.set(px, py + 1, cov);
                        }
                    }
                });
            }
            *x += sx + spacing;
        }
        if run.style.underline {
            let h = if run.style.underline2 {
                (sy * 0.3).max(2.0)
            } else {
                (sy * 0.15).max(1.0)
            };
            let y0 = (baseline + 1.0) as i32;
            let y1 = (baseline + 1.0 + h) as i32;
            let x1 = if run.style.reverse {
                (run_x0 + run_w) as i32
            } else {
                *x as i32
            };
            if run.style.reverse {
                self.cv.erase_fill(run_x0 as i32, y0, x1, y1);
            } else {
                self.cv.fill(run_x0 as i32, y0, x1, y1);
            }
        }
    }

    fn draw_text_plain(&mut self, text: &str, center: bool) {
        let run = TextRun {
            text: text.to_string(),
            style: crate::parser::Style::default(),
        };
        let item = TextItem {
            runs: vec![run],
            align: if center {
                Alignment::Center
            } else {
                Alignment::Left
            },
        };
        self.render_text(&item);
    }

    fn draw_run_fallback(&mut self, run: &TextRun, x: &mut f32) {
        let scale_x = self.sx_aim(run).max(1.0) as i32;
        let scale_y = self.sy_for(run) as i32;
        let baseline = self.cv.y as i32 + scale_y;
        let spacing = run.style.char_spacing as i32;
        if run.style.reverse {
            let run_w = self.measure_run(run) as i32;
            self.cv.fill(
                *x as i32,
                baseline - scale_y,
                *x as i32 + run_w,
                baseline + 2,
            );
        }
        for ch in run.text.chars() {
            let glyph = font8x8::unicode::BASIC_UNICODE
                .iter()
                .find(|f| f.0 == ch)
                .map(|f| f.1)
                .unwrap_or([0u8; 8]);
            for row in 0..8 {
                for col in 0..8 {
                    if glyph[row] & (0x80 >> col) != 0 {
                        let x0 = *x as i32 + col as i32 * scale_x;
                        let y0 = baseline - (8 - row) as i32 * scale_y;
                        if run.style.reverse {
                            self.cv.erase_fill(x0, y0, x0 + scale_x, y0 + scale_y);
                        } else {
                            self.cv.fill(x0, y0, x0 + scale_x, y0 + scale_y);
                        }
                    }
                }
            }
            *x += 8.0 * scale_x as f32 + spacing as f32;
        }
    }

    fn render_raster(&mut self, ri: &crate::parser::RasterItem) {
        let ox = match ri.align {
            Alignment::Left => 0,
            Alignment::Center => (self.cv.w as i32 - ri.width as i32).max(0) / 2,
            Alignment::Right => (self.cv.w as i32 - ri.width as i32).max(0),
        };
        self.cv.ensure(self.cv.y as usize + ri.height);
        let row_bytes = (ri.width + 7) / 8;
        for row in 0..ri.height {
            let ro = row * row_bytes;
            for col in 0..ri.width {
                let byte = ri.data[ro + col / 8];
                let bit = 7 - (col % 8);
                if byte & (1 << bit) != 0 {
                    self.cv.set(ox + col as i32, self.cv.y as i32 + row as i32, 1.0);
                }
            }
        }
        self.cv.y += ri.height as f32;
        self.cv.x = self.left_margin;
        self.cv.used = self.cv.used.max(self.cv.y as usize);
    }

    fn render_bitimage(&mut self, bi: &crate::parser::BitImageItem) {
        let ox = match bi.align {
            Alignment::Left => 0,
            Alignment::Center => (self.cv.w as i32 - bi.width as i32 * bi.spread as i32).max(0) / 2,
            Alignment::Right => (self.cv.w as i32 - bi.width as i32 * bi.spread as i32).max(0),
        };
        let height = bi.bytes_per_col * 8;
        self.cv.ensure(self.cv.y as usize + height);
        for col in 0..bi.width {
            for by in 0..bi.bytes_per_col {
                let byte = bi.data[col * bi.bytes_per_col + by];
                for bit in 0..8 {
                    if byte & (0x80 >> bit) != 0 {
                        for s in 0..bi.spread {
                            self.cv.set(
                                ox + col as i32 * bi.spread as i32 + s as i32,
                                self.cv.y as i32 + (by * 8 + bit) as i32,
                                1.0,
                            );
                        }
                    }
                }
            }
        }
        self.cv.y += height as f32;
        self.cv.x = self.left_margin;
        self.cv.used = self.cv.used.max(self.cv.y as usize);
    }

    fn render_barcode(&mut self, b: &crate::parser::BarcodeItem) {
        let module = b.module.max(1) as i32;
        let bar_h = b.height.max(8) as i32;
        let modules: Option<Vec<bool>> = match b.m {
            67 => crate::barcode::ean13_modules(&b.data),
            68 => crate::barcode::ean8_modules(&b.data),
            65 => crate::barcode::upca_as_ean13(&b.data)
                .and_then(|d| crate::barcode::ean13_modules(&d)),
            73 => crate::barcode::code128_modules(&b.data),
            70 => crate::barcode::itf_modules(&b.data),
            69 => crate::barcode::code39_modules(&b.data),
            _ => None,
        };
        let data_w = if let Some(m) = &modules {
            m.len() as i32 * module
        } else {
            b.data.len() as i32 * 8 * module
        };
        let ox = (self.cv.w as i32 - data_w).max(0) / 2;
        self.cv.ensure(self.cv.y as usize + bar_h as usize);
        let y0 = self.cv.y as i32;
        match modules {
            Some(m) => {
                for (i, bar) in m.iter().enumerate() {
                    if *bar {
                        let x0 = ox + i as i32 * module;
                        self.cv.fill(x0, y0, x0 + module, y0 + bar_h);
                    }
                }
            }
            None => {
                for (i, byte) in b.data.iter().enumerate() {
                    for bit in 0..8u32 {
                        if byte & (0x80 >> bit) != 0 {
                            let x0 = ox + (i as i32 * 8 + bit as i32) * module;
                            self.cv.fill(x0, y0, x0 + module, y0 + bar_h);
                        }
                    }
                }
            }
        }
        self.cv.y += bar_h as f32;
        self.cv.x = self.left_margin;
        self.cv.used = self.cv.used.max(self.cv.y as usize);
        if b.hri != 0 {
            let text: String = b
                .data
                .iter()
                .map(|&c| if (0x20..=0x7E).contains(&c) { c as char } else { '?' })
                .collect();
            self.draw_text_plain(&text, true);
        }
    }

    fn render_2d(&mut self, b: &Barcode2DItem) {
        let bitmap = match b.kind {
            Barcode2DKind::Qr => crate::barcode2d::qr(&b.data, b.ecc),
            Barcode2DKind::Pdf417 => crate::barcode2d::pdf417(&b.data, b.cols, b.rows, Some(b.ecc)),
        };
        let Some(bm) = bitmap else {
            return;
        };
        let module = b.module.max(1) as usize;
        let quiet = match b.kind {
            Barcode2DKind::Qr => 4,
            Barcode2DKind::Pdf417 => 2,
        };
        let total_w = (bm.width + quiet * 2) * module;
        let total_h = (bm.height + quiet * 2) * module;
        let ox = ((self.cv.w as i32 - total_w as i32).max(0)) / 2;
        let y0 = self.cv.y as i32;
        self.cv.ensure(self.cv.y as usize + total_h);
        let m = module as i32;
        let q = quiet as i32;
        for row in 0..bm.height {
            for col in 0..bm.width {
                if bm.dark[row * bm.width + col] {
                    let x0 = ox + (col as i32 + q) * m;
                    let yy0 = y0 + (row as i32 + q) * m;
                    self.cv.fill(x0, yy0, x0 + m, yy0 + m);
                }
            }
        }
        self.cv.y += total_h as f32;
        self.cv.x = self.left_margin;
        self.cv.used = self.cv.used.max(self.cv.y as usize);
    }

    fn render_logo(&mut self, l: &LogoItem, scale_x: usize, scale_y: usize) {
        let w = l.width * scale_x.max(1);
        let h = l.height * scale_y.max(1);
        let ox = (self.cv.w as i32 - w as i32).max(0) / 2;
        self.cv.ensure(self.cv.y as usize + h);
        let row_bytes = (l.width + 7) / 8;
        for row in 0..l.height {
            let ro = row * row_bytes;
            for col in 0..l.width {
                let byte = l.data[ro + col / 8];
                let bit = 7 - (col % 8);
                if byte & (1 << bit) != 0 {
                    let x0 = ox + (col * scale_x) as i32;
                    let y0 = self.cv.y as i32 + (row * scale_y) as i32;
                    for sy in 0..scale_y {
                        for sx in 0..scale_x {
                            self.cv.set(x0 + sx as i32, y0 + sy as i32, 1.0);
                        }
                    }
                }
            }
        }
        self.cv.y += h as f32;
        self.cv.x = self.left_margin;
        self.cv.used = self.cv.used.max(self.cv.y as usize);
    }
}

/// Downscale an RGBA image (nearest neighbour) so the height fits `max_h`.
pub fn downscale_rgba(w: usize, h: usize, rgba: &[u8], max_h: usize) -> (usize, usize, Vec<u8>) {
    if h <= max_h {
        return (w, h, rgba.to_vec());
    }
    let k = (h + max_h - 1) / max_h;
    let nw = (w / k).max(1);
    let nh = (h / k).max(1);
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        for x in 0..nw {
            let src = ((y * k) * w + x * k) * 4;
            let dst = (y * nw + x) * 4;
            out[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    (nw, nh, out)
}
