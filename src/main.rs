use std::time::Instant;
mod capture;
mod usb;
pub mod error;
use rusb::DeviceHandle;
use usb::{ find_compatible_usb, send_capture_data };
use capture::capture_screen;

fn main() {
    let mut found = false;
    let mut device: Option<rusb::DeviceHandle<rusb::GlobalContext>> = None;
    loop {
        match find_compatible_usb() {
            Ok(a) => {
                match capture_screen() {
                    Ok(b) => send_capture_data(b, &a),
                    Err(_) => {
                        continue;
                    }
                };
            }
            Err(_) => {
                continue;
            }
        }

    }
}
