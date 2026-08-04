use std::path::{Path, PathBuf};
use std::process::Command;

pub enum RegResult {
    Ok(String),
    NeedsAdmin(String),
}

const REGISTER_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$okFile = "$env:TEMP\pos_emu_reg_ok"
$errFile = "$env:TEMP\pos_emu_reg_err"
Remove-Item $okFile,$errFile -ErrorAction SilentlyContinue
try {
    if (-not (Get-PrinterPort -Name 'POSEmulator' -ErrorAction SilentlyContinue)) {
        Add-PrinterPort -Name 'POSEmulator' -PrinterHostAddress '127.0.0.1' -PortNumber 9100
    }
    if (-not (Get-Printer -Name 'POS Printer Emulator' -ErrorAction SilentlyContinue)) {
        Add-Printer -Name 'POS Printer Emulator' -DriverName 'Generic / Text Only' -PortName 'POSEmulator'
    }
    Set-Content -Path $okFile -Value 'ok'
} catch {
    Set-Content -Path $errFile -Value $_.Exception.Message
}
"#;

const UNREGISTER_SCRIPT: &str = r#"
$okFile = "$env:TEMP\pos_emu_unreg_ok"
$errFile = "$env:TEMP\pos_emu_unreg_err"
Remove-Item $okFile,$errFile -ErrorAction SilentlyContinue
try {
    $ErrorActionPreference = 'Stop'
    Remove-Printer -Name 'POS Printer Emulator' -ErrorAction SilentlyContinue
    Remove-PrinterPort -Name 'POSEmulator' -ErrorAction SilentlyContinue
    Set-Content -Path $okFile -Value 'ok'
} catch {
    Set-Content -Path $errFile -Value $_.Exception.Message
}
"#;

fn write_script(name: &str, content: &str) -> Option<PathBuf> {
    let p = std::env::temp_dir().join(name);
    std::fs::write(&p, content).ok()?;
    Some(p)
}

fn run_powershell(script: &Path) -> (bool, String) {
    match Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .output()
    {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => (false, e.to_string()),
    }
}

fn run_elevated(script: &Path) -> (bool, String) {
    let arg = format!("-NoProfile -ExecutionPolicy Bypass -File \"{}\"", script.display());
    let inner = format!(
        "Start-Process -FilePath 'powershell.exe' -Verb RunAs -Wait -ArgumentList '{}'",
        arg
    );
    match Command::new("powershell")
        .args(["-NoProfile", "-Command"])
        .arg(inner)
        .output()
    {
        Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => (false, e.to_string()),
    }
}

fn check_marker(ok_file: &str, err_file: &str) -> RegResult {
    if Path::new(ok_file).exists() {
        RegResult::Ok("Operación completada correctamente".to_string())
    } else {
        let err = std::fs::read_to_string(err_file).unwrap_or_else(|_| "error desconocido".into());
        RegResult::NeedsAdmin(err)
    }
}

pub fn register_printer() -> RegResult {
    let Some(script) = write_script("pos_emulator_register.ps1", REGISTER_SCRIPT) else {
        return RegResult::NeedsAdmin("No se pudo escribir el script temporal".to_string());
    };
    let (_, _stderr) = run_powershell(&script);
    check_marker(
        &std::env::temp_dir().join("pos_emu_reg_ok").display().to_string(),
        &std::env::temp_dir().join("pos_emu_reg_err").display().to_string(),
    )
}

pub fn register_printer_uac() -> RegResult {
    let Some(script) = write_script("pos_emulator_register.ps1", REGISTER_SCRIPT) else {
        return RegResult::NeedsAdmin("No se pudo escribir el script temporal".to_string());
    };
    let (_, _stderr) = run_elevated(&script);
    check_marker(
        &std::env::temp_dir().join("pos_emu_reg_ok").display().to_string(),
        &std::env::temp_dir().join("pos_emu_reg_err").display().to_string(),
    )
}

pub fn unregister_printer() -> RegResult {
    let Some(script) = write_script("pos_emulator_unregister.ps1", UNREGISTER_SCRIPT) else {
        return RegResult::NeedsAdmin("No se pudo escribir el script temporal".to_string());
    };
    let (_, _stderr) = run_powershell(&script);
    check_marker(
        &std::env::temp_dir().join("pos_emu_unreg_ok").display().to_string(),
        &std::env::temp_dir().join("pos_emu_unreg_err").display().to_string(),
    )
}
