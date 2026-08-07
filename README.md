# POS Printer Emulator

Emulador de impresora POS (ESC/POS) para Windows, escrito en Rust. Crea una impresora virtual que cualquier aplicación puede usar para imprimir tickets, y muestra en pantalla una previsualización del documento junto con el historial de peticiones recibidas.

## Características

- **Servidor TCP en el puerto 9100** (protocolo RAW, estándar en impresoras térmicas de red). Cualquier aplicación que sepa imprimir a `127.0.0.1:9100` puede enviar documentos.
- **Impresora registrada en Windows**: la app crea la impresora `POS Printer Emulator` conectada a un puerto RAW local (`POSEmulator` → `127.0.0.1:9100`), usando automáticamente el driver genérico POS disponible (`Generic / Text Only`, `Generic / ESC/POS`, ...). Aparece en el diálogo de impresión de cualquier programa y se puede "imprimir" sin tocar el código.
- **Previsualización en tiempo real**: cada documento se renderiza como bitmap (58/80/112 mm de papel) con zoom, y se puede exportar a PNG.
- **Historial de peticiones**: lista de trabajos con hora, origen, tamaño, hexdump de los bytes crudos y resumen de los comandos ESC/POS detectados.
- **Registro y desinstalación** de la impresora desde la propia interfaz (con reintento elevado vía UAC si se requieren permisos de administrador).

## ESC/POS soportado

- Texto, alineación (izquierda/centro/derecha), negrita, subrayado (simple/doble), inverso (`GS b`, `GS B`, bit 6 de `ESC !`), cursiva (`ESC 4`/`ESC 5`), tamaño doble (ancho/alto), fuente A/B (`ESC !`, `ESC %`), espaciado entre caracteres (`ESC SP`, `GS SP`).
- **UTF-8 multibyte**: las secuencias UTF-8 válidas (á, ñ, €, emoji...) se decodifican automáticamente; el resto de bytes altos se decodifican por codepage.
- **Envoltorio automático**: las líneas que exceden el ancho de papel/área de impresión saltan de línea como una impresora real.
- Codepages: CP437, CP850, CP858, CP866, CP1252 (`ESC t`).
- Avance de línea/puntos (`LF`, `FF`, `ESC d`, `ESC J`), espaciado de línea (`ESC 3`, `ESC A`).
- Posicionamiento: posición absoluta (`ESC $`, `GS $`) y relativa (`ESC \`, `GS \`), márgenes izquierdo (`ESC l`, `ESC L`, `GS L`) y área de impresión (`ESC W`, `GS W`).
- Cortes de papel completo/parcial (`ESC i`, `ESC v`, `GS V`), apertura de cajón (`ESC p`).
- Códigos de barras 1D (`GS k`: CODE39, EAN-13, EAN-8, **Code 128**, **ITF**, UPC-A...) con HRI; EAN/UPC/Code 128/ITF/Code 39 se dibujan con módulos reales (barras y espacios reales, dígito de control calculado/verificado).
- Códigos 2D: **QR** (`GS ( k` cn=49, incl. modelo/tamaño/ECC y store+print) y **PDF417** (`GS ( k` cn=48).
- Imágenes raster (`GS v 0`) y bitmap (`ESC *`).
- Logos NV: definir (`FS q`, `GS ( L`, `GS 8 L`) e imprimir con escalado (`FS p`, `GS /`, `GS 8 H`).
- **Transmit de códigos 2D**: `GS ( k` fn 81 (QR y PDF417) devuelve los datos almacenados por el socket, como una impresora real.
- **Modo página**: `GS P` (área y entrada), `GS T` (dirección), texto con posicionamiento absoluto y descarga con `ESC FF`.
- **Fuente fija tipo térmica**: cada carácter ocupa una celda de ancho constante (12×24 fuente A, 9×17 fuente B), como una térmica real.
- **Estado simulado configurable en la GUI**: toggles de papel agotado, casi-fin, tapa abierta, error y cajón abierto; los bits de `DLE EOT`, `DLE ENQ`, `GS r` y ASB cambian en consecuencia.
- **Zumbador**: `ESC ( C` fn 0x06 (aviso sonoro, registrado en el resumen).
- **Fuentes descargadas**: `ESC &` y `GS ( A` (almacenadas y resumidas).
- Respuestas de estado en tiempo real: `DLE EOT 1..8/10..19`, `DLE ENQ`, `GS r` (estado de impresora/offline/error/papel/recovery/búfer, escritas al socket como en una impresora real).
- **Estado automático (ASB)**: `GS a n` envía los 4 bytes de estado al activarse **y empuja automáticamente los cambios de estado posteriores** (un hilo vigila la máquina de estado mientras la conexión esté activa).
- **Mecánica física**: el avance usa unidades de movimiento vertical reales (1/203", ~0.125 mm); botones **Feed** y **Corte** en la GUI que envían `ESC J n` y `GS V 66` por el mismo pipeline que una aplicación real.
- **Memoria NV persistente**: logos, fuentes descargadas y densidad se guardan en un fichero binario (`%LOCALAPPDATA%\pos_printer_emulator\nv.bin`) y se restauran entre sesiones.

## Requisitos

- Windows 10/11 (registro de impresora).
- [Rust](https://rustup.rs/) (edición 2024, rustc ≥ 1.85) para compilar.

## Compilar y ejecutar

```powershell
cargo run --release
```

El binario se genera en `target\release\pos_printer_emulator.exe` y no muestra ventana de consola.

> Nota: la dependencia `pdf417` 0.2.1 se provee en `vendor/pdf417` (el crate publicado incluye `#![feature(const_mut_refs)]`, estable desde Rust 1.83, que rompe el compilador stable; se patchea localmente vía `[patch.crates-io]`).

## Pruebas

```powershell
cargo test
```

Suite de 28 pruebas: parser (estilos, posicionamiento y márgenes, corte, QR/PDF417, UTF-8, ASB, raster `GS 8 L/H`, escalas `GS /`, modo página, transmit fn 81, zumbador y fuentes descargadas), estado simulado (`state.rs`), módulos de barcode reales (`barcode.rs`: EAN/Code 128/ITF/Code 39), persistencia NV (`nvstore.rs`) y pruebas de integración por socket real (`server.rs`) que verifican `DLE EOT` completo, ASB push y el transmit de QR.

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
| `server.rs`       | Listener TCP 9100, un hilo por conexión, límites de jobs, respuestas real-time (`DLE EOT 1..19`, `DLE ENQ`, `GS r`, ASB push) y transmit fn 81. |
| `parser.rs`       | Convierte bytes ESC/POS en una lista de ítems renderizables y genera el resumen. |
| `render.rs`       | Dibuja el ticket en un bitmap RGBA (fuentes del sistema + fallback de 8×8). |
| `barcode.rs`      | Dígitos de control y patrones de módulos reales (EAN/UPC, Code 128, ITF, Code 39) + validación de `GS k`. |
| `barcode2d.rs`    | Genera QR (`qrcode`) y PDF417 (`pdf417`) como bitmap.     |
| `codepages.rs`    | Tablas de conversión CP437/850/858/866/1252.              |
| `printer.rs`      | Scripts PowerShell para registrar/quitar la impresora (con UAC). |
| `nvstore.rs`      | Persistencia NV entre sesiones (fichero binario).         |
| `sample.rs`       | Documento de prueba con texto, QR/PDF417 (+transmit), barcode (EAN-13, Code 128, ITF), logos NV (`FS q`, `GS 8 L`, escalas `GS /`), modo página, buzzer y fuente descargada. |
| `main.rs`         | Interfaz egui/eframe.                                     |

## Limitaciones

- Solo funciona en Windows para el registro de la impresora (el servidor TCP es multiplataforma).
- Las fuentes descargadas (`ESC &`, `GS ( A`) se reconocen y almacenan, pero no se renderizan aún; el modo página soporta la dirección 0 (izquierda→derecha, arriba→abajo).
- El puerto RAW simula el envío de la aplicación a la impresora; no es un driver de impresora USB virtual (eso requeriría un driver de kernel firmado). El resultado es equivalente para la mayoría de software de punto de venta.
