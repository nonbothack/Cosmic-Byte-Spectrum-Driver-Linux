//! Cosmic Byte Spectrum Desktop Control Server
//!
//! Serves the responsive, modern glassmorphic web dashboard
//! connected directly to the Spectrum HID driver and core engine.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use anyhow::{Context, Result};
use clap::Parser;
use log::{error, info, warn};

use spectrum_core::{
    DeviceInfo, ProfileManager, SpectrumController, SpectrumDevice,
};
use spectrum_hid::LinuxHidRaw;
use spectrum_protocol::{DpiStage, PollingRate, RgbColor, RgbConfig, RgbMode};

const HTML: &str = include_str!("web/index.html");
const CSS: &str = include_str!("web/style.css");
const JS: &str = include_str!("web/app.js");

#[derive(Parser, Debug)]
#[command(
    name = "spectrum-gui",
    author = "Cosmic Byte Spectrum Linux Contributors",
    version,
    about = "Desktop GUI Control Center for Cosmic Byte Spectrum Gaming Mouse"
)]
struct Args {
    /// Port to bind web dashboard
    #[arg(short, long, default_value_t = 4567)]
    port: u16,

    /// Do not automatically launch default web browser
    #[arg(long)]
    no_browser: bool,

    /// Specify hidraw device node (e.g. /dev/hidraw5)
    #[arg(short, long)]
    device: Option<PathBuf>,
}

type DeviceLock = Arc<Mutex<Option<SpectrumController<LinuxHidRaw>>>>;

fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .init();

    let args = Args::parse();

    // Try connecting to device
    let controller = match &args.device {
        Some(path) => match SpectrumController::connect_path(path) {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Could not connect to {}: {}", path.display(), e);
                None
            }
        },
        None => match SpectrumController::connect_auto() {
            Ok(c) => Some(c),
            Err(e) => {
                warn!("Spectrum mouse not detected yet: {}", e);
                None
            }
        },
    };

    let device_lock: DeviceLock = Arc::new(Mutex::new(controller));

    // Bind server port
    let mut port = args.port;
    let listener = loop {
        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(l) => break l,
            Err(e) => {
                if port < args.port + 20 {
                    port += 1;
                } else {
                    return Err(e).context("Failed to bind port for spectrum-gui");
                }
            }
        }
    };

    let url = format!("http://127.0.0.1:{}", port);
    info!("Cosmic Byte Spectrum GUI Control Center active at {}", url);
    println!("============================================================");
    println!("  Cosmic Byte Spectrum RGB Gaming Mouse — Linux Control Center");
    println!("  Dashboard URL: {}", url);
    println!("============================================================");

    if !args.no_browser {
        let launch_url = url.clone();
        thread::spawn(move || {
            thread::sleep(std::time::Duration::from_millis(300));
            let _ = std::process::Command::new("xdg-open")
                .arg(&launch_url)
                .spawn();
        });
    }

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let dev_clone = Arc::clone(&device_lock);
                thread::spawn(move || {
                    if let Err(e) = handle_client(s, dev_clone) {
                        error!("HTTP client error: {}", e);
                    }
                });
            }
            Err(e) => error!("Connection failed: {}", e),
        }
    }

    Ok(())
}

fn with_device<R, F>(dev_lock: &DeviceLock, f: F) -> std::result::Result<R, String>
where
    F: Fn(&mut SpectrumController<LinuxHidRaw>) -> std::result::Result<R, spectrum_core::CoreError>,
{
    let mut guard = dev_lock.lock().unwrap();

    // 1. If we have a cached controller, attempt the operation
    if let Some(ctrl) = guard.as_mut() {
        match f(ctrl) {
            Ok(val) => return Ok(val),
            Err(e) => {
                warn!("Device operation error on existing handle: {}. Attempting auto-reconnect...", e);
            }
        }
    }

    // 2. Either no controller or first attempt failed (e.g. device replugged to new /dev/hidraw node)
    match SpectrumController::connect_auto() {
        Ok(mut new_ctrl) => {
            let res = f(&mut new_ctrl);
            *guard = Some(new_ctrl);
            res.map_err(|e| e.to_string())
        }
        Err(e) => {
            *guard = None;
            Err(format!("Mouse not connected: {}", e))
        }
    }
}

fn handle_client(mut stream: TcpStream, dev_lock: DeviceLock) -> Result<()> {
    let mut reader = BufReader::new(&stream);
    let mut req_line = String::new();
    if reader.read_line(&mut req_line)? == 0 {
        return Ok(());
    }

    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }

    let method = parts[0];
    let path = parts[1];

    // Read headers
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if line.to_lowercase().starts_with("content-length:") {
            if let Some(val) = line.split(':').nth(1) {
                content_length = val.trim().parse().unwrap_or(0);
            }
        }
    }

    // Read body if POST
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body)?;
    }

    // Routing
    match (method, path) {
        ("GET", "/") | ("GET", "/index.html") => {
            send_response(&mut stream, 200, "text/html; charset=utf-8", HTML.as_bytes())?;
        }
        ("GET", "/style.css") => {
            send_response(&mut stream, 200, "text/css; charset=utf-8", CSS.as_bytes())?;
        }
        ("GET", "/app.js") => {
            send_response(
                &mut stream,
                200,
                "application/javascript; charset=utf-8",
                JS.as_bytes(),
            )?;
        }
        ("GET", "/api/info") => {
            let info = with_device(&dev_lock, |ctrl| ctrl.identify())
                .unwrap_or_else(|_| DeviceInfo::default());
            let json = serde_json::to_vec(&info)?;
            send_response(&mut stream, 200, "application/json", &json)?;
        }
        ("GET", "/api/settings") => {
            match with_device(&dev_lock, |ctrl| ctrl.get_settings()) {
                Ok(settings) => {
                    let json = serde_json::to_vec(&settings)?;
                    send_response(&mut stream, 200, "application/json", &json)?;
                }
                Err(err) => {
                    let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                    send_response(&mut stream, 500, "application/json", &err_json)?;
                }
            }
        }
        ("GET", "/api/profiles") => {
            let profiles = ProfileManager::list().unwrap_or_else(|_| vec!["default".to_string()]);
            let json = serde_json::to_vec(&profiles)?;
            send_response(&mut stream, 200, "application/json", &json)?;
        }
        ("POST", "/api/dpi") => {
            #[derive(serde::Deserialize)]
            struct DpiReq {
                dpi: u16,
            }
            if let Ok(req) = serde_json::from_slice::<DpiReq>(&body) {
                match with_device(&dev_lock, |ctrl| ctrl.set_dpi(req.dpi)) {
                    Ok(()) => {
                        send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                    }
                    Err(err) => {
                        let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                        send_response(&mut stream, 500, "application/json", &err_json)?;
                    }
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid payload\"}",
                )?;
            }
        }
        ("POST", "/api/dpi-stage") => {
            if let Ok(stage) = serde_json::from_slice::<DpiStage>(&body) {
                match with_device(&dev_lock, |ctrl| ctrl.set_dpi_stage(stage.clone())) {
                    Ok(()) => {
                        send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                    }
                    Err(err) => {
                        let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                        send_response(&mut stream, 500, "application/json", &err_json)?;
                    }
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid stage\"}",
                )?;
            }
        }
        ("POST", "/api/polling") => {
            #[derive(serde::Deserialize)]
            struct PollingReq {
                rate: u16,
            }
            if let Ok(req) = serde_json::from_slice::<PollingReq>(&body) {
                if let Ok(prate) = PollingRate::from_hz(req.rate) {
                    match with_device(&dev_lock, |ctrl| ctrl.set_polling_rate(prate)) {
                        Ok(()) => {
                            send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                        }
                        Err(err) => {
                            let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                            send_response(&mut stream, 500, "application/json", &err_json)?;
                        }
                    }
                } else {
                    send_response(
                        &mut stream,
                        400,
                        "application/json",
                        b"{\"error\":\"Invalid rate\"}",
                    )?;
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid payload\"}",
                )?;
            }
        }
        ("POST", "/api/rgb") => {
            #[derive(serde::Deserialize)]
            struct RgbReq {
                mode: String,
                color: String,
                brightness: u8,
                speed: u8,
                direction: bool,
                symmetry: bool,
            }
            if let Ok(req) = serde_json::from_slice::<RgbReq>(&body) {
                let mode: RgbMode = req.mode.parse().unwrap_or(RgbMode::Neon);
                let color = RgbColor::from_hex(&req.color).unwrap_or(RgbColor::new(255, 0, 0));
                let config = RgbConfig {
                    mode,
                    color,
                    brightness: req.brightness.min(4),
                    speed: req.speed.min(4),
                    direction: req.direction,
                    symmetry: req.symmetry,
                };
                match with_device(&dev_lock, |ctrl| ctrl.set_rgb(config.clone())) {
                    Ok(()) => {
                        send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                    }
                    Err(err) => {
                        let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                        send_response(&mut stream, 500, "application/json", &err_json)?;
                    }
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid payload\"}",
                )?;
            }
        }
        ("POST", "/api/led-off") => {
            let config = RgbConfig {
                mode: RgbMode::Off,
                color: RgbColor::new(0, 0, 0),
                brightness: 0,
                speed: 0,
                direction: false,
                symmetry: true,
            };
            match with_device(&dev_lock, |ctrl| ctrl.set_rgb(config.clone())) {
                Ok(()) => {
                    send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                }
                Err(err) => {
                    let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                    send_response(&mut stream, 500, "application/json", &err_json)?;
                }
            }
        }
        ("POST", "/api/button") => {
            #[derive(serde::Deserialize)]
            struct BtnReq {
                button: String,
                action: String,
            }
            if let Ok(req) = serde_json::from_slice::<BtnReq>(&body) {
                if let (Ok(btn), Ok(act)) = (req.button.parse(), req.action.parse()) {
                    match with_device(&dev_lock, |ctrl| ctrl.set_button(btn, act)) {
                        Ok(()) => {
                            send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                        }
                        Err(err) => {
                            let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                            send_response(&mut stream, 500, "application/json", &err_json)?;
                        }
                    }
                } else {
                    send_response(
                        &mut stream,
                        400,
                        "application/json",
                        b"{\"error\":\"Unknown button or action\"}",
                    )?;
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid payload\"}",
                )?;
            }
        }
        ("POST", "/api/save") => {
            match with_device(&dev_lock, |ctrl| ctrl.save_to_onboard()) {
                Ok(()) => {
                    send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\",\"saved\":true}")?;
                }
                Err(err) => {
                    let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                    send_response(&mut stream, 500, "application/json", &err_json)?;
                }
            }
        }
        ("POST", "/api/profiles/load") => {
            #[derive(serde::Deserialize)]
            struct LoadReq {
                name: String,
            }
            if let Ok(req) = serde_json::from_slice::<LoadReq>(&body) {
                if let Ok(prof) = ProfileManager::load(&req.name) {
                    match with_device(&dev_lock, |ctrl| ctrl.apply_profile(&prof)) {
                        Ok(()) => {
                            send_response(&mut stream, 200, "application/json", b"{\"status\":\"ok\"}")?;
                        }
                        Err(err) => {
                            let err_json = serde_json::to_vec(&serde_json::json!({ "error": err }))?;
                            send_response(&mut stream, 500, "application/json", &err_json)?;
                        }
                    }
                } else {
                    send_response(
                        &mut stream,
                        404,
                        "application/json",
                        b"{\"error\":\"Profile not found\"}",
                    )?;
                }
            } else {
                send_response(
                    &mut stream,
                    400,
                    "application/json",
                    b"{\"error\":\"Invalid payload\"}",
                )?;
            }
        }
        _ => {
            send_response(
                &mut stream,
                404,
                "text/plain",
                b"404 Not Found - Cosmic Byte Spectrum Control",
            )?;
        }
    }

    Ok(())
}

fn send_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Status",
    };

    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len()
    );

    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}
