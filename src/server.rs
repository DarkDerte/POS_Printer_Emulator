use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use eframe::egui;

use crate::parser::Parser;
use crate::render::{self, Fonts, RenderedPage};
use crate::state::{now_hm, Job, Shared};

const MAX_JOBS: usize = 400;
const READ_TIMEOUT: Duration = Duration::from_secs(30);

// Convierte los bytes recibidos en un volcado hexadecimal estilo "hex dump"
// de fábrica (16 bytes por línea: hex a la izquierda, ASCII a la derecha).
fn hex_dump_items(data: &[u8]) -> Vec<crate::parser::Item> {
    use crate::parser::{Alignment, Item, Style, TextItem, TextRun};
    let mut items = Vec::new();
    for chunk in data.chunks(16) {
        let mut hex = String::new();
        let mut asc = String::new();
        for &b in chunk {
            hex.push_str(&format!("{b:02X} "));
            asc.push(if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '.'
            });
        }
        items.push(Item::Text(TextItem {
            runs: vec![TextRun {
                text: format!("{hex:<48} {asc}"),
                style: Style::default(),
            }],
            align: Alignment::Left,
        }));
    }
    items
}

// Imprime una página de auto-test (como mantener FEED al encender en una real).
pub fn push_selftest(state: Shared, fonts: &Arc<Fonts>) {
    use crate::parser::{Alignment, Item, Style, TextItem, TextRun};
    let mut items = Vec::new();
    let mut line = |text: String, center: bool| {
        items.push(Item::Text(TextItem {
            runs: vec![TextRun {
                text,
                style: Style::default(),
            }],
            align: if center { Alignment::Center } else { Alignment::Left },
        }));
    };
    let (model, fw, serial, width, density, njobs, bytes, roll, hex) = {
        let s = state.lock().unwrap();
        (
            s.model.label().to_string(),
            s.model.firmware().to_string(),
            s.model.serial().to_string(),
            s.paper_width,
            s.memory.density,
            s.jobs.len(),
            s.total_bytes,
            s.sim.paper_mm,
            s.hex_dump,
        )
    };
    line("======== AUTO-TEST ========".to_string(), true);
    line(String::new(), false);
    line(format!("Modelo : {model}"), false);
    line(format!("Firmware: {fw}"), false);
    line(format!("Serie  : {serial}"), false);
    line(String::new(), false);
    line(format!("Papel  : {width} px (203 ppp)"), false);
    line(format!("Densidad: {density}"), false);
    line(format!("Rollo  : {roll:.1} m restantes"), false);
    line(format!("Trabajos: {njobs}  Bytes: {bytes}"), false);
    line(format!("Hex dump: {hex}"), false);
    line(String::new(), false);
    line("Comandos soportados: ESC/POS, PDF417, QR".to_string(), false);
    line("El papel se corta automáticamente al final de la página.".to_string(), false);
    line(String::new(), false);
    line("==== FIN DE AUTO-TEST ====".to_string(), true);
    line(String::new(), false);

    let page = render::render(&items, width, fonts);
    let mut s = state.lock().unwrap();
    let id = s.next_id;
    s.next_id += 1;
    s.total_bytes += 0;
    s.log("Auto-test impreso".to_string());
    s.jobs.push(Job {
        id,
        when: now_hm(),
        peer: "self-test".to_string(),
        size: 0,
        raw: Vec::new(),
        page: Some(page),
        summary: vec!["Página de auto-test generada localmente".to_string()],
    });
    if s.jobs.len() > MAX_JOBS {
        let _ = s.jobs.remove(0);
    }
}

pub fn spawn(
    state: Shared,
    fonts: Arc<Fonts>,
    port: u16,
    stop: Arc<AtomicBool>,
    ctx: egui::Context,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let listener = match TcpListener::bind(("0.0.0.0", port)) {
            Ok(l) => l,
            Err(e) => {
                let mut s = state.lock().unwrap();
                s.server_error = Some(format!("No se pudo abrir el puerto {port}: {e}"));
                s.log(format!("ERROR: {e}"));
                return;
            }
        };
        {
            let mut s = state.lock().unwrap();
            s.server_error = None;
            s.log(format!("Servidor escuchando en el puerto {port} (0.0.0.0:{port})"));
        }
        // Puerto estándar RAW 9100 (como las térmicas de red)
        match TcpListener::bind(("0.0.0.0", 9100)) {
            Ok(l) => {
                let st = state.clone();
                let f = fonts.clone();
                let c = ctx.clone();
                let stop2 = stop.clone();
                thread::spawn(move || {
                    let mut s = st.lock().unwrap();
                    s.log("RAW TCP escuchando en el puerto 9100".to_string());
                    drop(s);
                    accept_loop(l, "9100", st, f, c, stop2);
                });
            }
            Err(e) => {
                let mut s = state.lock().unwrap();
                s.log(format!("RAW 9100 no disponible: {e}"));
            }
        }
        // Descubrimiento mDNS (anuncia _pdl-datastream/_printer)
        crate::mdns::spawn_mdns(state.clone(), stop.clone());
        // Motor de impresión: drena la cola con el retardo mecánico real.
        printer_engine(state.clone(), stop.clone(), ctx.clone());
        accept_loop(listener, &port.to_string(), state.clone(), fonts, ctx, stop);
        let mut s = state.lock().unwrap();
        s.log("Servidor detenido".to_string());
    })
}

// Motor de impresión en segundo plano: procesa la cola de trabajos, aplica el
// retardo mecánico (velocidad mm/s) y consume el rollo. Si la impresora está
// offline, los trabajos quedan en cola hasta que se recupera.
fn printer_engine(state: Shared, stop: Arc<AtomicBool>, ctx: egui::Context) {
    thread::spawn(move || {
        let mut offline_logged = false;
        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            let next: Option<(u64, Option<usize>)> = {
                let mut s = state.lock().unwrap();
                if s.printing || !s.is_printable() {
                    if !s.print_queue.is_empty() && !offline_logged {
                        s.log(
                            "Impresora offline: trabajo en cola (se imprime al recuperar)"
                                .to_string(),
                        );
                        offline_logged = true;
                    }
                    None
                } else {
                    offline_logged = false;
                    s.print_queue.pop_front().map(|id| {
                        let h = s
                            .jobs
                            .iter()
                            .find(|j| j.id == id)
                            .and_then(|j| j.page.as_ref())
                            .map(|p| p.height);
                        (id, h)
                    })
                }
            };
            let Some((id, height)) = next else {
                thread::sleep(Duration::from_millis(80));
                continue;
            };
            {
                let mut s = state.lock().unwrap();
                s.printing = true;
            }
            let mm = height.unwrap_or(0) as f32 / 203.0 * 25.4;
            let (speed, instant) = {
                let s = state.lock().unwrap();
                (s.print_speed_mm_s, s.instant_print)
            };
            let secs = if instant {
                0.0
            } else {
                (mm / speed.max(1) as f32).min(3.0)
            };
            if secs > 0.0 {
                thread::sleep(Duration::from_secs_f32(secs));
            }
            {
                let mut s = state.lock().unwrap();
                s.printing = false;
                if mm > 0.0 {
                    s.sim.consume_paper(mm);
                    if s.sim.is_paper_out() {
                        s.log("ROLLO AGOTADO: inserte papel (botón Recargar)".to_string());
                    } else if s.sim.is_near_end() {
                        s.log("Aviso: papel próximo al final del rollo".to_string());
                    }
                }
                s.log(format!("Trabajo #{id} impreso (mm={mm:.1})"));
            }
            ctx.request_repaint();
        }
    });
}

fn accept_loop(
    listener: TcpListener,
    label: &str,
    state: Shared,
    fonts: Arc<Fonts>,
    ctx: egui::Context,
    stop: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, addr)) => {
                let peer = format!("{addr} (:{label})");
                let st = state.clone();
                let f = fonts.clone();
                let c = ctx.clone();
                thread::spawn(move || handle_connection(stream, peer, st, f, c));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    peer: String,
    state: Shared,
    fonts: Arc<Fonts>,
    ctx: egui::Context,
) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let mut data: Vec<u8> = Vec::new();
    let mut buf = [0u8; 8192];
    let mut pending: Vec<u8> = Vec::new();
    // Estado automático (ASB): cuando se activa, un hilo vigila los cambios de
    // estado y los envía por el socket sin necesidad de que llegue más datos.
    let mut asb_on = false;
    let pusher_stop = Arc::new(AtomicBool::new(false));
    let mut pusher: Option<thread::JoinHandle<()>> = None;
    loop {
        let mut chunk = Vec::with_capacity(pending.len() + buf.len());
        chunk.extend(pending.drain(..));
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => chunk.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
        // Apagada: no responde nada (el cliente agota su timeout), descartando datos.
        if !state.lock().unwrap().power_on {
            continue;
        }
        let mut clean = Vec::with_capacity(chunk.len());
        let mut i = 0;
        while i < chunk.len() {
            match chunk[i] {
                0x10 => {
                    if i + 1 < chunk.len() {
                        match chunk[i + 1] {
                            0x05 => {
                                // DLE ENQ: estado de impresora
                                let sim = state.lock().unwrap().sim;
                                let _ = stream.write_all(&[sim.printer_status()]);
                                i += 2;
                                continue;
                            }
                            0x04 => {
                                if i + 2 < chunk.len() {
                                        let sim = state.lock().unwrap().sim;
                                        let resp = match chunk[i + 2] {
                                            1 => sim.printer_status(),
                                            2 => sim.gs_r2(),
                                            3 | 4 => sim.error_status(),
                                            5 => sim.paper_sensor(),
                                            6 => sim.recovery_status(),
                                            7 => sim.roll_end_sensor(),
                                            8 => sim.near_end_sensor(),
                                            9 => sim.recovery_sensor(),
                                            10..=19 => state.lock().unwrap().buffer_fill(),
                                            _ => 0x00,
                                        };
                                    let _ = stream.write_all(&[resp]);
                                    i += 3;
                                    continue;
                                } else {
                                    pending.push(chunk[i]);
                                    break;
                                }
                            }
                            _ => {
                                clean.push(chunk[i]);
                                i += 1;
                                continue;
                            }
                        }
                    } else {
                        pending.push(0x10);
                        break;
                    }
                }
                0x1B => {
                    if i + 1 < chunk.len() && chunk[i + 1] == 0x75 {
                        // ESC u n: estado de periféricos (cajón kick)
                        if i + 2 < chunk.len() {
                            let sim = state.lock().unwrap().sim;
                            let resp = if sim.drawer_open { 0x01 } else { 0x00 };
                            let _ = stream.write_all(&[resp]);
                            i += 3;
                            continue;
                        } else {
                            pending.extend_from_slice(&chunk[i..]);
                            break;
                        }
                    }
                    clean.push(chunk[i]);
                    i += 1;
                }
                0x1D => {
                    if i + 1 < chunk.len() && chunk[i + 1] == 0x72 {
                        if i + 2 < chunk.len() {
                            let n = chunk[i + 2];
                            let sim = state.lock().unwrap().sim;
                            let resp = match n {
                                49 | 50 => sim.paper_sensor(),
                                2 => sim.gs_r2(),
                                _ => sim.gs_r1(),
                            };
                            let _ = stream.write_all(&[resp]);
                            i += 3;
                            continue;
                        } else {
                            pending.extend_from_slice(&chunk[i..]);
                            break;
                        }
                    } else if i + 1 < chunk.len() && chunk[i + 1] == 0x4F {
                        // GS O n: seleccionar online/offline
                        if i + 2 < chunk.len() {
                            let n = chunk[i + 2];
                            let mut s = state.lock().unwrap();
                            s.sim.offline = n != 0;
                            s.log(format!(
                                "GS O: impresora {}",
                                if n != 0 { "OFFLINE" } else { "ONLINE" }
                            ));
                            i += 3;
                            continue;
                        } else {
                            pending.extend_from_slice(&chunk[i..]);
                            break;
                        }
                    } else if i + 1 < chunk.len() && chunk[i + 1] == 0x61 {
                        // GS a n: activar/desactivar estado automático (ASB)
                        if i + 2 < chunk.len() {
                            let n = chunk[i + 2];
                            if n == 1 && !asb_on {
                                asb_on = true;
                                let sim = state.lock().unwrap().sim;
                                let _ = stream.write_all(&sim.asb());
                                if let Ok(mut w) = stream.try_clone() {
                                    let st = state.clone();
                                    let stop = pusher_stop.clone();
                                    // El empujador parte de la misma instantánea ya
                                    // respondida, para no perder cambios posteriores.
                                    pusher = Some(thread::spawn(move || {
                                        let mut last = Some(sim);
                                        loop {
                                            if stop.load(Ordering::SeqCst) {
                                                break;
                                            }
                                            let sim = st.lock().unwrap().sim;
                                            if last != Some(sim) {
                                                if w.write_all(&sim.asb()).is_err() {
                                                    break;
                                                }
                                                last = Some(sim);
                                            }
                                            thread::sleep(Duration::from_millis(50));
                                        }
                                    }));
                                }
                            } else if n == 0 && asb_on {
                                asb_on = false;
                                pusher_stop.store(true, Ordering::SeqCst);
                                if let Some(h) = pusher.take() {
                                    let _ = h.join();
                                }
                            }
                            i += 3;
                            continue;
                        } else {
                            pending.extend_from_slice(&chunk[i..]);
                            break;
                        }
                    }
                    clean.push(chunk[i]);
                    i += 1;
                }
                _ => {
                    clean.push(chunk[i]);
                    i += 1;
                }
            }
        }
        data.extend(clean);
    }
    pusher_stop.store(true, Ordering::SeqCst);
    if let Some(h) = pusher.take() {
        let _ = h.join();
    }

    let (page, summary, paper, transmit, buzz, auto_test, drawer_ms) = {
        let mut s = state.lock().unwrap();
        let paper = s.paper_width;
        let memory = s.memory.clone();
        let model = s.model;
        let hex_dump = s.hex_dump;
        // Cualquier dato entrante despierta a la impresora (como en una real)
        if !data.is_empty() && s.sim.sleeping {
            s.sim.sleeping = false;
            s.log("Impresora despierta al recibir datos".to_string());
        }
        let doc = Parser::parse_with_dip(
            &data,
            &memory,
            model,
            s.dip.initial_codepage,
            s.dip.hri_below,
            s.dip.auto_cut,
        );
        // Persistir la memoria NV entre trabajos (logos, fuentes, densidad, macros)
        s.memory = doc.memory.clone();
        // Guardarla también en disco para que sobreviva a la sesión
        let _ = crate::nvstore::save(&s.memory);
        // ESC 8 / ESC c 5: estado de los botones del panel
        s.panel_enabled = doc.panel_enabled;
        // Modo hex dump: se imprime la traza hex/ASCII de los datos en vez del parseo
        let page: Option<RenderedPage> = if data.is_empty() {
            None
        } else if hex_dump {
            Some(render::render(&hex_dump_items(&data), paper, &fonts))
        } else {
            Some(render::render(&doc.items, paper, &fonts))
        };
        (
            page,
            doc.summary,
            paper,
            doc.transmit,
            doc.buzz,
            doc.auto_test,
            doc.drawer_pulse_ms,
        )
    };

    // Apertura de cajón: ESC p/GS p abre el cajón (el pulso lo mantiene abierto
    // hasta que se cierra; el estado ASB lo refleja en tiempo real).
    if let Some(_ms) = drawer_ms {
        let mut s = state.lock().unwrap();
        s.sim.drawer_open = true;
        s.log("Cajón abierto (pulso kick) — ciérralo en la GUI".to_string());
    }

    for t in &transmit {
        let _ = stream.write_all(t);
    }
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);

    let mut s = state.lock().unwrap();
    let id = s.next_id;
    s.next_id += 1;
    s.total_bytes += data.len() as u64;
    let has_printable = !data.is_empty();
    s.log(format!(
        "Petición #{id} de {peer} ({size} bytes)",
        size = data.len()
    ));
    s.jobs.push(Job {
        id,
        when: now_hm(),
        peer,
        size: data.len(),
        raw: data,
        page,
        summary,
    });
    if s.jobs.len() > MAX_JOBS {
        let _ = s.jobs.remove(0);
    }
    // Encolar el trabajo: el motor de impresión lo imprime con el retardo
    // mecánico real y consume rollo. Si la impresora está offline queda en cola.
    if has_printable {
        s.print_queue.push_back(id);
    }
    drop(s);

    // Zumbador real (campana de consola) con el nº de timbres solicitados.
    let beeps = {
        let s = state.lock().unwrap();
        if s.dip.buzzer_enabled {
            buzz.min(5)
        } else {
            0
        }
    };
    if beeps > 0 {
        for _ in 0..beeps {
            let _ = std::io::Write::write(&mut std::io::stdout(), &[0x07]);
            let _ = std::io::stdout().flush();
            thread::sleep(Duration::from_millis(120));
        }
    }

    // GS ( E fn=5: además del documento, se imprime una página de auto-test.
    if auto_test {
        push_selftest(state.clone(), &fonts);
    }

    let _ = paper;
    ctx.request_repaint();
}

pub fn poke_to_stop(port: u16) {
    if let Ok(mut c) = TcpStream::connect(("127.0.0.1", port)) {
        let _ = c.write_all(b"");
        let _ = c.shutdown(std::net::Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Item;
    use crate::state::SharedState;
    use std::sync::Mutex;
    use std::time::Instant;

    #[test]
    fn hex_dump_formatea_bytes() {
        let data = [0x1B, 0x40, b'H', b'i'];
        let items = hex_dump_items(&data);
        assert_eq!(items.len(), 1);
        match &items[0] {
            Item::Text(t) => {
                let s = &t.runs[0].text;
                assert!(s.contains("1B 40 48 69"), "hex: {s}");
                assert!(s.contains("Hi"), "ascii: {s}");
            }
            other => panic!("hex dump debería ser texto, fue {other:?}"),
        }
    }

    fn free_port() -> u16 {
        let l = TcpListener::bind(("0.0.0.0", 0)).unwrap();
        l.local_addr().unwrap().port()
    }

    // Redirigir la persistencia NV a un fichero temporal en las pruebas para
    // no tocar el NV real del usuario.
    fn nv_tmp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("pos_nv_socket_{}", std::process::id())).join("nv.bin")
    }

    fn roundtrip(port: u16, payload: &[u8]) -> Vec<u8> {
        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        c.write_all(payload).unwrap();
        // Cerrar solo la escritura para que el servidor vea EOF limpio
        c.shutdown(std::net::Shutdown::Write).unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 256];
        loop {
            match c.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        out
    }

    #[test]
    fn estado_y_transmit_por_socket() {
        unsafe { std::env::set_var("POS_NV_PATH", nv_tmp_path()) };
        let state = Arc::new(Mutex::new(SharedState::new()));
        let fonts = Arc::new(Fonts::load());
        let stop = Arc::new(AtomicBool::new(false));
        let port = free_port();
        let ctx = egui::Context::default();
        let handle = spawn(state.clone(), fonts, port, stop.clone(), ctx);
        thread::sleep(Duration::from_millis(500));
        {
            let s = state.lock().unwrap();
            eprintln!("server_error = {:?}", s.server_error);
        }

        // Consultas de estado + ASB + QR store + transmit fn 81
        let mut payload: Vec<u8> = vec![
            0x10, 0x04, 0x01, // DLE EOT 1 -> printer status
            0x10, 0x04, 0x05, // DLE EOT 5 -> papel
            0x1D, 0x72, 0x01, // GS r 1 -> online
            0x1D, 0x61, 0x01, // GS a 1 -> ASB
        ];
        payload.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x09, 0x00, 0x31, 0x43, 0x30]);
        payload.extend_from_slice(&[0x03, 0x00, 0x00, 0x58, 0x59, 0x5A]); // datos "XYZ"
        payload.extend_from_slice(&[0x1D, 0x28, 0x6B, 0x02, 0x00, 0x31, 0x51]); // fn 81

        let resp = roundtrip(port, &payload);
        let expected: Vec<u8> = vec![0x36, 0x00, 0x18, 0x36, 0x00, 0x00, 0x00, b'X', b'Y', b'Z'];
        assert_eq!(resp, expected, "estado online + transmit");

        // Con papel agotado y error: los bits de estado deben cambiar
        {
            let mut s = state.lock().unwrap();
            s.sim.paper_out = true;
            s.sim.error = true;
        }
        let resp2 = roundtrip(port, &[0x10, 0x04, 0x01, 0x1D, 0x72, 0x01, 0x1D, 0x61, 0x01]);
        assert_eq!(resp2, vec![0x23, 0x00, 0x23, 0x01, 0x0C, 0x00], "estado con fallo");

        stop.store(true, Ordering::SeqCst);
        poke_to_stop(port);
        let _ = handle.join();
    }

    #[test]
    fn dle_eot_completo() {
        unsafe { std::env::set_var("POS_NV_PATH", nv_tmp_path()) };
        let state = Arc::new(Mutex::new(SharedState::new()));
        let fonts = Arc::new(Fonts::load());
        let stop = Arc::new(AtomicBool::new(false));
        let port = free_port();
        let ctx = egui::Context::default();
        let handle = spawn(state.clone(), fonts, port, stop.clone(), ctx);
        thread::sleep(Duration::from_millis(500));

        // n=1 estado, n=2 offline, n=3/4 error, n=5/7/8 papel, n=6 recovery, n=10 búfer
        let mut payload: Vec<u8> = Vec::new();
        for n in [1, 2, 3, 4, 5, 6, 7, 8, 10] {
            payload.extend_from_slice(&[0x10, 0x04, n]);
        }
        let resp = roundtrip(port, &payload);
        // online por defecto: 0x36, resto 0x00
        assert_eq!(resp, vec![0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // Con papel agotado y error: EOT 2/3/4/5/6 cambian
        {
            let mut s = state.lock().unwrap();
            s.sim.paper_out = true;
            s.sim.error = true;
        }
        let resp2 = roundtrip(port, &[0x10, 0x04, 2, 0x10, 0x04, 3, 0x10, 0x04, 4, 0x10, 0x04, 5]);
        // gs_r2=1 (error), error_status=0x08, papel=0x18
        assert_eq!(resp2, vec![0x01, 0x08, 0x08, 0x18]);

        stop.store(true, Ordering::SeqCst);
        poke_to_stop(port);
        let _ = handle.join();
    }

    #[test]
    fn cola_offline_gs_o_esc_u_y_buffer_por_socket() {
        unsafe { std::env::set_var("POS_NV_PATH", nv_tmp_path()) };
        let state = Arc::new(Mutex::new(SharedState::new()));
        let fonts = Arc::new(Fonts::load());
        let stop = Arc::new(AtomicBool::new(false));
        let port = free_port();
        let ctx = egui::Context::default();
        let handle = spawn(state.clone(), fonts, port, stop.clone(), ctx);
        thread::sleep(Duration::from_millis(500));

        // 1) GS O 1 pone la impresora offline (sin respuesta)
        assert!(roundtrip(port, &[0x1D, 0x4F, 0x01]).is_empty());

        // 2) Un trabajo enviado offline se queda en cola (no se imprime)
        assert!(roundtrip(port, b"Trabajo en cola\n").is_empty());
        {
            let s = state.lock().unwrap();
            assert_eq!(s.print_queue.len(), 1, "el trabajo queda encolado");
        }

        // 3) DLE EOT 10..19 -> búfer ocupado (0x10) mientras haya cola
        let mut payload: Vec<u8> = Vec::new();
        for n in 10..=19 {
            payload.extend_from_slice(&[0x10, 0x04, n]);
        }
        let resp = roundtrip(port, &payload);
        assert_eq!(resp, vec![0x10; 10], "búfer ocupado con trabajo en cola");

        // 4) ESC u n: estado del cajón (cerrado = 0x00)
        assert_eq!(roundtrip(port, &[0x1B, 0x75, 0x00]), vec![0x00]);

        // 5) Al abrir el cajón, ESC u responde 0x01
        {
            let mut s = state.lock().unwrap();
            s.sim.drawer_open = true;
        }
        assert_eq!(roundtrip(port, &[0x1B, 0x75, 0x01]), vec![0x01]);

        // 6) GS O 0 vuelve a online: la cola se drena y el búfer se vacía
        assert!(roundtrip(port, &[0x1D, 0x4F, 0x00]).is_empty());
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let r = roundtrip(port, &[0x10, 0x04, 10]);
            if r == vec![0x00] {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "el búfer no se vacía tras recuperar online: {r:?}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        // Con el cajón abierto el bit 0x20 del estado queda a 0: 0x16
        assert_eq!(roundtrip(port, &[0x10, 0x04, 1]), vec![0x16]);

        stop.store(true, Ordering::SeqCst);
        poke_to_stop(port);
        let _ = handle.join();
    }

    #[test]
    fn asb_push_por_cambio_de_estado() {
        unsafe { std::env::set_var("POS_NV_PATH", nv_tmp_path()) };
        let state = Arc::new(Mutex::new(SharedState::new()));
        let fonts = Arc::new(Fonts::load());
        let stop = Arc::new(AtomicBool::new(false));
        let port = free_port();
        let ctx = egui::Context::default();
        let handle = spawn(state.clone(), fonts, port, stop.clone(), ctx);
        thread::sleep(Duration::from_millis(500));

        let mut c = TcpStream::connect(("127.0.0.1", port)).unwrap();
        c.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
        c.write_all(&[0x1D, 0x61, 0x01]).unwrap();
        let mut b = [0u8; 4];
        c.read_exact(&mut b).unwrap();
        assert_eq!(b, [0x36, 0x00, 0x00, 0x00], "ASB inicial al activar");

        // Cambio de papel: la impresora empuja el nuevo ASB sin más entrada
        {
            let mut s = state.lock().unwrap();
            s.sim.paper_out = true;
        }
        let mut got = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while got.len() < 4 && Instant::now() < deadline {
            let mut tmp = [0u8; 4];
            match c.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => got.extend_from_slice(&tmp[..n]),
                Err(_) => {}
            }
        }
        assert_eq!(got, vec![0x26, 0x00, 0x0C, 0x00], "ASB push por papel agotado");

        // Al desactivar (GS a 0) deja de empujar aunque el estado cambie
        c.write_all(&[0x1D, 0x61, 0x00]).unwrap();
        thread::sleep(Duration::from_millis(200));
        {
            let mut s = state.lock().unwrap();
            s.sim.paper_out = false;
            s.sim.drawer_open = true;
        }
        thread::sleep(Duration::from_millis(400));
        let mut r = [0u8; 4];
        assert!(
            c.read(&mut r).is_err(),
            "no debe llegar ASB tras GS a 0"
        );

        drop(c);
        stop.store(true, Ordering::SeqCst);
        poke_to_stop(port);
        let _ = handle.join();
    }
}

