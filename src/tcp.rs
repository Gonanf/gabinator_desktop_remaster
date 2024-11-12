use image::{ImageBuffer, RgbaImage};
use std::collections::HashMap;
use std::fs::File;
use turbojpeg::Image;

use crate::Logger;
use crate::{capture::capture_screen, error::GabinatorError};
use std::io::Read;
use std::net::TcpStream;
use std::{io::Write, net::TcpListener};

pub fn start_server( test_server: bool, test_data: bool) {
    let config = Logger::get_config_content();
    let socket = TcpListener::bind("0.0.0.0:3000").unwrap();
    let ip = local_ip_address::local_ip().unwrap();
    println!("SERVER IP -> {:?}", ip);
    for st in socket.incoming() {
        println!("Conectado");
        let mut client = st.unwrap();
        let mut tries = 0;
        if test_server{
            if test_data{
                if client.write_all(&[10,8,8,8,8,8,8,b'\n']).is_err() {
                    GabinatorError::newCapture(
                        format!("Error sending test data"),
                        crate::error::LoggerLevel::Error,
                        Some(config.clone()));
                    }
            }
            else{send_image_data(config.clone(),&mut client);}
        }
        else{
            loop {
                if test_data{
                    if client.write_all(&[10,8,8,8,8,8,8,b'\n']).is_err() {
                        GabinatorError::newCapture(
                            format!("Error sending test data"),
                            crate::error::LoggerLevel::Error,
                            Some(config.clone()));
                        }
                }
                else{
                    let value = send_image_data(config.clone(),&mut client);
                if value.is_some() {
                    tries += 1;
                    println!("Package failed, {} more tries remaining", 5 - tries);
                    if tries >= 5 {
                        println!("Connection with too many errors");
                        break;
                    }
                } else {
                    tries = 0;
                }
                }
            }
        }
        
    }
}

fn send_image_data(config: HashMap<String,String>,  client: &mut TcpStream) -> Option<GabinatorError> {
    let mut data = capture_screen().unwrap();
    if client.write_all(&data).is_err() {
        return Some(GabinatorError::newCapture(format!("Error writing data into client"), crate::error::LoggerLevel::Error, Some(config.clone())))
    }
    if client.flush().is_err() {
        return Some(GabinatorError::newCapture(
            format!("Error flushing server"),
            crate::error::LoggerLevel::Error,
            Some(config.clone()),
        ));
    }
    None
}

pub fn test_server() {
    let config = Logger::get_config_content();
    let mut server = TcpStream::connect("0.0.0.0:3000").unwrap();
    let mut total_buffer = File::create("amongas.jpg").unwrap();
    let mut total_bytes = 0;
    let mut iteration = 0;
    loop {
        let mut buffer = [0; 1024];
        let bytes = server.read(&mut buffer).unwrap();
        total_buffer.write_all(&buffer);
        total_bytes += bytes;
        dbg!(&bytes);
        if bytes < 1024 {
            let mut test = String::new();
            dbg!(&total_bytes);
            total_bytes = 0;
            iteration += 1;
            total_buffer = File::create(format!("amongas{iteration}.jpg")).unwrap();
        }
    }
}
