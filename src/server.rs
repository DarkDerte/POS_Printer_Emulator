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
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let peer = addr.to_string();
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
        let mut s = state.lock().unwrap();
        s.log("Servidor detenido".to_string());
    })
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
                                        7 | 8 => sim.paper_sensor(),
                                        10..=19 => 0x00, // estado de búfer (BSB): vacío
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

    let (page, summary, paper, transmit) = {
        let mut s = state.lock().unwrap();
        let paper = s.paper_width;
        let memory = s.memory.clone();
        let doc = Parser::parse_with(&data, &memory);
        // Persistir la memoria NV entre trabajos (logos, fuentes, densidad)
        s.memory = doc.memory.clone();
        // Guardarla también en disco para que sobreviva a la sesión
        let _ = crate::nvstore::save(&s.memory);
        let page: Option<RenderedPage> = if data.is_empty() {
            None
        } else {
            Some(render::render(&doc.items, paper, &fonts))
        };
        (page, doc.summary, paper, doc.transmit)
    };

    for t in &transmit {
        let _ = stream.write_all(t);
    }
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);

    let mut s = state.lock().unwrap();
    let id = s.next_id;
    s.next_id += 1;
    s.total_bytes += data.len() as u64;
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
    drop(s);
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
    use crate::state::SharedState;
    use std::sync::Mutex;
    use std::time::Instant;

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

