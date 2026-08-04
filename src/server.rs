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
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);

    let (page, summary, paper) = {
        let s = state.lock().unwrap();
        let paper = s.paper_width;
        let doc = Parser::parse(&data);
        let page: Option<RenderedPage> = if data.is_empty() {
            None
        } else {
            Some(render::render(&doc.items, paper, &fonts))
        };
        (page, doc.summary, paper)
    };

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
