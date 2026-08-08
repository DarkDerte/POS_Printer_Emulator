// Perfiles de firmware de impresoras reales. Cada modelo imita la identificación
// y algunos matices de respuesta del firmware original.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrinterModel {
    EpsonTmT88V,
    XprinterXp80,
    StarTsp650,
    CitizenCtS310,
}

impl PrinterModel {
    pub const ALL: [PrinterModel; 4] = [
        PrinterModel::EpsonTmT88V,
        PrinterModel::XprinterXp80,
        PrinterModel::StarTsp650,
        PrinterModel::CitizenCtS310,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            PrinterModel::EpsonTmT88V => "Epson TM-T88V",
            PrinterModel::XprinterXp80 => "Xprinter XP-T80",
            PrinterModel::StarTsp650 => "Star TSP650II",
            PrinterModel::CitizenCtS310 => "Citizen CT-S310",
        }
    }

    // Respuesta de GS I (ID de impresora) y GS ( K fn=65
    pub fn id_string(&self) -> &'static str {
        match self {
            PrinterModel::EpsonTmT88V => "EPSON TM-T88V Receipt",
            PrinterModel::XprinterXp80 => "XP-T80",
            PrinterModel::StarTsp650 => "TSP650II",
            PrinterModel::CitizenCtS310 => "CT-S310",
        }
    }

    // Versión de firmware: GS ( K fn=66
    pub fn firmware(&self) -> &'static str {
        match self {
            PrinterModel::EpsonTmT88V => "1.00",
            PrinterModel::XprinterXp80 => "V1.0.0",
            PrinterModel::StarTsp650 => "2.02",
            PrinterModel::CitizenCtS310 => "1.05",
        }
    }

    // Número de serie (GS ( K fn=67)
    pub fn serial(&self) -> &'static str {
        match self {
            PrinterModel::EpsonTmT88V => "T88V012345",
            PrinterModel::XprinterXp80 => "XP8T012345",
            PrinterModel::StarTsp650 => "TSP6T012345",
            PrinterModel::CitizenCtS310 => "CTS31012345",
        }
    }

    // Anchura de papel por defecto en píxeles (203 ppp)
    pub fn default_width_px(&self) -> usize {
        match self {
            // 80 mm
            PrinterModel::EpsonTmT88V
            | PrinterModel::XprinterXp80
            | PrinterModel::StarTsp650
            | PrinterModel::CitizenCtS310 => 576,
        }
    }
}
