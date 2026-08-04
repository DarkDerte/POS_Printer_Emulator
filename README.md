# POS Printer Emulator

Emulador de impresora POS (ESC/POS) para Windows, escrito en Rust. Crea una impresora virtual que cualquier aplicación puede usar para imprimir tickets, y muestra en pantalla una previsualización del documento junto con el historial de peticiones recibidas.

## Características

- **Servidor TCP en el puerto 9100** (protocolo RAW, estándar en impresoras térmicas de red). Cualquier aplicación que sepa imprimir a `127.0.0.1:9100` puede enviar documentos.
- **Impresora registrada en Windows**: la app crea la impresora `POS Printer Emulator` conectada a un puerto RAW local (`POSEmulator` → `127.0.0.1:9100`), de modo que aparece en el diálogo de impresión de cualquier programa y se puede "imprimir" sin tocar el código.
- **Previsualización en tiempo real**: cada documento se renderiza como bitmap (58/80/112 mm de papel) con zoom, y se puede exportar a PNG.
- **Historial de peticiones**: lista de trabajos con hora, origen, tamaño, hexdump de los bytes crudos y resumen de los comandos ESC/POS detectados.
- **Registro y desinstalación** de la impresora desde la propia interfaz (con reintento elevado vía UAC si se requieren permisos de administrador).

## ESC/POS soportado

- Texto, alineación (izquierda/centro/derecha), negrita, subrayado, tamaño de fuente doble (ancho/alto), fuente A/B.
- Codepages: CP437, CP850, CP858, CP866, CP1252 (`ESC t`).
- Avance de línea/puntos (`LF`, `FF`, `ESC d`, `ESC J`), espaciado de línea (`ESC 3`, `ESC A`).
- Cortes de papel completo/parcial (`ESC i`, `ESC v`), apertura de cajón (`ESC p`).
- Códigos de barras (`GS k`: CODE39, EAN-13, EAN-8, etc.) con HRI.
- Imágenes raster (`GS v 0`) y bitmap (`ESC *`).

## Requisitos

- Windows 10/11 (registro de impresora).
- [Rust](https://rustup.rs/) (edición 2024, rustc ≥ 1.85) para compilar.

## Compilar y ejecutar

```powershell
cargo run --release
```

El binario se genera en `target\release\pos_printer_emulator.exe` y no muestra ventana de consola.

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
| `server.rs`       | Listener TCP 9100, un hilo por conexión, límites de jobs. |
| `parser.rs`       | Convierte bytes ESC/POS en una lista de ítems renderizables y genera el resumen. |
| `render.rs`       | Dibuja el ticket en un bitmap RGBA (fuentes del sistema + fallback de 8×8). |
| `codepages.rs`    | Tablas de conversión CP437/850/858/866/1252.              |
| `printer.rs`      | Scripts PowerShell para registrar/quitar la impresora (con UAC). |
| `sample.rs`       | Documento de prueba con texto, barcode y raster.          |
| `main.rs`         | Interfaz egui/eframe.                                     |

## Limitaciones

- Solo funciona en Windows para el registro de la impresora (el servidor TCP es multiplataforma).
- El puerto RAW simula el envío de la aplicación a la impresora; no es un driver de impresora USB virtual (eso requeriría un driver de kernel firmado). El resultado es equivalente para la mayoría de software de punto de venta.
