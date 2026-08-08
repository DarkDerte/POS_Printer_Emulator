use crate::parser::LogoItem;

// Memoria de la impresora que persiste entre trabajos (como la NV/flash real):
// logos NV, fuentes descargadas, densidad de impresión y macros definidas.
#[derive(Clone, Debug, Default)]
pub struct PrinterMemory {
    pub logo: Option<LogoItem>,
    pub logos: Vec<LogoItem>,
    pub user_font: Vec<(u8, Vec<u8>)>,
    pub user_kanji: Vec<(u16, Vec<u8>)>,
    pub density: u8,
    // Macro definida con GS : (bytes ESC/POS que se ejecutan con GS ^).
    pub macro_bytes: Option<Vec<u8>>,
}
