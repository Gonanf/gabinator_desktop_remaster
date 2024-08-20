use std::{io::{stdin, Read}, time::Instant};
mod capture;
mod usb;
pub mod error;
use rusb::DeviceHandle;
use usb::{ capture_and_send, find_compatible_usb, prepare_accesory, send_capture_data };
use capture::capture_screen;

fn main() {
    capture_screen();
    use std::time::Instant;
    //TEST
    //capture_screen();
    loop {
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
