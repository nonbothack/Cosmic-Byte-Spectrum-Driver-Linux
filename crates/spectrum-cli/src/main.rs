//! spectrumctl - Command-line interface for Cosmic Byte Spectrum Gaming Mouse

use std::path::PathBuf;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use log::LevelFilter;

use spectrum_core::{
    Profile, ProfileManager, SpectrumController, SpectrumDevice,
};
use spectrum_hid::{find_spectrum_device, LinuxHidRaw};
use spectrum_protocol::{
    Button, ButtonAction, PollingRate, RgbColor, RgbConfig, RgbMode, VALID_DPI_STEPS,
};

#[derive(Parser, Debug)]
#[command(
    name = "spectrumctl",
    author = "Cosmic Byte Spectrum Linux Contributors",
    version,
    about = "Control and configure the Cosmic Byte Spectrum RGB Gaming Mouse on Linux"
)]
struct Cli {
    /// Target specific hidraw device path (e.g. /dev/hidraw5)
    #[arg(short, long, global = true)]
    device: Option<PathBuf>,

    /// Output responses as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Enable verbose diagnostic logs
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Detect and list connected Spectrum devices
    List,

    /// Show device identity and connection information
    Info,

    /// Read and display current mouse settings
    Get,

    /// Manage DPI settings
    Dpi {
        #[command(subcommand)]
        command: DpiCommands,
    },

    /// Manage USB polling rate
    Polling {
        #[command(subcommand)]
        command: PollingCommands,
    },

    /// Manage RGB lighting effects and colors
    Rgb {
        #[command(subcommand)]
        command: RgbCommands,
    },

    /// Manage programmable button mappings
    Button {
        #[command(subcommand)]
        command: ButtonCommands,
    },

    /// Manage configuration profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Commit active settings to the mouse's onboard flash memory
    Save,
}

#[derive(Subcommand, Debug)]
enum DpiCommands {
    /// Get currently active DPI
    Get,
    /// Set mouse DPI (supported: 200..12800)
    Set {
        /// Target DPI value
        dpi: u16,
    },
    /// List all 23 valid hardware sensor DPI steps
    ListSteps,
}

#[derive(Subcommand, Debug)]
enum PollingCommands {
    /// Get current USB polling rate
    Get,
    /// Set USB polling rate in Hz (125, 250, 500, 1000)
    Set {
        /// Polling rate in Hz
        rate: u16,
    },
}

#[derive(Subcommand, Debug)]
enum RgbCommands {
    /// Get current RGB configuration
    Get,
    /// Set RGB mode, color, brightness, and speed
    Set(RgbSetArgs),
    /// List all 16 supported hardware RGB lighting modes
    ListModes,
}

#[derive(Args, Debug)]
struct RgbSetArgs {
    /// RGB mode name (e.g. static, neon, breathing, flow, cycle, off)
    #[arg(short, long, default_value = "neon")]
    mode: String,

    /// Hex color code (e.g. ff0000, 00ff00, 0080ff)
    #[arg(short, long, default_value = "#ff0000")]
    color: String,

    /// Brightness level (1 to 5)
    #[arg(short, long, default_value_t = 3)]
    brightness: u8,

    /// Effect animation speed (1 to 5)
    #[arg(short, long, default_value_t = 3)]
    speed: u8,

    /// Effect direction (false = forward/normal, true = reverse)
    #[arg(long, default_value_t = false)]
    reverse: bool,

    /// Symmetry mode
    #[arg(long, default_value_t = true)]
    symmetry: bool,
}

#[derive(Subcommand, Debug)]
enum ButtonCommands {
    /// List current button assignments
    List,
    /// Remap a button (Button 1..6)
    Set {
        /// Button (left, middle, right, back, forward, dpi, or 1..6)
        button: String,
        /// Action (left, middle, right, back, forward, dpi, rapidfire, ledloop, dpi+, dpi-, mode, disabled)
        action: String,
    },
}

#[derive(Subcommand, Debug)]
enum ProfileCommands {
    /// List saved configuration profiles
    List,
    /// Save current settings as a new profile
    Save {
        /// Profile name
        name: String,
        /// Optional description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Load and apply a saved profile to the mouse
    Load {
        /// Profile name (e.g. default, gaming, office)
        name: String,
    },
    /// View contents of a saved profile
    Show {
        /// Profile name
        name: String,
    },
}

fn connect_device(explicit_path: Option<&PathBuf>) -> Result<SpectrumController<LinuxHidRaw>> {
    let controller = match explicit_path {
        Some(path) => SpectrumController::connect_path(path)
            .with_context(|| format!("Failed to open specified device '{}'", path.display()))?,
        None => SpectrumController::connect_auto().context(
            "Cosmic Byte Spectrum mouse not found. Check USB connection and udev permissions.",
        )?,
    };
    Ok(controller)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    env_logger::Builder::new()
        .target(env_logger::Target::Stderr)
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    match cli.command {
        Commands::List => {
            let found = find_spectrum_device();
            if cli.json {
                let res = serde_json::json!({
                    "detected": found.is_ok(),
                    "device": found.as_ref().map(|p| p.to_string_lossy().to_string()).ok(),
                    "vendor_id": "0x30fa",
                    "product_id": "0x1440",
                    "model": "Cosmic Byte Spectrum"
                });
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                match found {
                    Ok(path) => {
                        println!("Found Cosmic Byte Spectrum Gaming Mouse:");
                        println!("  Node:        {}", path.display());
                        println!("  USB VID/PID: 30fa:1440");
                        println!("  Status:      Ready");
                    }
                    Err(e) => {
                        println!("No Cosmic Byte Spectrum mouse detected: {}", e);
                    }
                }
            }
        }

        Commands::Info => {
            let controller = connect_device(cli.device.as_ref())?;
            let info = controller.identify()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("Device Information:");
                println!("  Name:        {}", info.name);
                println!("  VID:         0x{:04x}", info.vendor_id);
                println!("  PID:         0x{:04x}", info.product_id);
                println!(
                    "  Path:        {}",
                    info.hidraw_path.as_deref().unwrap_or("unknown")
                );
                println!("  Interface:   {}", info.interface);
                println!("  Connected:   {}", if info.connected { "Yes" } else { "No" });
            }
        }

        Commands::Get => {
            let mut controller = connect_device(cli.device.as_ref())?;
            let settings = controller.get_settings()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&settings)?);
            } else {
                println!("Active Settings:");
                println!("  DPI:          {}", settings.active_dpi);
                println!("  Polling Rate: {}", settings.polling_rate);
                println!("  RGB Mode:     {}", settings.rgb.mode);
                println!("  RGB Color:    {}", settings.rgb.color);
                println!("  Brightness:   {}/5", settings.rgb.brightness + 1);
                println!("  Speed:        {}/5", settings.rgb.speed + 1);
                println!("\nDPI Stages:");
                for stage in &settings.stages {
                    let mark = if stage.enabled { "✔" } else { " " };
                    println!(
                        "  [{}] Stage {}: {:>5} DPI  ({})",
                        mark,
                        stage.stage_index + 1,
                        stage.dpi,
                        stage.color
                    );
                }
                println!("\nButton Mappings:");
                for (btn, act) in &settings.buttons {
                    println!("  Button {:?}: {:?}", btn, act);
                }
            }
        }

        Commands::Dpi { command } => match command {
            DpiCommands::Get => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let settings = controller.get_settings()?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({ "active_dpi": settings.active_dpi })
                    );
                } else {
                    println!("Current DPI: {}", settings.active_dpi);
                }
            }
            DpiCommands::Set { dpi } => {
                let mut controller = connect_device(cli.device.as_ref())?;
                controller.set_dpi(dpi)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "status": "ok", "dpi": dpi, "verified": true }));
                } else {
                    println!("✔ Successfully set and hardware-verified DPI to {} (Chunk 0x10 EEPROM)", dpi);
                }
            }
            DpiCommands::ListSteps => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&VALID_DPI_STEPS)?);
                } else {
                    println!("Supported Hardware DPI Steps (23 total):");
                    for (i, &step) in VALID_DPI_STEPS.iter().enumerate() {
                        println!("  {:>2}. {:>5} DPI", i + 1, step);
                    }
                }
            }
        },

        Commands::Polling { command } => match command {
            PollingCommands::Get => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let settings = controller.get_settings()?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "polling_rate_hz": settings.polling_rate.hz()
                        })
                    );
                } else {
                    println!("Current Polling Rate: {}", settings.polling_rate);
                }
            }
            PollingCommands::Set { rate } => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let p_rate = PollingRate::from_hz(rate)?;
                controller.set_polling_rate(p_rate)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "status": "ok", "rate_hz": rate, "verified": true }));
                } else {
                    println!("✔ Successfully set and hardware-verified polling rate to {} Hz", rate);
                }
            }
        },

        Commands::Rgb { command } => match command {
            RgbCommands::Get => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let settings = controller.get_settings()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&settings.rgb)?);
                } else {
                    println!("RGB Lighting Configuration:");
                    println!("  Mode:       {}", settings.rgb.mode);
                    println!("  Color:      {}", settings.rgb.color);
                    println!("  Brightness: {}/5", settings.rgb.brightness + 1);
                    println!("  Speed:      {}/5", settings.rgb.speed + 1);
                }
            }
            RgbCommands::ListModes => {
                let modes = [
                    RgbMode::DpiBreathing,
                    RgbMode::CycleBreathing,
                    RgbMode::StaticLight,
                    RgbMode::FlowingWater,
                    RgbMode::MonoWater,
                    RgbMode::CometStreak,
                    RgbMode::Neon,
                    RgbMode::Ambilight,
                    RgbMode::Flicker,
                    RgbMode::StarTrek,
                    RgbMode::Ripple,
                    RgbMode::Enraptured,
                    RgbMode::ButtonResponse,
                    RgbMode::Off,
                    RgbMode::SingleBreath,
                    RgbMode::CycleColor,
                ];
                if cli.json {
                    let mode_list: Vec<_> = modes
                        .iter()
                        .map(|m| {
                            serde_json::json!({
                                "id": *m as u8,
                                "name": m.name()
                            })
                        })
                        .collect();
                    println!("{}", serde_json::to_string_pretty(&mode_list)?);
                } else {
                    println!("Supported Hardware RGB Modes (16 total):");
                    for m in &modes {
                        println!("  {:>2}. {}", *m as u8, m.name());
                    }
                }
            }
            RgbCommands::Set(args) => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let mode: RgbMode = args.mode.parse()?;
                let color = RgbColor::from_hex(&args.color)?;
                let brightness = (args.brightness.saturating_sub(1)).min(4);
                let speed = (args.speed.saturating_sub(1)).min(4);

                let config = RgbConfig {
                    mode,
                    brightness,
                    speed,
                    color,
                    direction: args.reverse,
                    symmetry: args.symmetry,
                };

                controller.set_rgb(config)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "status": "ok", "mode": mode.name() }));
                } else {
                    println!("✔ Applied RGB mode: {}", mode.name());
                }
            }
        },

        Commands::Button { command } => match command {
            ButtonCommands::List => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let settings = controller.get_settings()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&settings.buttons)?);
                } else {
                    println!("Button Assignments:");
                    for (btn, act) in &settings.buttons {
                        println!("  Button {:?}: {:?}", btn, act);
                    }
                }
            }
            ButtonCommands::Set { button, action } => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let btn: Button = button.parse()?;
                let act: ButtonAction = action.parse()?;
                controller.set_button(btn, act)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "ok",
                            "button": format!("{:?}", btn),
                            "action": format!("{:?}", act)
                        })
                    );
                } else {
                    println!("✔ Successfully mapped Button {:?} -> {:?}", btn, act);
                }
            }
        },

        Commands::Profile { command } => match command {
            ProfileCommands::List => {
                let profiles = ProfileManager::list()?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&profiles)?);
                } else {
                    println!("Available Profiles:");
                    for p in &profiles {
                        println!("  • {}", p);
                    }
                }
            }
            ProfileCommands::Save { name, description } => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let settings = controller.get_settings()?;
                let profile = Profile {
                    name: name.clone(),
                    description,
                    dpi_stages: settings.stages.iter().map(|s| s.dpi).collect(),
                    active_dpi: settings.active_dpi,
                    polling_rate: settings.polling_rate.hz(),
                    rgb: settings.rgb,
                    buttons: settings.buttons.into_iter().collect(),
                };
                let path = ProfileManager::save(&profile)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "ok",
                            "saved_profile": name,
                            "path": path.display().to_string()
                        })
                    );
                } else {
                    println!("✔ Saved profile '{}' to {}", name, path.display());
                }
            }
            ProfileCommands::Load { name } => {
                let mut controller = connect_device(cli.device.as_ref())?;
                let profile = ProfileManager::load(&name)?;
                controller.apply_profile(&profile)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "status": "ok", "loaded": name }));
                } else {
                    println!("✔ Successfully loaded and applied profile '{}'", name);
                }
            }
            ProfileCommands::Show { name } => {
                let profile = ProfileManager::load(&name)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&profile)?);
                } else {
                    println!("Profile '{}':", profile.name);
                    if let Some(desc) = &profile.description {
                        println!("  Description:  {}", desc);
                    }
                    println!("  Active DPI:   {}", profile.active_dpi);
                    println!("  Polling Rate: {} Hz", profile.polling_rate);
                    println!("  RGB Mode:     {}", profile.rgb.mode);
                    println!("  RGB Color:    {}", profile.rgb.color);
                    println!("  DPI Stages:   {:?}", profile.dpi_stages);
                }
            }
        },

        Commands::Save => {
            let mut controller = connect_device(cli.device.as_ref())?;
            controller.save_to_onboard()?;
            if cli.json {
                println!("{}", serde_json::json!({ "status": "ok", "onboard_save": true }));
            } else {
                println!("✔ Successfully committed all settings to mouse onboard memory.");
            }
        }
    }

    Ok(())
}
