use core::time;
use std::{
    alloc::GlobalAlloc,
    fmt::{format, Error},
    ops::Deref,
    ptr::{null, null_mut},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    capture::capture_screen,
    error::{self, GabinatorError, GabinatorResult, Logger, LoggerLevel},
};
use config::Config;
use rusb::{
    self, open_device_with_vid_pid, Device, DeviceDescriptor, DeviceHandle, Direction,
    GlobalContext, LogLevel, TransferType,
};
use xcb::x::Time;

const manufacturer: &str = "Chaos";
const modelName: &str = "EEST";
const description: &str = "Gabinator";
const version: &str = "1.0";
const uri: &str = "https://github.com/Gonanf/Gabinator_Android/tree/master";
const serialNumber: &str = "1990";

//Finds and returns the AOA Compatible devices
pub fn find_compatible_usb<'a>(
    ignore_already_initialized: bool,
) -> Result<Vec<Device<rusb::GlobalContext>>, error::GabinatorError> {
    let config = Logger::get_config_content();
    let devices = rusb::devices();
    let mut compatible_devices: Vec<Device<rusb::GlobalContext>> = Vec::new();
    if devices.is_err() {
        return Err(error::GabinatorError::newUSB(
            "Failed to get devices",
            error::LoggerLevel::Error,
            Some(config.clone()),
        ));
    }
    for device in devices.unwrap().iter() {
        let descriptor: rusb::DeviceDescriptor = device.device_descriptor().unwrap();

        Logger::log(
            format!(
                "Bus {:03} Device {:03} ID {:04x}:{:04x}",
                device.bus_number(),
                device.address(),
                descriptor.vendor_id(),
                descriptor.product_id()
            ),
            LoggerLevel::Info,
            Some(config.clone()),
        );

        let device_handle: DeviceHandle<rusb::GlobalContext> = match device.open() {
            Ok(a) => a,
            Err(_) => {
                error::GabinatorError::newUSB(
                    "Could not open this device",
                    error::LoggerLevel::Warning,
                    Some(config.clone()),
                );
                continue;
            }
        };
        let result = is_in_AOA(&device_handle, descriptor);
        if result.is_some() {
            if !ignore_already_initialized {
                compatible_devices.push(device);
                continue;
            }
        };

        let result = get_AOA_version(&device_handle);
        if result.is_ok() {
            compatible_devices.push(device);
            continue;
        };
    }
    return Ok(compatible_devices);
}

pub fn connect_to_device(pid: u16, vid: u16) -> Result<GabinatorResult, GabinatorError> {
    let config = Logger::get_config_content();
    let device = match open_device_with_vid_pid(vid, pid) {
        Some(a) => a,
        None => {
            return Err(GabinatorError::newUSB(
                "Failed to open, maybe device does not exist?",
                error::LoggerLevel::Error,
                Some(config.clone()),
            ))
        }
    };

    match device.set_active_configuration(0) {
        Ok(a) => a,
        Err(a) => Logger::log(
            format!("Failed to set active configuration: {a}"),
            LoggerLevel::Critical,
            Some(config.clone()),
        ),
    }

    match device.set_auto_detach_kernel_driver(true) {
        Ok(a) => a,
        Err(a) => Logger::log(
            format!("Failed to detach kernel driver: {a}"),
            LoggerLevel::Critical,
            Some(config.clone()),
        ),
    }

    /*match device.claim_interface(0) {
        Ok(a) => a,
        Err(a) => Logger::log(
            format!("Failed to claim interface: {a}"),
            LoggerLevel::Critical,
            Some(config.clone()),
        ),
    } */

    let result = initialize_AOA_device(device);
    if result.is_err() {
        GabinatorError::newUSB(
            format!(
                "Failed to initialize AOA protocol on this device {}",
                result.unwrap_err()
            ),
            error::LoggerLevel::Error,
            Some(config.clone()),
        );
    }
    let device = match try_to_open_AOA_device() {
        Ok(a) => a,
        Err(a) => {
            return Err(GabinatorError::newUSB(
                format!("Failed to open AOA device"),
                LoggerLevel::Critical,
                Some(config.clone()),
            ))
        }
    };
    let running = Arc::new(AtomicBool::new(true));
    let copy = running.clone();
    let config_copy = config.clone();
    match device.claim_interface(0) {
        Ok(a) => a,
        Err(a) => {
            GabinatorError::newUSB(
                format!("Cannot claim interface {a}"),
                LoggerLevel::Error,
                Some(config.clone()),
            );
        }
    }

    //set control + c thread and behaviour
    ctrlc::set_handler(move || {
        println!("Closing device...");
        match device.unconfigure() {
            Ok(a) => {
                Logger::log(format!("Device reconfigured"), LoggerLevel::Debug, None);
            }
            Err(a) => {
                GabinatorError::newUSB(
                    format!("Cannot unconfigure the device, the device needs to be disconnected phisicaly: {a}"),
                    LoggerLevel::Error,
                    Some(config_copy.clone()),
                );
            }
        };

        match device.release_interface(0) {
            Ok(a) => {
                Logger::log(format!("Device released"), LoggerLevel::Debug, None);
            }
            Err(a) => {
                GabinatorError::newUSB(
                    format!("Cannot release the device, the device needs to be disconnected phisicaly: {a}"),
                    LoggerLevel::Error,
                    Some(config_copy.clone()),
                );
            }
        };

        match device.reset() {
            Ok(a) => {
                Logger::log(format!("Device reseted"), LoggerLevel::Debug, None);
            }
            Err(a) => {
                GabinatorError::newUSB(
                    format!("Cannot reset the device, the device needs to be disconnected phisicaly: {a}"),
                    LoggerLevel::Error,
                    Some(config_copy.clone()),
                );
            }
        };

        copy.store(false, Ordering::SeqCst);
        return;
    }).expect("ERRROR CTRLC");

    find_bulk_endpoint(&device);

    while running.load(Ordering::SeqCst) {}
    return Ok(GabinatorResult::newUSB(
        "Session succes",
        Some(config.clone()),
    ));
}

fn try_to_open_AOA_device() -> Result<DeviceHandle<rusb::GlobalContext>, error::GabinatorError> {
    let config = Logger::get_config_content();
    for _i in 0..10 {
        let value = open_device_with_vid_pid(0x18d1, 0x2d00);
        match value {
            Some(a) => {
                Logger::log(
                    format!("FOUND AT 0x2d00"),
                    LoggerLevel::Info,
                    Some(config.clone()),
                );
                return Ok(a);
            }
            None => error::GabinatorError::newUSB(
                "0x18d1:0x2d00 unopenable",
                LoggerLevel::Error,
                Some(config.clone()),
            ),
        };

        let value = open_device_with_vid_pid(0x18d1, 0x2d01);
        match value {
            Some(a) => {
                Logger::log(
                    format!("FOUND AT 0x2d01"),
                    LoggerLevel::Info,
                    Some(config.clone()),
                );
                return Ok(a);
            }
            None => error::GabinatorError::newUSB(
                "0x18d1:0x2d01 unopenable",
                LoggerLevel::Error,
                Some(config.clone()),
            ),
        };
        sleep(time::Duration::from_secs(1));
    }
    return Err(error::GabinatorError::newUSB(
        "This is not an AOA device (or at least one that is openable)",
        LoggerLevel::Error,
        Some(config),
    ));
}

fn is_in_AOA(
    device: &DeviceHandle<rusb::GlobalContext>,
    descriptor: DeviceDescriptor,
) -> Option<error::GabinatorError> {
    let config = Logger::get_config_content();
    if descriptor.vendor_id() != 0x18d1 {
        return Some(error::GabinatorError::newUSB(
            "Device is not in AOA mode",
            LoggerLevel::Warning,
            Some(config),
        ));
    }
    if descriptor.product_id() == 0x2d00 || descriptor.product_id() == 0x2d01 {
        return None;
    }

    Some(error::GabinatorError::newUSB(
        "Device is not an AOA device",
        LoggerLevel::Warning,
        Some(config),
    ))
}

fn get_AOA_version(
    device: &DeviceHandle<rusb::GlobalContext>,
) -> Result<u8, error::GabinatorError> {
    let config = Logger::get_config_content();

    let mut version_buffer = vec![0u8; 2];
    if device
        .read_control(
            0xc0,
            51,
            0,
            0,
            &mut version_buffer,
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB(
            "Could not read control to device",
            LoggerLevel::Error,
            Some(config),
        ));
    }
    let version_AOA = (version_buffer[1] << 7) | version_buffer[0];
    if version_AOA > 2 || version_AOA <= 0 {
        return Err(error::GabinatorError::newUSB(
            "Device does not support AOA",
            LoggerLevel::Warning,
            Some(config),
        ));
    }
    return Ok(version_AOA);
}

fn initialize_AOA_device(device: DeviceHandle<rusb::GlobalContext>) -> Result<usize, rusb::Error> {
    match device.write_control(0x40, 53, 0, 0, &[0], Duration::from_secs(10)) {
        Ok(a) => Ok(a),
        Err(a) => Err(a),
    }
}

struct endpoint {
    config: u8,
    interface: u8,
    setting: u8,
    address: u8,
}

fn find_bulk_endpoint(device: &DeviceHandle<GlobalContext>) -> Option<endpoint> {
    let descriptor = device.device().device_descriptor().unwrap();
    for e in 0..descriptor.num_configurations() {
        let config = match device.device().config_descriptor(e) {
            Ok(a) => a,
            Err(_) => {
                continue;
            }
        };

        for interface in config.interfaces() {
            for int_descriptors in interface.descriptors() {
                for endpoint in int_descriptors.endpoint_descriptors() {
                    if endpoint.direction() == Direction::Out
                        && endpoint.transfer_type() == TransferType::Bulk
                    {
                        return Some(endpoint {
                            config: endpoint.number(),
                            interface: int_descriptors.interface_number(),
                            setting: int_descriptors.setting_number(),
                            address: endpoint.address(),
                        });
                    }
                }
            }
        }
    }
    None
}

pub fn capture_and_send(
    handler: &DeviceHandle<GlobalContext>,
    endpoint_data: u8,
) -> Option<rusb::Error> {
    let data = match capture_screen() {
        Ok(a) => a,
        Err(a) => return Some(rusb::Error::Other),
    };
    send_USB_data(&data, handler, endpoint_data)
}

pub fn prepare_accesory(handler: &DeviceHandle<GlobalContext>) -> Result<endpoint, rusb::Error> {
    let config = Logger::get_config_content();

    if rusb::supports_detach_kernel_driver() {
        if handler
            .kernel_driver_active(0)
            .expect("Error obteniendo estado de drivers")
        {
            println!("Kernel Drivers Active");
            match handler.detach_kernel_driver(0) {
                Ok(_) => println!("Kernel Drivers Detached"),
                Err(a) => {
                    dbg!(a);
                    return Err(a);
                }
            }
        }
    }
    match handler.claim_interface(0) {
        Ok(_) => println!("Interface claimed"),
        Err(a) => {
            dbg!(a);
            return Err(a);
        }
    }
    let descriptor = handler
        .device()
        .device_descriptor()
        .expect("No pudo obtener el descriptor");
    let endpoint_data = match find_bulk_endpoint(&handler, descriptor) {
        Some(a) => a,
        None => {
            error::GabinatorError::newUSB(
                "Cannot find endpoint",
                LoggerLevel::Critical,
                Some(config),
            );
            return Err(rusb::Error::NotFound);
        }
    };
    return Ok(endpoint_data);
}

pub fn send_USB_data(
    data: &Vec<u8>,
    handler: &DeviceHandle<GlobalContext>,
    endpoint_data: u8,
) -> Option<rusb::Error> {
    let result = handler.write_bulk(endpoint_data, &data, Duration::from_millis(5000));
    if result.is_err() {
        return Some(result.unwrap_err());
    }
    return None;
}
