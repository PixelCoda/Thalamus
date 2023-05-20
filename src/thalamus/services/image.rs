// ████████ ██   ██  █████  ██       █████  ███    ███ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ████  ████ ██    ██ ██      
//    ██    ███████ ███████ ██      ███████ ██ ████ ██ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ██  ██  ██ ██    ██      ██ 
//    ██    ██   ██ ██   ██ ███████ ██   ██ ██      ██  ██████  ███████                                                                             
// Copyright 2021-2023 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.
pub mod nst;
pub mod srgan;






use rouille::Request;
use rouille::Response;



pub fn install() -> std::io::Result<()> {
    match nst::install(){
        Ok(_) => {
            log::info!("NST installed successfully");
        },
        Err(e) => {
            log::error!("Failed to install NST: {}", e);
        }
    }

    Ok(())
}


pub fn handle(_request: &Request) -> Result<Response, crate::thalamus::http::Error> {
    return Ok(Response::empty_404());
}