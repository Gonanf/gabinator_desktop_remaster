use std::time::Instant;
mod capture;
mod usb;
pub mod error;
use rusb::DeviceHandle;
use usb::{ capture_and_send, find_compatible_usb, prepare_accesory, send_capture_data };
use capture::capture_screen;

fn main() {
    let mut found = false;
    let mut device: Option<rusb::DeviceHandle<rusb::GlobalContext>> = None;
    
    //TEST
    //capture_screen();
    loop {
        let mut tries = 0;
        match find_compatible_usb() {
            Ok(a) => {
                capture_and_send(&a);
            }
            Err(_) => {
                continue;
            }
        }
    }
}
