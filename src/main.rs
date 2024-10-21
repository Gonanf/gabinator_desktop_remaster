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
use error::{GabinatorError, Logger, LoggerLevel};
use rusb::{DeviceHandle, GlobalContext};
use usb::{capture_and_send, find_compatible_usb};
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

    let mut vid = 0;
    let mut pid = 0;
    let mut verbose = false;
    let mut mode: u8 = 0;
    let mut endpoint = 0;
    let mut device = None;

    parse_arg(
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
    );

    parse_arg(
        &args,
        "-v".to_string(),
        "--verbose".to_string(),
        false,
        |a: &String| {
            //println!("VID: {}", a);
            verbose = true;
            true
        },
    );

    parse_arg(
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
    );

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

    //Connect to the device and send capture
    parse_arg(
        &args,
        "-C".to_string(),
        "--connect".to_string(),
        false,
        |a: &String| {
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

    //Setup mode (USB or TCP)
    parse_arg(
        &args,
        "-M".to_string(),
        "--mode".to_string(),
        true,
        |a: &String| -> bool {
            match a.as_str() {
                "AOA" => {
                    if pid > 0 && vid > 0 {
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
                }

                "TCP" => todo!(),

                a => Logger::log(
                    format!("not valid {a}"),
                    LoggerLevel::Error,
                    Some(config.clone()),
                ),
            }
            return true;
        },
    );

    //Get device endpoint
    parse_arg(
        &args,
        "-e".to_string(),
        "--get-endpoint".to_string(),
        false,
        |a: &String| -> bool {
            if vid > 0 && pid > 0 {
                match usb::find_bulk_endpoint(
                    &rusb::open_device_with_vid_pid(vid, pid)
                        .expect("Cannot open device")
                        .device(),
                ) {
                    Some(a) => {
                        endpoint = a.address;
                        if verbose {
                            Logger::log(
                                format!("Endpoint: {a}"),
                                LoggerLevel::Info,
                                Some(config.clone()),
                            );
                        } else {
                            Logger::log(
                                format!("{}", a.address),
                                LoggerLevel::Debug,
                                Some(config.clone()),
                            );
                        }
                    }
                    None => Logger::log(
                        format!("Didnt found a bulk transfer endpoint"),
                        LoggerLevel::Critical,
                        Some(config.clone()),
                    ),
                }
            }
            true
        },
    );

    //Send current frame to device
    parse_arg(
        &args,
        "-S".to_string(),
        "--send-frame".to_string(),
        false,
        |a: &String| -> bool {
            if pid > 0 && vid > 0 && endpoint > 0 && device.is_some() {
                match usb::capture_and_send(&device.as_mut().unwrap(), endpoint) {
                    Some(a) => {
                        let _b = GabinatorError::newMain(
                            "Error sending image {a}",
                            LoggerLevel::Critical,
                            Some(config.clone()),
                        );
                    }

                    None => {
                        if verbose {
                            println!("Sending complete without errors");
                        };
                    }
                }
                true;
            }
            return true;
        },
    );

    //Open AOA device
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
