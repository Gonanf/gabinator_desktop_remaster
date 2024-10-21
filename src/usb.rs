use core::{fmt, time};
use std::{
    alloc::GlobalAlloc,
    fmt::{format, Error},
    ops::Deref,
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
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
    let stop_signal = Arc::new(AtomicBool::new(true));
    let copy_stop = stop_signal.clone();
    let running = Arc::new(AtomicBool::new(true));
    let copy = running.clone();
    let config_copy = config.clone();

    //set control + c thread and behaviour
    let clone_device = Arc::new(Mutex::new(device));
    let clousure = Arc::clone(&clone_device);
    ctrlc::set_handler(move || {
        println!("Closing device...");
        copy.store(false, Ordering::SeqCst);
        let device = clousure.lock().unwrap();

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

        println!("Closed");
        copy_stop.store(false, Ordering::SeqCst);
        return;
    })
    .expect("ERRROR CTRLC");

    //TODO: Separate this into a public function (and for all devices)
    let endpoint_data = match find_bulk_endpoint(&clone_device.lock().unwrap().device()) {
        Some(a) => a,
        None => {
            error::GabinatorError::newUSB(
                "Cannot find endpoint",
                LoggerLevel::Critical,
                Some(config.clone()),
            );
            return Err(GabinatorError::newUSB(
                "Not able to find the bulk transfer endpoint",
                LoggerLevel::Critical,
                Some(config.clone()),
            ));
        }
    };

    while running.load(Ordering::SeqCst) {
        let device_new = clone_device.lock().unwrap();

        //TODO: Separar en funcion
        let data = match capture_screen() {
            Ok(a) => a,
            Err(a) => {
                GabinatorError::newUSB(
                    format!("Error capturing image: {a}"),
                    LoggerLevel::Debug,
                    Some(config.clone()),
                );
                continue;
            }
        };

        match send_USB_data(&data, &device_new, endpoint_data.address) {
            None => continue,
            Some(a) => GabinatorError::newUSB(
                format!("Not able to write bulk: {a}"),
                LoggerLevel::Error,
                Some(config.clone()),
            ),
        };
    }

    while stop_signal.load(Ordering::SeqCst) {}
    return Ok(GabinatorResult::newUSB(
        "Session succes",
        Some(config.clone()),
    ));
}

pub fn try_to_open_AOA_device() -> Result<DeviceHandle<rusb::GlobalContext>, error::GabinatorError>
{
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

pub fn initialize_AOA_device(
    device: DeviceHandle<rusb::GlobalContext>,
) -> Result<usize, rusb::Error> {
    match device.write_control(0x40, 53, 0, 0, &[0], Duration::from_secs(10)) {
        Ok(a) => Ok(a),
        Err(a) => Err(a),
    }
}

pub struct endpoint {
    config: u8,
    interface: u8,
    setting: u8,
    pub address: u8,
}

impl fmt::Display for endpoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CONFIG: {} INTERFACE: {} SETTING: {} ADDRESS: {}",
            self.config, self.interface, self.setting, self.address
        )
    }
}

pub fn find_bulk_endpoint(device: &Device<GlobalContext>) -> Option<endpoint> {
    let descriptor = device.device_descriptor().unwrap();
    for e in 0..descriptor.num_configurations() {
        let config = match device.config_descriptor(e) {
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
