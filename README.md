# POS Printer Emulator

Emulador de impresora POS (ESC/POS) para Windows, escrito en Rust. Crea una impresora virtual que cualquier aplicación puede usar para imprimir tickets, y muestra en pantalla una previsualización del documento junto con el historial de peticiones recibidas.

## Características

- **Servidor TCP en el puerto 9100** (protocolo RAW, estándar en impresoras térmicas de red) además del puerto configurable. Cualquier aplicación que sepa imprimir a `127.0.0.1:9100` puede enviar documentos.
- **Descubrimiento mDNS/ZeroConf**: responde a consultas `_pdl-datastream._tcp.local` y `_printer._tcp.local`, anunciando la impresora (raw 9100) para Windows/Android. *(En Windows el puerto mDNS 5353 suele estar ocupado por el sistema, así que el anuncio funciona en Linux/WSL/macOS.)*
- **Impresora registrada en Windows**: la app crea la impresora `POS Printer Emulator` conectada a un puerto RAW local (`POSEmulator` → `127.0.0.1:9100`), usando automáticamente el driver genérico POS disponible (`Generic / Text Only`, `Generic / ESC/POS`, ...). Aparece en el diálogo de impresión de cualquier programa y se puede "imprimir" sin tocar el código.
- **Previsualización en tiempo real**: cada documento se renderiza como bitmap (58/80/112 mm de papel) con aspecto de papel térmico (tono crema), con zoom, y se puede exportar a PNG.
- **Historial de peticiones**: lista de trabajos con hora, origen, tamaño, hexdump de los bytes crudos y resumen de los comandos ESC/POS detectados.
- **Registro y desinstalación** de la impresora desde la propia interfaz (con reintento elevado vía UAC si se requieren permisos de administrador).

## ESC/POS soportado

- Texto, alineación (izquierda/centro/derecha), negrita, subrayado (simple/doble), inverso (`GS b`, `GS B`, bit 6 de `ESC !`), cursiva (`ESC 4`/`ESC 5`), tamaño doble (ancho/alto), fuente A/B (`ESC !`, `ESC %`), espaciado entre caracteres (`ESC SP`, `GS SP`).
- **UTF-8 multibyte**: las secuencias UTF-8 válidas (á, ñ, €, emoji...) se decodifican automáticamente; el resto de bytes altos se decodifican por codepage.
- **Envoltorio automático**: las líneas que exceden el ancho de papel/área de impresión saltan de línea como una impresora real.
- Codepages: CP437, CP850, CP858, CP866, CP1250/1251/1253/1256, CP874 y multibyte CP932/936/949/950 (`ESC t`), incl. **kanji Shift-JIS** (`FS &`/`FS .`) y kanji definido por el usuario (`FS 2`, `FS "`).
- Avance de línea/puntos (`LF`, `FF`, `ESC d`, `ESC J`), espaciado de línea (`ESC 3`, `ESC A`).
- Posicionamiento: posición absoluta (`ESC $`, `GS $`) y relativa (`ESC \`, `GS \`), márgenes izquierdo (`ESC l`, `ESC L`, `GS L`) y área de impresión (`ESC W`, `GS W`).
- Cortes de papel completo/parcial (`ESC i`, `ESC v`, `GS V`), apertura de cajón (`ESC p`, incl. dos cajones).
- Códigos de barras 1D (`GS k`: CODE39, EAN-13, EAN-8, **Code 128**, **ITF**, **UPC-A**, **UPC-E**, **Codabar**, **Code 93**) con **HRI** configurable (posición encima/debajo/ambas con `GS H`, fuente A/B con `GS f`); se dibujan con módulos reales (barras y espacios reales, dígito de control calculado/verificado).
- Códigos 2D: **QR** (`GS ( k` cn=49, incl. modelo/tamaño/ECC y store+print) y **PDF417** (`GS ( k` cn=48).
- Imágenes raster (`GS v 0`) y bitmap (`ESC *`).
- Logos NV: definir (`FS q`, `GS ( L`, `GS 8 L`) e imprimir con escalado (`FS p`, `GS /`, `GS 8 H`); **multi-logo NV** (hasta 32) con conteo transmitido (`GS ( L` fn 73/74/75).
- **Transmit de códigos 2D**: `GS ( k` fn 81 (QR y PDF417) devuelve los datos almacenados por el socket, como una impresora real.
- **Modo página**: `GS P` (área y entrada), `GS T` (dirección 0..3 con rotación real), texto con posicionamiento absoluto y descarga con `ESC FF`.
- **Perfiles de modelo**: selector de firmware (Epson TM-T88V, Xprinter XP-T80, Star TSP650II, Citizen CT-S310) que cambia la identificación (`GS I`, `GS ( K` fn 65/66/67: modelo, firmware y serie) y la anchura por defecto.
- **Fuente fija tipo térmica**: cada carácter ocupa una celda de ancho constante (12×24 fuente A, 9×17 fuente B), como una térmica real.
- **Estado simulado configurable en la GUI**: toggles de sin papel y casi-fin (derivados del **rollo finito**), tapa abierta, error, **cuchilla atascada**, sleep y dos cajones; los bits de `DLE EOT`, `DLE ENQ`, `GS r` y ASB cambian en consecuencia.
- **Rollo finito**: cada trabajo consume papel (mm reales a 203 ppp); al agotarse la impresora pasa a offline y muestra "SIN PAPEL" (botón **Recargar rollo**).
- **Sleep/standby**: con el toggle activo la impresora responde offline; cualquier dato entrante la despierta.
- **Alimentación**: el toggle **Encendida** apaga la máquina (no responde a la red hasta volver a encenderse).
- **Velocidad de impresión y retardo mecánico**: slider de velocidad (mm/s) con retardo real al imprimir, o **Impresión instantánea** para pruebas.
- **Zumbador**: `ESC ( C` fn 0x06 (aviso sonoro real de consola).
- **Modo hex dump**: imprime los bytes recibidos como volcado hex/ASCII (toggle en GUI).
- **Auto-test**: botón que imprime una página de diagnóstico (modelo, firmware, serie, papel, densidad, rollo restante), como el auto-test real al mantener FEED al encender.
- **Fuentes descargadas**: `ESC &` y `GS ( A` (almacenadas y resumidas).
- Respuestas de estado en tiempo real: `DLE EOT 1..19` (incl. sensores de rollo 7/8 y recuperación 9), `DLE ENQ`, `GS r` (estado de impresora/offline/error/papel/recovery/búfer, escritas al socket como en una impresora real).
- **Estado automático (ASB)**: `GS a n` envía los 4 bytes de estado al activarse **y empuja automáticamente los cambios de estado posteriores** (un hilo vigila la máquina de estado mientras la conexión esté activa).
- **Mecánica física**: el avance usa unidades de movimiento vertical reales (1/203", ~0.125 mm); botones **Feed** y **Corte** en la GUI que envían `ESC J n` y `GS V 66` por el mismo pipeline que una aplicación real.
- **Cola de impresión en tiempo real**: cada trabajo entrante se encola y el **motor de impresión** lo procesa por orden: retardo real según velocidad (mm/s), consumo de rollo y log al terminar; una luz **Imprimiendo** se enciende mientras hay trabajo y **PAUSA** interrumpe la cola.
- **Panel DIP y botones**: interruptores DIP en la GUI (codepage inicial, auto-corte, posición HRI, ancho de papel 58/80/112 mm, zumbador) y botones Feed/Corte/Auto-test que se desactivan con `ESC 8` / `ESC c 5` como en una térmica real. El DIP se aplica al render: **HRI abajo** hace que los códigos de barras muestren el texto debajo por defecto, y con **auto-corte desactivado** los comandos de corte (`GS V`, `ESC i`, `ESC m`) no dibujan la línea de cuchilla (no hay cuchilla instalada).
- **Búfer y estado de ocupado**: `DLE EOT` n=10..19 devuelve el % de búfer real y `GS O n` (online/offline) y `ESC u n` (estado del cajón) responden en tiempo real.
- **Macros**: definición (`GS :`) y ejecución (`GS ^`) con guarda de profundidad.
- **Copias**: `GS #` duplica el contenido pendiente del documento.
- **Margen derecho**: `ESC Q n` limita el área de impresión.
- **Restablecer fábrica**: botón que restaura codepage, densidad, auto-corte, HRI y demás ajustes DIP.
- **Memoria NV persistente**: logos, fuentes descargadas, macro y densidad se guardan en un fichero binario (`%LOCALAPPDATA%\pos_printer_emulator\nv.bin`) y se restauran entre sesiones.

## Requisitos

- Windows 10/11 (registro de impresora).
- [Rust](https://rustup.rs/) (edición 2024, rustc ≥ 1.85) para compilar.

## Compilar y ejecutar

```powershell
cargo run --release
```

El binario se genera en `target\release\pos_printer_emulator.exe` y no muestra ventana de consola.

> Nota: la dependencia `pdf417` 0.2.1 se vende en `vendor/pdf417` (el crate publicado incluye `#![feature(const_mut_refs)]`, estable desde Rust 1.83, que rompe el compilador stable; se patchea localmente vía `[patch.crates-io]`).

## Pruebas

```powershell
cargo test
```

Suite de 60 pruebas: parser (estilos, posicionamiento y márgenes, corte, QR/PDF417, UTF-8, ASB, raster `GS 8 L/H`, escalas `GS /`, modo página con rotación, transmit fn 81, multi-logo, kanji, identificación por modelo `GS I`/`GS ( K`, HRI `GS H`/`GS f`, macros `GS :`/`GS ^`, copias `GS #`, panel `ESC 8`/`ESC c 5`, margen `ESC Q`, densidad/auto-test `GS ( E`, buzzer `ESC ( C`, cajón `ESC p`, periférico `ESC u`, DIP HRI/cuchilla), estado simulado (`sim.rs`, incl. sensores de rollo/cuchilla), módulos de barcode reales (`barcode.rs`: EAN/UPC/Code 128/ITF/Code 39/Codabar/Code 93), persistencia NV (`nvstore.rs`), respuesta mDNS (`mdns.rs`), hex dump (`server.rs`) y pruebas de integración por socket real (`server.rs`) que verifican `DLE EOT` completo (incl. búfer 10..19), ASB push, transmit de QR, cola offline con `GS O`, `ESC u` y recuperación online.

## Uso

1. Ejecuta la aplicación. El servidor TCP 9100 arranca automáticamente.
2. (Opcional) Pulsa **Registrar impresora en Windows**. Si tu usuario no es administrador, usa **Reintentar como administrador (UAC)**.
3. Envía un documento:
   - Pulsa **Enviar documento de prueba** en la barra superior, o
   - desde otra aplicación elige la impresora `POS Printer Emulator` e imprime, o
   - usa `netcat`/`curl`/tu propio código contra `127.0.0.1:9100`:

     ```powershell
     python -c "import socket;socket.create_connection(('127.0.0.1',9100)).sendall(b'Hello ESC/POS\n')"
     ```

4. Selecciona el trabajo en el panel izquierdo para ver el preview, el hexdump y el resumen de comandos.

Para dejar de usar la impresora, pulsa **Quitar impresora**.

> Nota: los comandos `Add-Printer`/`Remove-Printer` requieren permisos de administrador. Si la app no los tiene, se mostrará la opción de reintentar con elevación UAC.

## Arquitectura

```
                       ┌─────────────────────────────────┐
  Aplicaciones ──TCP──▶│  Servidor RAW 127.0.0.1:9100    │
  (diálogo imprimir,   └──────────────────┬──────────────┘
   scripts, red)                          ▼
                       ┌─────────────────────────────────┐
                       │  Parser ESC/POS  ──▶  Items     │
                       └──────────────────┬──────────────┘
                                          ▼
                       ┌─────────────────────────────────┐
                       │  Renderer (ab_glyph + font8x8)  │──▶ Preview PNG
                       └─────────────────────────────────┘
```

| Módulo            | Responsabilidad                                           |
| ----------------- | --------------------------------------------------------- |
| `server.rs`       | Listener TCP 9100 + puerto configurable, un hilo por conexión, límites de jobs, respuestas real-time (`DLE EOT 1..19`, `DLE ENQ`, `GS r`, ASB push), transmit fn 81, rollo finito, retardo de impresión, hex dump y auto-test. |
| `parser.rs`       | Convierte bytes ESC/POS en una lista de ítems renderizables y genera el resumen. |
| `render.rs`       | Dibuja el ticket en un bitmap RGBA con aspecto de papel térmico (fuentes del sistema + fallback de 8×8). |
| `barcode.rs`      | Dígitos de control y patrones de módulos reales (EAN/UPC, Code 128, ITF, Code 39, Codabar, Code 93) + validación de `GS k`. |
| `barcode2d.rs`    | Genera QR (`qrcode`) y PDF417 (`pdf417`) como bitmap.     |
| `codepages.rs`    | Tablas de conversión y decodificadores multibyte (CP437..1256, CP932/936/949/950, kanji). |
| `model.rs`        | Perfiles de firmware (Epson/Xprinter/Star/Citizen): ID, firmware, serie y anchura por defecto. |
| `mdns.rs`         | Responder mDNS/ZeroConf que anuncia la impresora (`_pdl-datastream`, `_printer`). |
| `printer.rs`      | Scripts PowerShell para registrar/quitar la impresora (con UAC). |
| `nvstore.rs`      | Persistencia NV entre sesiones (fichero binario).         |
| `sample.rs`       | Documento de prueba con texto, QR/PDF417 (+transmit), barcode (EAN-13, Code 128, ITF), logos NV (`FS q`, `GS 8 L`, escalas `GS /`), modo página, buzzer y fuente descargada. |
| `state.rs` / `sim.rs` | Estado compartido (`SharedState`), máquina de estado simulada (`SimState`), interruptores DIP y cola de impresión. |
| `memory.rs`      | Memoria de la impresora: NV logos, fuentes descargadas, densidad y macro. |
| `main.rs`         | Interfaz egui/eframe.                                     |

## Limitaciones

- Solo funciona en Windows para el registro de la impresora (el servidor TCP es multiplataforma).
- El responder mDNS puede no arrancar en Windows si el puerto 5353 está ocupado por el mDNS del sistema (funciona en Linux/WSL/macOS).
- Las fuentes descargadas (`ESC &`, `GS ( A`) se reconocen y almacenan, pero no se renderizan aún.
- El puerto RAW simula el envío de la aplicación a la impresora; no es un driver de impresora USB virtual (eso requeriría un driver de kernel firmado). El resultado es equivalente para la mayoría de software de punto de venta.
