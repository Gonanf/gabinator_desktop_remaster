use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

use config::{Config, ConfigError};

pub enum LoggerLevel {
    Info,
    Warning,
    Result,
    Error,
    Critical,
    Debug,
}

//LOGGER, will log any info, warning, result, error and critical
pub struct Logger {
    pub config: HashMap<String, String>,
}

impl Logger {
    pub fn get_config_content() -> HashMap<String, String> {
        match Config::builder()
            .add_source(config::File::with_name("gabinator_config"))
            .build()
        {
            Ok(a) => {
                return a
                    .try_deserialize::<HashMap<String, String>>()
                    .unwrap_or(Self::get_config_defaults())
            }
            Err(a) => {
                Self::log(format!("Not able to get the config file, this could be an error, aplying defaults... Error: {}",a)
                    , LoggerLevel::Info, None);
                return Self::get_config_defaults();
            }
        };
    }

    fn get_config_defaults() -> HashMap<String, String> {
        return HashMap::from([
            ("on_info".to_string(), "warn".to_string()),
            ("on_warning".to_string(), "warn".to_string()),
            ("on_result".to_string(), "warn".to_string()),
            ("on_error".to_string(), "warn".to_string()),
            ("on_critical".to_string(), "panic".to_string()),
            ("IP".to_string(), "127.0.0.1".to_string()),
            ("port".to_string(), "3000".to_string()),
        ]);
    }
    //Append a message to the log file
    fn append_file_content(content: &str) {
        let file = File::options().append(true).create(true).open("log.txt");
        if file.is_err() {
            println!("Could not open or create the log file, better check that");
            return;
        }
        let mut opened_file = file.unwrap();
        let size = opened_file.seek(SeekFrom::End(0)).unwrap();
        let mut new_content = String::new();
        if size == 0 {
            new_content = content.to_string();
        } else {
            new_content = format!("\n{}", &content);
        }
        opened_file.write_all(new_content.as_bytes());
    }

    pub fn start_new_page() {
        Self::append_file_content(
            format!(
                "\n\n----------------------------------------{:?}----------------------------------------\n\n"
                ,chrono::offset::Local::now()).as_str());
    }

    pub fn log(message: String, level: LoggerLevel, config: Option<HashMap<String, String>>) {
        let mut log_message = format!("[{:?}]", chrono::offset::Local::now());

        let print_log =
            |config: Option<HashMap<String, String>>, message: &String, type_log: &str| {
                match config
                    .unwrap_or(Self::get_config_defaults())
                    .get(format!("on_{type_log}").as_str())
                    .unwrap_or(
                        Self::get_config_defaults()
                            .get(format!("on_{type_log}").as_str())
                            .unwrap(),
                    )
                    .as_str()
                {
                    "panic" => {
                        Self::append_file_content(message.as_str());
                        Self::append_file_content(
                            format!(
                                "[{:?}]PANICKING AS DEFINED IN CONFIG",
                                chrono::offset::Local::now()
                            )
                            .as_str(),
                        );
                        panic!("{message}\nPANICKING AS DEFINED IN CONFIG");
                    }
                    "warn" => {
                        Self::append_file_content(message.as_str());
                    }
                    "debug" => {
                        Self::append_file_content(message.as_str());
                        println!("{message}");
                    }
                    _ => return,
                };
            };

        match level {
            LoggerLevel::Info => {
                log_message += "[LOG]";
                log_message += message.as_str();
                print_log(config, &log_message, "info");
            }
            LoggerLevel::Warning => {
                log_message += "[Warning]";
                log_message += message.as_str();
                print_log(config, &log_message, "warning");
            }
            LoggerLevel::Result => {
                log_message += "[Result]";
                log_message += message.as_str();
                print_log(config, &log_message, "result");
            }
            LoggerLevel::Error => {
                log_message += "[Error]";
                log_message += message.as_str();
                print_log(config, &log_message, "error");
            }
            LoggerLevel::Critical => {
                log_message += "[Critical]";
                log_message += message.as_str();
                print_log(config, &log_message, "critical");
            }
            LoggerLevel::Debug => {
                log_message += "[Debug]";
                log_message += message.as_str();
                print_log(config, &log_message, "debug");
            }
        }
    }
}

#[derive(Debug)]
pub enum GabinatorError {
    UsbError(String),
    CaptureError(String),
    MainError(String),
    LoggerError(String),
}

static DEBUG_MESSAGE: bool = true;
//Make Connection error

impl GabinatorError {
    pub fn newLogger<S: ToString>(
        message: S,
        level: LoggerLevel,
        config: Option<HashMap<String, String>>,
    ) -> Self {
        Logger::log(message.to_string(), level, config);
        GabinatorError::LoggerError(message.to_string())
    }

    pub fn newUSB<S: ToString>(
        message: S,
        level: LoggerLevel,
        config: Option<HashMap<String, String>>,
    ) -> Self {
        Logger::log(message.to_string(), level, config);
        GabinatorError::UsbError(message.to_string())
    }

    pub fn newCapture<S: ToString>(
        message: S,
        level: LoggerLevel,
        config: Option<HashMap<String, String>>,
    ) -> Self {
        Logger::log(message.to_string(), level, config);
        GabinatorError::CaptureError(message.to_string())
    }

    pub fn newMain<S: ToString>(
        message: S,
        level: LoggerLevel,
        config: Option<HashMap<String, String>>,
    ) -> Self {
        Logger::log(message.to_string(), level, config);
        GabinatorError::MainError(message.to_string())
    }
}

#[derive(Debug)]
pub enum GabinatorResult {
    UsbResult(String),
    CaptureResult(String),
    MainResult(String),
    LoggerResult(String),
}

impl GabinatorResult {
    pub fn newLogger<S: ToString>(message: S, config: Option<HashMap<String, String>>) -> Self {
        Logger::log(message.to_string(), LoggerLevel::Result, config);
        GabinatorResult::LoggerResult(message.to_string())
    }

    pub fn newUSB<S: ToString>(message: S, config: Option<HashMap<String, String>>) -> Self {
        Logger::log(message.to_string(), LoggerLevel::Result, config);
        GabinatorResult::UsbResult(message.to_string())
    }

    pub fn newCapture<S: ToString>(message: S, config: Option<HashMap<String, String>>) -> Self {
        Logger::log(message.to_string(), LoggerLevel::Result, config);
        GabinatorResult::CaptureResult(message.to_string())
    }

    pub fn newMain<S: ToString>(message: S, config: Option<HashMap<String, String>>) -> Self {
        Logger::log(message.to_string(), LoggerLevel::Result, config);
        GabinatorResult::MainResult(message.to_string())
    }
}
