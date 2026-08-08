// Responder mDNS/ZeroConf mínimo: anuncia la impresora como servicio
// `_pdl-datastream._tcp.local` (raw 9100) para que Windows/Android la
// descubran sin instalar driver.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::state::Shared;

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const PDL_SVC: &[u8] = b"_pdl-datastream._tcp.local";
const PRN_SVC: &[u8] = b"_printer._tcp.local";
const RAW_PORT: u16 = 9100;

// Nombre de instancia con el nombre del servicio al final.
fn instance_name(inst: &str, svc: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for label in inst.split('.') {
        let lb = label.as_bytes();
        if !lb.is_empty() {
            out.push(lb.len() as u8);
            out.extend_from_slice(lb);
        }
    }
    out.push(0);
    out.extend_from_slice(svc);
    out.push(0);
    out
}

// Codifica un nombre de dominio en etiquetas (para rdata).
fn encode_name(name: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for label in name.split('.') {
        if !label.is_empty() {
            let lb = label.as_bytes();
            out.push(lb.len() as u8);
            out.extend_from_slice(lb);
        }
    }
    out.push(0);
    out
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_be_bytes());
}
fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

// Dirección IP local no-loopback (truco del socket "connectado" sin enviar nada).
fn lan_ip() -> Ipv4Addr {
    if let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) {
        if sock.connect(("8.8.8.8", 80)).is_ok() {
            if let Ok(addr) = sock.local_addr() {
                if let std::net::IpAddr::V4(v4) = addr.ip() {
                    return v4;
                }
            }
        }
    }
    Ipv4Addr::LOCALHOST
}

fn build_response(query: &[u8], instance: &str, ip: Ipv4Addr) -> Option<Vec<u8>> {
    // Cabecera DNS: id + flags 0x8400
    if query.len() < 12 {
        return None;
    }
    let q = query[12..].to_vec();
    // Extraer el nombre (etiquetas terminadas en 0x00) como texto "a.b.local."
    let mut labels = Vec::new();
    let mut idx = 0;
    loop {
        if idx >= q.len() {
            return None;
        }
        let len = q[idx] as usize;
        if len == 0 {
            idx += 1;
            break;
        }
        if len > 63 || idx + 1 + len > q.len() {
            return None;
        }
        labels.push(String::from_utf8_lossy(&q[idx + 1..idx + 1 + len]).to_ascii_lowercase());
        idx += 1 + len;
    }
    if q.len() < idx + 4 {
        return None;
    }
    let qtype = u16::from_be_bytes([q[idx], q[idx + 1]]);
    let name_lower = format!("{}.", labels.join("."));

    let inst_lc = instance.to_ascii_lowercase();
    let svc = if name_lower == "_pdl-datastream._tcp.local."
        || name_lower == format!("{inst_lc}._pdl-datastream._tcp.local.")
    {
        Some(PDL_SVC)
    } else if name_lower == "_printer._tcp.local."
        || name_lower == format!("{inst_lc}._printer._tcp.local.")
    {
        Some(PRN_SVC)
    } else {
        None
    };
    let Some(svc) = svc else {
        return None;
    };

    let inst_name = instance_name(instance, svc);
    let host_name = encode_name("pos-printer-emulator.local");
    let mut answers = Vec::new();

    // PTR: instance -> service (solo cuando la query es PTR sobre el servicio)
    if qtype == 12 {
        // PTR rdata: nombre de instancia
        let mut a = Vec::new();
        push_u16(&mut a, 12); // PTR
        push_u16(&mut a, 0x8001); // class + cache-flush
        push_u32(&mut a, 120);
        push_u16(&mut a, inst_name.len() as u16);
        a.extend_from_slice(&inst_name);
        answers.push(a);

        // SRV: servicio -> host:puerto
        let mut srv = Vec::new();
        push_u16(&mut srv, 33);
        push_u16(&mut srv, 0x8001);
        push_u32(&mut srv, 120);
        let mut rdata = Vec::new();
        push_u16(&mut rdata, 0); // prioridad
        push_u16(&mut rdata, 0); // peso
        push_u16(&mut rdata, RAW_PORT);
        rdata.extend_from_slice(&host_name);
        push_u16(&mut srv, rdata.len() as u16);
        srv.extend_from_slice(&rdata);
        answers.push(srv);

        // A
        let mut a_rec = Vec::new();
        push_u16(&mut a_rec, 1);
        push_u16(&mut a_rec, 0x8001);
        push_u32(&mut a_rec, 120);
        push_u16(&mut a_rec, 4);
        a_rec.extend_from_slice(&ip.octets());
        answers.push(a_rec);

        // TXT
        let mut txt = Vec::new();
        push_u16(&mut txt, 16);
        push_u16(&mut txt, 0x8001);
        push_u32(&mut txt, 4500);
        let attrs: [&[u8]; 3] = [b"pdl=application/octet-stream", b"ty=Thermal", b"product=POS Emulator"];
        let mut rdata = Vec::new();
        for attr in attrs {
            rdata.push(attr.len() as u8);
            rdata.extend_from_slice(attr);
        }
        push_u16(&mut txt, rdata.len() as u16);
        txt.extend_from_slice(&rdata);
        answers.push(txt);
    } else if qtype == 33 {
        // SRV directo
        let mut srv = Vec::new();
        push_u16(&mut srv, 33);
        push_u16(&mut srv, 0x8001);
        push_u32(&mut srv, 120);
        let mut rdata = Vec::new();
        push_u16(&mut rdata, 0);
        push_u16(&mut rdata, 0);
        push_u16(&mut rdata, RAW_PORT);
        rdata.extend_from_slice(&host_name);
        push_u16(&mut srv, rdata.len() as u16);
        srv.extend_from_slice(&rdata);
        answers.push(srv);
    } else if qtype == 1 {
        // A directo
        let mut a_rec = Vec::new();
        push_u16(&mut a_rec, 1);
        push_u16(&mut a_rec, 0x8001);
        push_u32(&mut a_rec, 120);
        push_u16(&mut a_rec, 4);
        a_rec.extend_from_slice(&ip.octets());
        answers.push(a_rec);
    }

    if answers.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    out.extend_from_slice(&query[0..2]); // id
    push_u16(&mut out, 0x8400); // flags: respuesta, authoritative
    push_u16(&mut out, 0); // qdcount (no repetimos la pregunta)
    push_u16(&mut out, answers.len() as u16);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    for a in answers {
        out.push(0xC0);
        out.push(0x0C); // puntero a la pregunta
        out.extend_from_slice(&a);
    }
    Some(out)
}

pub fn spawn_mdns(state: Shared, stop: Arc<AtomicBool>) {
    thread_spawn(move || {
        let sock = match UdpSocket::bind(("0.0.0.0", MDNS_PORT)) {
            Ok(s) => s,
            Err(e) => {
                let mut s = state.lock().unwrap();
                s.log(format!("mDNS no disponible ({e})"));
                return;
            }
        };
        if let Err(e) = sock.join_multicast_v4(&MDNS_ADDR, &Ipv4Addr::UNSPECIFIED) {
            let mut s = state.lock().unwrap();
            s.log(format!("mDNS: no se pudo unir al grupo multicast ({e})"));
            return;
        }
        let _ = sock.set_read_timeout(Some(Duration::from_millis(200)));
        let ip = lan_ip();
        let mut buf = [0u8; 4096];
        {
            let label = state.lock().unwrap().model.label().to_string();
            let mut s = state.lock().unwrap();
            s.log(format!("mDNS activo: {label} (raw TCP en {RAW_PORT})"));
        }
        while !stop.load(Ordering::SeqCst) {
            let (n, src) = match sock.recv_from(&mut buf) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let query = &buf[..n];
            let instance = {
                let s = state.lock().unwrap();
                format!("{}-Emulador", s.model.label().replace(' ', "-"))
            };
            let Some(resp) = build_response(query, &instance, ip) else {
                continue;
            };
            let dst = if src.ip() == std::net::IpAddr::V4(MDNS_ADDR) {
                SocketAddr::new(MDNS_ADDR.into(), MDNS_PORT)
            } else {
                src
            };
            let _ = sock.send_to(&resp, dst);
        }
    });
}

fn thread_spawn(f: impl FnOnce() + Send + 'static) {
    std::thread::spawn(f);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptr_query(svc: &str) -> Vec<u8> {
        let mut q = Vec::new();
        q.extend_from_slice(&[0x12, 0x34]); // id
        q.extend_from_slice(&[0, 0]); // flags
        q.extend_from_slice(&[0, 1]); // qdcount
        q.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // an/ns/ar
        for label in svc.trim_end_matches('.').split('.') {
            q.push(label.len() as u8);
            q.extend_from_slice(label.as_bytes());
        }
        q.push(0);
        q.extend_from_slice(&[0, 12]); // PTR
        q.extend_from_slice(&[0, 1]); // IN
        q
    }

    #[test]
    fn responde_a_query_pdl() {
        let resp = build_response(&ptr_query("_pdl-datastream._tcp.local"), "MiPrinter", Ipv4Addr::LOCALHOST)
            .expect("debería responder a _pdl-datastream");
        assert_eq!(resp[2], 0x84, "flags de respuesta");
        let ancount = u16::from_be_bytes([resp[6], resp[7]]);
        assert_eq!(ancount, 4, "PTR+SRV+A+TXT");
    }

    #[test]
    fn responde_a_consulta_por_instancia() {
        let resp = build_response(
            &ptr_query("MiPrinter._pdl-datastream._tcp.local"),
            "MiPrinter",
            Ipv4Addr::LOCALHOST,
        )
        .expect("debería responder a la instancia");
        assert_eq!(u16::from_be_bytes([resp[6], resp[7]]), 4);
    }

    #[test]
    fn no_responde_a_otro_servicio() {
        let query = ptr_query("_ipp._tcp.local");
        assert!(build_response(&query, "MiPrinter", Ipv4Addr::LOCALHOST).is_none());
    }
}
