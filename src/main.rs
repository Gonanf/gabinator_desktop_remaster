use std::{
    env,
    io::{stdin, Read},
    thread,
    time::Instant,
    usize,
};
mod capture;
pub mod error;
mod usb;
use capture::capture_screen;
use error::{Logger, LoggerLevel};
use rusb::DeviceHandle;
use usb::{capture_and_send, find_compatible_usb, prepare_accesory, send_capture_data};
mod tcp;
fn main() {
    let config = Logger::get_config_content();

    //TEST
    //capture_screen();
    Logger::start_new_page();
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        return;
    }

    parse_arg(
        &args,
        "-G".to_string(),
        "--get-devices".to_string(),
        false,
        |_a: &String| match usb::find_compatible_usb(false) {
            Ok(a) => {
                for i in a {
                    println!(
                        "DEVICE {:03} ADDRESS {:03} PID {:?} VID {:?}",
                        i.bus_number(),
                        i.address(),
                        i.device_descriptor().unwrap().product_id(),
                        i.device_descriptor().unwrap().vendor_id(),
                    );
                }
                true
            }
            Err(a) => return false,
        },
    );

    parse_arg(
        &args,
        "-C".to_string(),
        "--connect".to_string(),
        false,
        |a: &String| {
            let mut vid = 0;
            let mut pid = 0;

            if parse_arg(
                &args,
                "-V".to_string(),
                "--vendor-id".to_string(),
                true,
                |a: &String| {
                    //println!("VID: {}", a);
                    vid = match a.parse() {
                        Ok(a) => a,
                        Err(a) => {
                            Logger::log(
                                format!("Not able to parse the VID: {a}"),
                                LoggerLevel::Critical,
                                Some(config.clone()),
                            );
                            return false;
                        }
                    };
                    true
                },
            ) == false
            {
                return false;
            }

            if parse_arg(
                &args,
                "-P".to_string(),
                "--product-id".to_string(),
                true,
                |a: &String| {
                    //println!("PID: {}", a);
                    pid = match a.parse() {
                        Ok(a) => a,
                        Err(a) => {
                            Logger::log(
                                format!("Not able to parse the PID: {a}"),
                                LoggerLevel::Critical,
                                Some(config.clone()),
                            );
                            return false;
                        }
                    };
                    true
                },
            ) == false
            {
                return false;
            }

            if vid > 0 && pid > 0 {
                match usb::connect_to_device(pid, vid) {
                    Ok(a) => Logger::log(
                        format!("Connected"),
                        LoggerLevel::Info,
                        Some(config.clone()),
                    ),
                    Err(a) => Logger::log(
                        format!("Not able to connect"),
                        LoggerLevel::Critical,
                        Some(config.clone()),
                    ),
                }
            }
            true
        },
    );

    parse_arg(
        &args,
        "-M".to_string(),
        "--mode".to_string(),
        true,
        |a: &String| -> bool {
            print!("{}", a);
            return true;
        },
    );
}

fn parse_arg<F>(
    list: &Vec<String>,
    argument: String,
    argument_alt: String,
    accepts_value: bool,
    mut function: F,
) -> bool
where
    F: FnMut(&String) -> bool,
{
    let config = Logger::get_config_content();
    let index = list
        .iter()
        .position(|a| a == argument.as_str() || a == argument_alt.as_str());
    if index.is_none() {
        Logger::log(
            format!("Argument not found {argument_alt}"),
            LoggerLevel::Info,
            Some(config.clone()),
        );
        return false;
    }
    if !accepts_value {
        return function(&String::new());
    }
    let index = index.unwrap();
    if list.len() == index + 1 {
        Logger::log(
            format!("{argument_alt} needs a value"),
            LoggerLevel::Warning,
            Some(config.clone()),
        );
        return false;
    }
    let var = &list[index + 1];
    function(&var.replace("\n", ""));
    Logger::log(
        format!("Found parameter {argument_alt} with value {var}"),
        LoggerLevel::Info,
        Some(config),
    );
    true
}
