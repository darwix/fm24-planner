use crate::{Error, Result, APP_ID, VERSION};
use std::path::PathBuf;

const DEFAULT_PORT: u16 = 4242;

pub struct AppConfig {
    pub port: u16,
    pub no_open: bool,
    pub data_dir: PathBuf,
}

/// What the command line asks for: run the server, or print something and exit.
pub enum Command {
    Run(AppConfig),
    Print(String),
}

impl AppConfig {
    pub fn from_args(args: impl Iterator<Item = String>) -> Result<Command> {
        let mut port = match std::env::var("FM_SQUAD_PLANNER_PORT") {
            Ok(p) => p
                .parse::<u16>()
                .map_err(|_| Error::query("Invalid FM_SQUAD_PLANNER_PORT."))?,
            Err(_) => DEFAULT_PORT,
        };
        let mut no_open = false;
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--browser" => {}
                "--no-open" => no_open = true,
                "--port" => {
                    port = args
                        .next()
                        .ok_or_else(|| Error::query("--port needs a value."))?
                        .parse()
                        .map_err(|_| Error::query("Invalid port."))?
                }
                "--version" => return Ok(Command::Print(format!("FM Squad Planner {VERSION}"))),
                "--help" => {
                    return Ok(Command::Print(
                        "FM Squad Planner server [--port PORT] [--no-open]\nFM_SQUAD_PLANNER_DATA_DIR overrides local storage. Stop with Ctrl+C.".into(),
                    ))
                }
                _ => return Err(Error::query(format!("Unknown option: {arg}"))),
            }
        }
        Ok(Command::Run(Self {
            port,
            no_open,
            data_dir: default_data_dir(),
        }))
    }
}

pub fn default_data_dir() -> PathBuf {
    std::env::var_os("FM_SQUAD_PLANNER_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_local_dir()
                .unwrap_or_else(std::env::temp_dir)
                .join(APP_ID)
        })
}

/// The game's own save folder, when it exists.
pub fn default_folder() -> String {
    dirs::document_dir()
        .map(|p| p.join("Sports Interactive/Football Manager 2024/games"))
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}
