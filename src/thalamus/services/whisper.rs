// ████████ ██   ██  █████  ██       █████  ███    ███ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ████  ████ ██    ██ ██      
//    ██    ███████ ███████ ██      ███████ ██ ████ ██ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ██  ██  ██ ██    ██      ██ 
//    ██    ██   ██ ██   ██ ███████ ██   ██ ██      ██  ██████  ███████                                                                             
// Copyright 2021-2023 The Open thalamus Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use rouille::Request;
use rouille::Response;
use rouille::input::post::BufferedFile;
use rouille::post_input;
use serde::{Serialize, Deserialize};

use std::path::Path;
use std::fs::File;

use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;

use std::time::Duration;
use std::io::Write;




// /opt/thalamus/bin/whisper -m /opt/thalamus/models/ggml-* -f ./output.wav -otxt
pub fn whisper(file_path: String, method: &str) -> Result<String, crate::thalamus::services::Error> {

    // Force all input to become wav@16khz
    match crate::thalamus::tools::wav_to_16000(file_path.clone()){
        Ok(_) => (),
        Err(e) => return Err(crate::thalamus::services::Error::from(e))
    };

    // Execute Whisper
    match method {
        "tiny" => log::warn!("{}", crate::thalamus::tools::whisper("tiny", file_path.as_str())?),
        "base" => log::warn!("{}", crate::thalamus::tools::whisper("base", file_path.as_str())?),
        "medium" => log::warn!("{}", crate::thalamus::tools::whisper("medium", file_path.as_str())?),
        "large" => log::warn!("{}", crate::thalamus::tools::whisper("large", file_path.as_str())?),
        &_ => log::warn!("{}", crate::thalamus::tools::whisper("tiny", file_path.as_str())?)
    };
    
    // Copy the results to memory
    let data = std::fs::read_to_string(format!("{}.16.wav.txt", file_path).as_str())?;

    // Cleanup
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(60000));
        crate::thalamus::tools::rm(format!("{}*", file_path).as_str()).unwrap();
    });

    // Return the results
    return Ok(data);
}

// Patch linux whisper WTS files
pub fn patch_whisper_wts(file_path: String) -> Result<(), crate::thalamus::services::Error>{
    let mut data = std::fs::read_to_string(format!("{}", file_path).as_str())?;
    data = data.replace("ffmpeg", "/opt/thalamus/bin/ffmpeg").replace("/System/Library/Fonts/Supplemental/Courier New Bold.ttf","/opt/thalamus/fonts/courier.ttf");
    std::fs::remove_file(format!("{}", file_path).as_str())?;
    std::fs::write(file_path, data)?;
    return Ok(());
}


// TODO: Compile whisper for raspi and patch installer
pub fn install() -> std::io::Result<()> {

    if !Path::new("/opt/thalamus/models/ggml-tiny.bin").exists(){
        log::warn!("ggml-tiny.bin is missing.....downloading it from https://huggingface.co/");
        match crate::thalamus::tools::download("/opt/thalamus/models/ggml-tiny.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"){
            Ok(_) => {
                log::info!("Stored model ggml-tiny.bin in /opt/thalamus/models/");
            },
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to download tiny whisper model"))
        }
    }

    if !Path::new("/opt/thalamus/models/ggml-base.bin").exists(){
        log::warn!("ggml-base.bin is missing.....downloading it from https://huggingface.co/");
        match crate::thalamus::tools::download("/opt/thalamus/models/ggml-base.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"){
            Ok(_) => {
                log::info!("Stored model ggml-base.bin in /opt/thalamus/models/");
            },
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to download base whisper model"))
        }
    }

    if !Path::new("/opt/thalamus/models/ggml-medium.bin").exists(){
        log::warn!("ggml-medium.bin is missing.....downloading it from https://huggingface.co/");
        match crate::thalamus::tools::download("/opt/thalamus/models/ggml-medium.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin"){
            Ok(_) => {
                log::info!("Stored model ggml-medium.bin in /opt/thalamus/models/");
            },
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to download medium whisper model"))
        }
    }

    if !Path::new("/opt/thalamus/models/ggml-large.bin").exists(){
        log::warn!("ggml-large.bin is missing.....downloading it from https://huggingface.co/");
        match crate::thalamus::tools::download("/opt/thalamus/models/ggml-large.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large.bin"){
            Ok(_) => {
                log::info!("Stored model ggml-large.bin in /opt/thalamus/models/");
            },
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to download large whisper model"))
        }
    }

    #[cfg(target_arch = "x86_64")]{
        log::info!("Installing whisper (x86_64) /opt/thalamus/bin/whisper");
        let data = include_bytes!("../../../packages/whisper/main-amd64");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/bin/whisper")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }

        log::info!("Unpacking models.zip...");
        let data = include_bytes!("../../../packages/whisper/models.zip");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/models/models.zip")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }
    
        crate::thalamus::tools::extract_zip("/opt/thalamus/models/models.zip", format!("/opt/thalamus/models/"));
        match crate::thalamus::tools::rmd("/opt/thalamus/models/models.zip"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to remove models.zip"))
        }
        match crate::thalamus::tools::mv("/opt/thalamus/models/models/*", "/opt/thalamus/models/"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to move model data"))
        }
        match crate::thalamus::tools::rmd("/opt/thalamus/models/models/"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to cleanup models data"))
        }


        // Install ffmpeg
        let data = include_bytes!("../../../packages/ffmpeg/amd64/ffmpeg");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/bin/ffmpeg")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }
    
        match crate::thalamus::tools::mark_as_executable("/opt/thalamus/bin/ffmpeg"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to chmod ffmpeg"))
        }



    }

    // Apple M1/M2
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))] {
        log::info!("Installing whisper (aarch64) /opt/thalamus/bin");
        let data = include_bytes!("../../../packages/whisper/apple/main");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/bin/whisper")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }

        log::info!("Unpacking models.zip...");
        let data = include_bytes!("../../../packages/whisper/models.zip");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/models/models.zip")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }

        log::info!("Unpacking coreml.sh...");
        let data = include_bytes!("../../../packages/whisper/coreml.sh");
        let mut pos = 0;
        let mut buffer = File::create("/opt/thalamus/models/coreml.sh")?;
        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }
    
        crate::thalamus::tools::extract_zip("/opt/thalamus/models/models.zip", format!("/opt/thalamus/models/"));
        match crate::thalamus::tools::rmd("/opt/thalamus/models/models.zip"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to remove models.zip"))
        }
        match crate::thalamus::tools::mv("/opt/thalamus/models/models/*", "/opt/thalamus/models/"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to move model data"))
        }
        match crate::thalamus::tools::rmd("/opt/thalamus/models/models/"){
            Ok(_) => (),
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to cleanup models data"))
        }

        // Fix permissions
        match crate::thalamus::tools::fix_permissions("/opt/thalamus/models"){
            Ok(_) => {},
            Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to chmod /opt/thalamus")),
        }
        match crate::thalamus::tools::mark_as_executable("/opt/thalamus/models/coreml.sh"){
            Ok(_) => {},
            Err(_) => {},
        }

        match crate::thalamus::tools::mark_as_executable("/opt/thalamus/models/generate-coreml-model.sh"){
            Ok(_) => {},
            Err(_) => {},
        }

        // Configure Miniconda and Generate ML models if necessary
        if !Path::new("/opt/thalamus/models/coreml-encoder-tiny.mlpackage").exists() || !Path::new("/opt/thalamus/models/coreml-encoder-large.mlpackage").exists(){
            log::warn!("CoreML Encoders are missing...please be patient while they are being generated. This may take a while. Future launches will be faster.");
            match crate::thalamus::tools::sh("/opt/thalamus/models/coreml.sh"){
                Ok(_) => {},
                Err(_) => {},
            }  
            log::warn!("CoreML encoders have been generated. Please check the log for more information.");  
        }
    }

    let data = include_bytes!("../../../fonts/courier.ttf");
    let mut pos = 0;
    let mut buffer = File::create("/opt/thalamus/fonts/courier.ttf")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }


    
    match crate::thalamus::tools::mark_as_executable("/opt/thalamus/bin/whisper"){
        Ok(_) => (),
        Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to chmod whisper"))
    }

    Ok(())
}




#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct STTReply {
    pub text: String,
    pub time: f64,
    pub response_type: Option<String>,
}



pub fn handle(request: &Request) -> Result<Response, crate::thalamus::http::Error> {
    
   

    
    if request.url() == "/api/services/whisper" {

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let input = post_input!(request, {
            speech: BufferedFile,
            method: String
        })?;

        let tmp_file_path = format!("/opt/thalamus/tmp/{}.wav", timestamp.clone());
        let mut file = File::create(tmp_file_path.clone())?;
        file.write_all(&input.speech.data)?;

        let stt = whisper(tmp_file_path, input.method.as_str())?;

        let reply = STTReply{
            text: stt,
            time: timestamp as f64,
            response_type: None
        };

        log::info!("{}", reply.text.clone());

        return Ok(Response::json(&reply));
      
    }



    
    return Ok(Response::empty_404());
}