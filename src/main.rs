use std::{env, io::{stdin, Read}, thread, time::Instant};
mod capture;
mod usb;
pub mod error;
use rusb::DeviceHandle;
use usb::{ capture_and_send, find_compatible_usb, prepare_accesory, send_capture_data };
use capture::capture_screen;
mod tcp;
fn main() {
    //TEST
    //capture_screen();
    let args: Vec<String> = env::args().collect();
    if args.len() == 1{
        USB();
        return;
    }
    match args[1].as_str() {
        "TCP" => TCP(),
        "USB" => USB(),
        &_ => USB(),
    }
}

fn TCP(){
    tcp::start_server();
}


fn USB(){
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