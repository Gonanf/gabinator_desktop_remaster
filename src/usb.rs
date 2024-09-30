use std::{
    alloc::GlobalAlloc,
    fmt::Error,
    ptr::{null, null_mut},
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{capture::capture_screen, error};
use rusb::{
    self, open_device_with_vid_pid, DeviceDescriptor, DeviceHandle, Direction, GlobalContext,
    TransferType,
};

const manufacturer: &str = "Chaos";
const modelName: &str = "EEST";
const description: &str = "Gabinator";
const version: &str = "1.0";
const uri: &str = "https://github.com/Gonanf/Gabinator_Android/tree/master";
const serialNumber: &str = "1990";

pub fn find_compatible_usb() -> Result<DeviceHandle<rusb::GlobalContext>, error::GabinatorError> {
    let devices = rusb::devices();
    if devices.is_err() {
        return Err(error::GabinatorError::newUSB("Failed to get devices"));
    }
    for device in devices.unwrap().iter() {
        let descriptor: rusb::DeviceDescriptor = device.device_descriptor().unwrap();
        println!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x}",
            device.bus_number(),
            device.address(),
            descriptor.vendor_id(),
            descriptor.product_id()
        );

        //coloca error
        let device_handle: DeviceHandle<rusb::GlobalContext> = match device.open() {
            Ok(a) => a,
            Err(_) => {
                error::GabinatorError::newUSB("Could not open this device");
                continue;
            }
        };

        device_handle.set_active_configuration(0);
        device_handle.set_auto_detach_kernel_driver(true);

        match setup_AOA(device_handle, descriptor) {
            Ok(a) => {
                return Ok(a);
            }
            Err(_) => {
                continue;
            }
        };
    }
    Err(error::GabinatorError::newUSB("Fin"))
}

fn setup_AOA(
    device: DeviceHandle<rusb::GlobalContext>,
    descriptor: DeviceDescriptor,
) -> Result<DeviceHandle<rusb::GlobalContext>, error::GabinatorError> {
    if try_connect_to_AOA(&device, descriptor).is_err() {
        if send_AOA_protocol(&device).is_err() {
            return Err(error::GabinatorError::newUSB("Error sending AOA protocol"));
        }
        let value: Result<DeviceHandle<GlobalContext>, error::GabinatorError> =
            try_to_open_AOA_device();
        match value {
            Ok(a) => {
                return Ok(a);
            }
            Err(_) => {
                return Err(error::GabinatorError::newUSB("Did not find a AOA device"));
            }
        }
    }
    let value: Result<DeviceHandle<GlobalContext>, error::GabinatorError> =
        try_to_open_AOA_device();
    match value {
        Ok(a) => {
            return Ok(a);
        }
        Err(_) => {
            return Err(error::GabinatorError::newUSB("Could not connect"));
        }
    }
}

fn try_to_open_AOA_device() -> Result<DeviceHandle<rusb::GlobalContext>, error::GabinatorError> {
    for _i in 0..500 {
        let value = open_device_with_vid_pid(0x18d1, 0x2d00);
        match value {
            Some(a) => {
                dbg!("FOUND AT 0x2d00");
                return Ok(a);
            }
            None => error::GabinatorError::newUSB("0x18d1:0x2d00 unopenable"),
        };

        let value = open_device_with_vid_pid(0x18d1, 0x2d01);
        match value {
            Some(a) => {
                dbg!("FOUND AT 0x2d01");
                return Ok(a);
            }
            None => error::GabinatorError::newUSB("0x18d1:0x2d01 unopenable"),
        };
    }
    return Err(error::GabinatorError::newUSB(
        "This is not an AOA device (or at least one that is openable)",
    ));
}

fn try_connect_to_AOA(
    device: &DeviceHandle<rusb::GlobalContext>,
    descriptor: DeviceDescriptor,
) -> Result<&DeviceHandle<rusb::GlobalContext>, error::GabinatorError> {
    if descriptor.vendor_id() != 0x18d1 {
        return Err(error::GabinatorError::newUSB("Device is not in AOA mode"));
    }
    if descriptor.product_id() == 0x2d00 || descriptor.product_id() == 0x2d01 {
        return Ok(&device);
    }

    Err(error::GabinatorError::newUSB("Device is not an AOA device"))
}

fn send_AOA_protocol(
    device: &DeviceHandle<rusb::GlobalContext>,
) -> Result<String, error::GabinatorError> {
    //Si devuelve un error, significa que no es un dispositivo compatible con el modo accesorio (Fuente: https://source.android.com/docs/core/interaction/accessories/aoa?hl=es)
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
        ));
    }

    //Obtener la version de AOA que soporta el dispositivo
    let version_AOA = (version_buffer[1] << 7) | version_buffer[0];
    if version_AOA > 2 || version_AOA <= 0 {
        return Err(error::GabinatorError::newUSB("Device does not support AOA"));
    }
    println!("VEARSION: {}", version);

    //TEST: verificando si se puede iniciar sin tener que mandar datos de accesorio
    //send_accesory_source_data(device);

    match device.write_control(0x40, 53, 0, 0, &[0], Duration::from_millis(100)) {
        Ok(a) => println!("{}", a),
        Err(a) => {
            dbg!(a);
        }
    }
    Ok("All good".to_string())
}

fn send_accesory_source_data(
    device: &DeviceHandle<rusb::GlobalContext>,
) -> Result<String, error::GabinatorError> {
    if device
        .write_control(
            0x40,
            52,
            0,
            0,
            manufacturer.as_bytes(),
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB(
            "Could not send manufacturer data",
        ));
    }
    if device
        .write_control(
            0x40,
            52,
            0,
            1,
            modelName.as_bytes(),
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB(
            "Could not send model name data",
        ));
    }
    if device
        .write_control(
            0x40,
            52,
            0,
            2,
            description.as_bytes(),
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB(
            "Could not send description data",
        ));
    }
    if device
        .write_control(
            0x40,
            52,
            0,
            3,
            version.as_bytes(),
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB("Could not send version data"));
    }
    if device
        .write_control(0x40, 52, 0, 4, uri.as_bytes(), Duration::from_millis(100))
        .is_err()
    {
        return Err(error::GabinatorError::newUSB("Could not send uri data"));
    }
    if device
        .write_control(
            0x40,
            52,
            0,
            5,
            serialNumber.as_bytes(),
            Duration::from_millis(100),
        )
        .is_err()
    {
        return Err(error::GabinatorError::newUSB(
            "Could not send serial number data",
        ));
    }
    Ok("Perfeecto".to_string())
}

struct endpoint {
    config: u8,
    interface: u8,
    setting: u8,
    address: u8,
}

fn find_bulk_endpoint(
    device: &DeviceHandle<GlobalContext>,
    descriptor: DeviceDescriptor,
) -> Option<endpoint> {
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

pub fn capture_and_send(handler: &DeviceHandle<GlobalContext>) -> Option<rusb::Error> {
    let preparation_time = Instant::now();

    match prepare_accesory(handler) {
        Err(a) => return Some(a),
        _ => {}
    }

    let mut tries = 0;
    loop {
        let data = match capture_screen() {
            Ok(a) => a,
            Err(a) => return Some(rusb::Error::Other),
        };
        let sending_time = Instant::now();
        match send_capture_data(&data, handler) {
            Some(a) => {
                if tries == 5 {
                    return Some(a);
                }
                tries += 1;
            }
            None => continue,
        }
    }
    return None;
}

pub fn prepare_accesory(handler: &DeviceHandle<GlobalContext>) -> Result<endpoint, rusb::Error> {
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
            error::GabinatorError::newUSB("Cannot find endpoint");
            return Err(rusb::Error::NotFound);
        }
    };
    return Ok(endpoint_data);
}

pub fn send_capture_data(
    data: &Vec<u8>,
    handler: &DeviceHandle<GlobalContext>,
) -> Option<rusb::Error> {
    let descriptor = handler
        .device()
        .device_descriptor()
        .expect("No pudo obtener el descriptor");
    let endpoint_data = match find_bulk_endpoint(&handler, descriptor) {
        Some(a) => a,
        None => {
            error::GabinatorError::newUSB("Cannot find endpoint");
            return Some(rusb::Error::NotFound);
        }
    };
    let result = handler.write_bulk(endpoint_data.address, &data, Duration::from_millis(5000));
    dbg!(result);
    if result.is_err() {
        return Some(result.unwrap_err());
    }
    return None;
}
