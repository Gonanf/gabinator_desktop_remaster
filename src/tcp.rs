use std::{io::Write, net::TcpListener};

use crate::capture::capture_screen;

pub fn start_server(){
    let socket = TcpListener::bind("0.0.0.0:3000").unwrap();
    let ip = local_ip_address::local_ip().unwrap();
    println!("SERVER IP -> {:?}",ip);
    for st in socket.incoming(){
        println!("Conectado");
        let mut client = st.unwrap();
        let mut tries = 0;
        loop {
            if client.write_all(&capture_screen(false).unwrap()).is_err(){
                tries+=1;
                println!("Package failed, {} more tries remaining",5-tries);
                if tries >= 5{
                    println!("Connection with too many errors");
                    break;
                }
            }
            else{
                tries = 0;
            }
        }
    }
}

