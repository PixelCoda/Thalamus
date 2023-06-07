// ████████ ██   ██  █████  ██       █████  ███    ███ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ████  ████ ██    ██ ██      
//    ██    ███████ ███████ ██      ███████ ██ ████ ██ ██    ██ ███████ 
//    ██    ██   ██ ██   ██ ██      ██   ██ ██  ██  ██ ██    ██      ██ 
//    ██    ██   ██ ██   ██ ███████ ██   ██ ██      ██  ██████  ███████                                                                             
// Copyright 2021-2023 The Open Sam Foundation (OSF)
// Developed by Caleb Mitchell Smith (PixelCoda)
// Licensed under GPLv3....see LICENSE file.

use local_ip_address::list_afinet_netifas;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};
use simple_dns::{Name, CLASS, ResourceRecord, rdata::{RData, A, SRV}};
use simple_mdns::async_discovery::SimpleMdnsResponder;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{net::IpAddr};
use tokio::task::yield_now;
use tokio::task;
use std::thread;
use std::sync::mpsc;

extern crate rouille;

pub mod thalamus;
pub mod p2p;






/// Struct for storing all nodes connected to the client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThalamusClient {
    pub nodes: Arc<Mutex<Vec<ThalamusNode>>>,
}
impl ThalamusClient {
    pub fn new() -> ThalamusClient {
        let x: Vec<ThalamusNode> = Vec::new();
        ThalamusClient { 
            nodes: Arc::new(Mutex::new(x)),
        }
    }

    pub fn ipv4_discovery(&mut self){

        let network_interfaces = list_afinet_netifas().unwrap();

        let nodex = Arc::clone(&self.nodes);

        for (name, ip) in network_interfaces.iter() {
            if !ip.is_loopback() && !format!("{}", ip.clone()).contains(":"){
                log::warn!("{}:\t{:?}", name, ip);
                let ips = crate::thalamus::tools::netscan::scan_bulk(format!("{}", ip).as_str(), "8050", "/24").unwrap();
                log::warn!("Found {} ips", ips.len());
            
                // Check matching ips for thalamus version info
                for ipx in ips{
                    let version = fetch_version(ipx.as_str());
                    match version {
                        Ok(v) => {
                            let mut nodes = nodex.lock().unwrap();
                            let existing_index = nodes.clone().iter().position(|r| r.pid == v.pid.to_string());
                            match existing_index {
                                Some(_index) => {
                                },
                                None => {
                                    nodes.push(ThalamusNode::new(v.pid.to_string(), v.version.to_string(), ipx, 8050));
                                }
                            }
                            std::mem::drop(nodes);
                        },
                        Err(e) => {
                            log::error!("fetch_thalamus_version_error: {}", e);
                        }
                    }
                }
                
            }
           
        }


        
    }

    pub async fn mdns_discovery(&mut self, discovery: simple_mdns::async_discovery::ServiceDiscovery) -> Result<simple_mdns::async_discovery::ServiceDiscovery, std::io::Error> {
        let nodex = Arc::clone(&self.nodes);

        let services = discovery.get_known_services().await;
        if services.len() > 0 {
            for xy in services{
                log::info!("vhhjv: {:?}", xy);
                // TODO: Register 
                for ipfx in xy.ip_addresses{
                    let ipx = ipfx.to_string();
                    let port = xy.ports[0];
                    if !ipx.to_string().contains(".0.1"){
                        let version = async_fetch_version(format!("{}:{}", ipx, port).as_str()).await;
                        match version {
                            Ok(v) => {
                                let mut nodes = nodex.lock().unwrap();
                                let existing_index = nodes.clone().iter().position(|r| r.pid == v.pid.to_string());
                                match existing_index {
                                    Some(index) => {
                                        nodes[index].is_online = true;
                                        nodes[index].last_ping = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                                        std::mem::drop(nodes);
                                        self.save();
                                    },
                                    None => {
                                        nodes.push(ThalamusNode::new(v.pid.to_string(), v.version.to_string(), format!("{}:{}", ipx, port), 8050));
                                        std::mem::drop(nodes);
                                        self.save();
                                    }
                                }
                                
                            },
                            Err(e) => {
                                log::error!("fetch_thalamus_version_error: {}", e);
                                let mut nodes = nodex.lock().unwrap();
                                let existing_index = nodes.clone().iter().position(|r| r.ip_address == format!("{}:{}", ipx, port).as_str());
                                match existing_index {
                                    Some(index) => {
                                        nodes[index].is_online = false;
                                        std::mem::drop(nodes);
                                        self.save();
                                    },
                                    None => {
                                        std::mem::drop(nodes);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(discovery)
    }

    pub async fn start_mdns_responder(&mut self){
        let network_interfaces = list_afinet_netifas().unwrap();
        task::spawn(async move{
            
    
            let mut responder = SimpleMdnsResponder::new(10);
            let srv_name = Name::new_unchecked("_thalamus._tcp.local");
        
            for (_name, ip) in network_interfaces.iter() {
                if !ip.is_loopback() && !format!("{}", ip.clone()).contains(":") && !format!("{}", ip.clone()).contains(".0.1"){
                    match *ip {
                        IpAddr::V4(ipv4) => { 
                            responder.add_resource(ResourceRecord::new(
                                srv_name.clone(),
                                CLASS::IN,
                                10,
                                RData::A(A { address: ipv4.into() }),
                            )).await;
                         },
                        IpAddr::V6(_ipv6) => { /* handle IPv6 */ }
                    }
    
                    
                }
            }
        
            responder.add_resource(ResourceRecord::new(
                srv_name.clone(),
                CLASS::IN,
                10,
                RData::SRV(SRV {
                    port: 8050,
                    priority: 0,
                    weight: 0,
                    target: srv_name
                })
            )).await;
    
            yield_now().await;
            
        });
    }
    
    pub async fn nodex_discovery(&mut self){
        let nodell = self.nodes.lock().unwrap();
        let nodess = nodell.clone();
        std::mem::drop(nodell);
        for node in nodess{
            let nodexs_wrap = node.nodex();
            match nodexs_wrap {
                Ok(nodexs) => {
                    for nodex in nodexs{
                        let mut nodes = self.nodes.lock().unwrap();
                        let existing_index = nodes.clone().iter().position(|r| r.pid == nodex.pid.to_string());
                        match existing_index {
                            Some(index) => {
                                nodes[index].last_ping = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
                                std::mem::drop(nodes);
                                self.save();
                            },
                            None => {
                                nodes.push(nodex);
                                std::mem::drop(nodes);
                                self.save();
                            }
                        }
                    }
                },
                Err(e) => {
                    log::error!("nodex_discovery_error: {}", e);
                }
            }

        }
    }

    pub fn save(&self){
        std::fs::File::create("/opt/thalamusc/clients.json").expect("create failed");
        let j = serde_json::to_string(&self).unwrap();
        std::fs::write("/opt/thalamusc/clients.json", j).expect("Unable to write file");
    }

    pub fn load() -> Result<ThalamusClient, Box<dyn Error>>{
        let save_file = std::fs::read_to_string("/opt/thalamusc/clients.json");
        match save_file {
            Ok(save_data) => {
                let v: Result<ThalamusClient, _> = serde_json::from_str(&save_data);
                match v {
                    Ok(v2) => {
                        return Ok(v2);
                    },
                    Err(e) => {
                        log::error!("{}", format!("Unable to read save file: {}", e));
                        let new_c = ThalamusClient::new();
                        new_c.save();
                        return Ok(new_c);
                    }
                }
                
            },
            Err(e) => {
                log::error!("{}", format!("Unable to read save file: {}", e));
                let new_c = ThalamusClient::new();
                new_c.save();
                return Ok(new_c);
            }
        }
    }

    // pub fn select_optimal_node(&self, node_type: String) -> Result<ThalamusNode, Box<dyn Error + '_>> {
    //     let nodex = self.nodes.lock()?;
    //     let nodes = nodex.clone();
    //     std::mem::drop(nodex);

    //     let mut fastest_whisper_stt_score = 9999999;
    //     let mut fastest_whisper_vwav_score = 9999999;
    //     let mut fastest_srgan_score = 9999999;
    //     let mut fastest_llama_score = 9999999;
    //     let mut selected_node = nodes[0].clone();
    //     for node in nodes {
    //         let stats = node.stats.clone();
    //         if stats.whisper_stt_score < Some(fastest_whisper_stt_score) && node_type.contains("stt") {
    //             fastest_whisper_stt_score = stats.whisper_stt_score;
    //             selected_node = node.clone();
    //         }
    //         if stats.whisper_vwav_score < fastest_whisper_vwav_score && node_type.contains("vwav") {
    //             fastest_whisper_vwav_score = stats.whisper_vwav_score;
    //             selected_node = node.clone();
    //         }
    //         if stats.srgan < fastest_srgan_score && node_type.contains("srgan") {
    //             fastest_srgan_score = stats.srgan;
    //             selected_node = node.clone();
    //         }
    //         if stats.llama_score < fastest_llama_score && node_type.contains("llama") {
    //             fastest_llama_score = stats.llama_score;
    //             selected_node = node.clone();
    //         }
    //     }
        
    //     return Ok(selected_node);
    // }
}



pub fn fetch_version(host: &str) -> Result<VersionReply, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder().build()?;
    return Ok(client.get(format!("http://{}/api/thalamus/version", host)).send()?.json()?);
}

pub async fn async_fetch_version(host: &str) -> Result<VersionReply, Box<dyn Error>> {
    let client = reqwest::Client::builder().build()?;
    return Ok(client.get(format!("http://{}/api/thalamus/version", host)).send().await?.json().await?);
}





/// Struct for storing node information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThalamusNode {
    pub pid: String,
    pub ip_address: String, // unique
    pub version: String,
    pub port: u16,
    pub jobs: Vec<ThalamusNodeJob>,
    pub last_ping: i64,
    pub stats: ThalamusNodeStats,
    pub is_online: bool,
}
impl ThalamusNode {
    pub fn new(pid: String, version: String, ip_address: String, port: u16) -> ThalamusNode {
        let jobs: Vec<ThalamusNodeJob> = Vec::new();
        let mut node = ThalamusNode { 
            pid: pid,
            ip_address: ip_address,
            jobs: jobs,
            version: version,
            port: port,
            last_ping: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            stats: ThalamusNodeStats::new(),
            is_online: true,
        };
        let stats = ThalamusNodeStats::calculate(node.clone());
        node.stats = stats;
        return node;
    }

    pub fn whisper_stt_tiny(&self, tmp_file_path: String) -> Result<STTReply, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "tiny").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        return Ok(client.post(format!("http://{}/api/services/whisper", self.ip_address.clone()))
        .multipart(form)
        .send()?.json()?);
    }

    pub fn whisper_stt_base(&self, tmp_file_path: String) -> Result<STTReply, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "basic").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        return Ok(client.post(format!("http://{}/api/services/whisper", self.ip_address.clone()))
        .multipart(form)
        .send()?.json()?);
    }

    pub fn whisper_stt_medium(&self, tmp_file_path: String) -> Result<STTReply, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "medium").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        return Ok(client.post(format!("http://{}/api/services/whisper", self.ip_address.clone()))
        .multipart(form)
        .send()?.json()?);
    }

    pub fn whisper_stt_large(&self, tmp_file_path: String) -> Result<STTReply, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "large").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        return Ok(client.post(format!("http://{}/api/services/whisper", self.ip_address.clone()))
        .multipart(form)
        .send()?.json()?);
    }

    pub fn whisper_vwav_tiny(&self, tmp_file_path: String) -> Result<Vec<u8>, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "tiny").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/whisper/vwav", self.ip_address.clone()))
        .multipart(form)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn whisper_vwav_base(&self, tmp_file_path: String) -> Result<Vec<u8>, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "base").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/whisper/vwav", self.ip_address.clone()))
        .multipart(form)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn whisper_vwav_medium(&self, tmp_file_path: String) -> Result<Vec<u8>, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "medium").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/whisper/vwav", self.ip_address.clone()))
        .multipart(form)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn whisper_vwav_large(&self, tmp_file_path: String) -> Result<Vec<u8>, Box<dyn Error>>{
        let form = reqwest::blocking::multipart::Form::new().text("method", "large").file("speech", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/whisper/vwav", self.ip_address.clone()))
        .multipart(form)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn srgan(&self, tmp_file_path: String) -> Result<Vec<u8>, Box<dyn Error>>{

        let parts: Vec<&str> = tmp_file_path.split('.').collect();

        let extension = parts[parts.len() - 1];

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;

        let new_file_name = format!("{}.{}", timestamp, extension);

        let form = reqwest::blocking::multipart::Form::new().text("filename", new_file_name).file("input_file", tmp_file_path.as_str())?;

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/image/srgan", self.ip_address.clone()))
        .multipart(form)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn llama(&self, prompt: String, model: String) -> Result<String, Box<dyn Error>>{
        let params = [("model", model.as_str()), ("prompt", prompt.as_str())];

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/llama", self.ip_address.clone()))
        .form(&params)
        .send()?.text()?;

        return Ok(bytes.to_string());
    }

    pub fn tts(&self, prompt: String) -> Result<Vec<u8>, Box<dyn Error>>{
        let params = [("prompt", prompt.as_str())];

        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let bytes = client.post(format!("http://{}/api/services/tts", self.ip_address.clone()))
        .form(&params)
        .send()?.bytes()?;

        return Ok(bytes.to_vec());
    }

    pub fn nodex(&self) -> Result<Vec<ThalamusNode>, Box<dyn Error>>{
        let client = reqwest::blocking::Client::builder().timeout(None).build()?;

        let mut url = format!("http://{}/api/nodex", self.ip_address.clone());
        if !url.contains(":") {
            url = format!("{}:{}", url, self.port.clone());
        }

        return Ok(client.get(url)
        .send()?.json()?);
    }

    pub fn test_llama_7b(&self) -> Result<std::option::Option<i64>, std::sync::mpsc::RecvTimeoutError>{
        log::info!("{}: Running LLAMA 7B test...", self.pid);
        let (sender, receiver) = mpsc::channel();
        let node_c = self.clone();
        let t = thread::spawn(move || {

        
            let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
            let _llama = node_c.llama("Tell me about Abraham Lincoln.".to_string(), "7B".to_string()).unwrap();
            let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
            let time_elapsed = Some(end_timestamp - start_timestamp);

            match sender.send(time_elapsed) {
                Ok(()) => {}, // everything good
                Err(_) => {}, // we have been released, don't panic
            }
        });
        return receiver.recv_timeout(std::time::Duration::from_millis(60000));
    }

    pub fn test_llama_13b(&self) -> Result<std::option::Option<i64>, std::sync::mpsc::RecvTimeoutError>{
        log::info!("{}: Running LLAMA 13B test...", self.pid);
        let (sender, receiver) = mpsc::channel();
        let node_c = self.clone();
        let t = thread::spawn(move || {

        
            let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
            let _llama = node_c.llama("Tell me about Abraham Lincoln.".to_string(), "13B".to_string()).unwrap();
            let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
            let time_elapsed = Some(end_timestamp - start_timestamp);

            match sender.send(time_elapsed) {
                Ok(()) => {}, // everything good
                Err(_) => {}, // we have been released, don't panic
            }
        });
        return receiver.recv_timeout(std::time::Duration::from_millis(60000));
    }
}

/// Struct for storing the jobs of each node
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThalamusNodeJob {
    pub oid: String,
    pub url: String,
    pub started_at: i64,
}
impl ThalamusNodeJob {
    pub fn new() -> ThalamusNodeJob {
        let oid: String = thread_rng().sample_iter(&Alphanumeric).take(15).map(char::from).collect();
        ThalamusNodeJob { 
            oid: oid,
            url: String::new(),
            started_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
        }
    }
}

/// Struct for storing the stats of each node
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThalamusNodeStats {
    pub apple_tts: Option<i64>,
    pub bark_tts: Option<i64>,
    pub deepspeech_tts: Option<i64>,
    pub espeak_tts: Option<i64>,
    pub watson_tts: Option<i64>,
    pub tts_score: Option<i64>,
    pub llama_7b: Option<i64>,
    pub llama_13b: Option<i64>,
    pub llama_30b: Option<i64>,
    pub llama_65b: Option<i64>,
    pub llama_score: Option<i64>,
    pub nst_score: Option<i64>,
    pub srgan_score: Option<i64>,
    pub whisper_stt_tiny: Option<i64>,
    pub whisper_stt_base: Option<i64>,
    pub whisper_stt_medium: Option<i64>,
    pub whisper_stt_large: Option<i64>,
    pub whisper_stt_score: Option<i64>,
    pub whisper_vwav_tiny: Option<i64>,
    pub whisper_vwav_base: Option<i64>,
    pub whisper_vwav_medium: Option<i64>,
    pub whisper_vwav_large: Option<i64>,
    pub whisper_vwav_score: Option<i64>,
}
impl ThalamusNodeStats {
    pub fn new() -> ThalamusNodeStats {
        ThalamusNodeStats { 
            whisper_stt_tiny: None,
            whisper_stt_base: None,
            whisper_stt_medium: None,
            whisper_stt_large: None,
            whisper_stt_score: None,
            llama_7b: None,
            llama_13b: None,
            llama_30b: None,
            llama_65b: None,
            llama_score: None,
            whisper_vwav_tiny: None,
            whisper_vwav_base: None,
            whisper_vwav_medium: None,
            whisper_vwav_large: None,
            whisper_vwav_score: None, 
            srgan_score: None,
            espeak_tts: None,
            apple_tts: None,
            bark_tts: None,
            watson_tts: None,
            deepspeech_tts: None,
            tts_score: None,
            nst_score: None
        }
    }

    pub fn calculate(node: ThalamusNode) -> ThalamusNodeStats {

        log::info!("Calculating stats for node {}.....", node.pid);

        // Test STT Tiny
        log::info!("{}: Running STT Tiny test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_stt_tiny("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let tiny_stt = Some(end_timestamp - start_timestamp);
        log::info!("{}: STT Tiny test complete in {:?} miliseconds", node.pid, tiny_stt);
        
        // Test STT Base
        log::info!("{}: Running STT Base test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_stt_base("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let basic_stt = Some(end_timestamp - start_timestamp);
        log::info!("{}: STT Base test complete in {:?} miliseconds", node.pid, basic_stt);
        
        // Test STT Medium
        log::info!("{}: Running STT Medium test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_stt_medium("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let medium_stt = Some(end_timestamp - start_timestamp);
        log::info!("{}: STT Medium test complete in {:?} miliseconds", node.pid, medium_stt);

        // Test STT Large
        log::info!("{}: Running STT Large test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_stt_large("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let large_stt = Some(end_timestamp - start_timestamp);
        log::info!("{}: STT Large test complete in {:?} miliseconds", node.pid, large_stt);
        
        // Calculate average STT score
        // let whisper_stt_score = (tiny_stt + basic_stt + medium_stt + large_stt) / 4;

        // Test VWAV Tiny
        log::info!("{}: Running VWAV Tiny test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_vwav_tiny("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let whisper_vwav_tiny = Some(end_timestamp - start_timestamp);
        log::info!("{}: VWAV Tiny test complete in {:?} miliseconds", node.pid, whisper_vwav_tiny);
        
        // Test VWAV Base
        log::info!("{}: Running VWAV Base test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_vwav_base("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let whisper_vwav_base = Some(end_timestamp - start_timestamp);
        log::info!("{}: VWAV Base test complete in {:?} miliseconds", node.pid, whisper_vwav_base);
        
        // Test VWAV Medium
        log::info!("{}: Running VWAV Medium test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_vwav_medium("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let whisper_vwav_medium = Some(end_timestamp - start_timestamp);
        log::info!("{}: VWAV Medium test complete in {:?} miliseconds", node.pid, whisper_vwav_medium);

        // Test VWAV Large
        log::info!("{}: Running VWAV Large test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.whisper_vwav_large("/opt/thalamusc/test.wav".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let whisper_vwav_large = Some(end_timestamp - start_timestamp);
        log::info!("{}: VWAV Large test complete in {:?} miliseconds", node.pid, whisper_vwav_large);

        // Calculate average VWAV score
        // let whisper_vwav_score = (whisper_vwav_tiny + whisper_vwav_base + whisper_vwav_medium + whisper_vwav_large) / 4;

        // Test LLAMA 7B
        let mut llama_7b: Option<i64> = None;
        let llama_7b_test = node.test_llama_7b();
        match llama_7b_test {
            Ok(time_elapsed) => {
                llama_7b = time_elapsed;
            },
            Err(e) => {
                log::error!("{}: Error running Llama 7B test: {:?}", node.pid, e);
            }
        }
        log::info!("{}: LLAMA 7B test complete in {:?} miliseconds", node.pid, llama_7b);

        // Test LLAMA 13B
        let mut llama_13b: Option<i64> = None;
        let llama_13b_test = node.test_llama_13b();
        match llama_13b_test {
            Ok(time_elapsed) => {
                llama_13b = time_elapsed;
            },
            Err(e) => {
                log::error!("{}: Error running Llama 13B test: {:?}", node.pid, e);
            }
        }
        log::info!("{}: LLAMA 13B test complete in {:?} miliseconds", node.pid, llama_13b);

        // Test LLAMA 30B
        log::info!("{}: Running LLAMA 30B test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _llama = node.llama("Tell me about Abraham Lincoln.".to_string(), "30B".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let llama_30b = Some(end_timestamp - start_timestamp);
        log::info!("{}: LLAMA 30B test complete in {:?} miliseconds", node.pid, llama_7b);

        // Test LLAMA 65B
        log::info!("{}: Running LLAMA 65B test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _llama = node.llama("Tell me about Abraham Lincoln.".to_string(), "65B".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let llama_65b = Some(end_timestamp - start_timestamp);
        log::info!("{}: LLAMA 65B test complete in {:?} miliseconds", node.pid, llama_7b);

        // Calculate average llama score
        // let llama_score = (llama_7b + 0 + llama_30b + llama_65b) / 4;

        // Test SRGAN
        log::info!("{}: Running SRGAN test...", node.pid);
        let start_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let _stt = node.srgan("/opt/thalamusc/test.jpg".to_string()).unwrap();
        let end_timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64;
        let srgan = Some(end_timestamp - start_timestamp);
        log::warn!("{}: SRGAN test complete in {:?} miliseconds", node.pid, srgan);

        // Return stats
        return ThalamusNodeStats { 
            whisper_stt_tiny: tiny_stt,
            whisper_stt_base: basic_stt,
            whisper_stt_medium: medium_stt,
            whisper_stt_large: large_stt,
            whisper_stt_score: None,
            llama_7b: llama_7b,
            llama_13b: None,
            llama_30b: llama_30b,
            llama_65b: llama_65b,
            llama_score: None,
            whisper_vwav_tiny: whisper_vwav_tiny,
            whisper_vwav_base: whisper_vwav_base,
            whisper_vwav_medium: whisper_vwav_medium,
            whisper_vwav_large: whisper_vwav_large,
            whisper_vwav_score: None, 
            srgan_score: srgan,
            espeak_tts: None,
            apple_tts: None,
            bark_tts: None,
            watson_tts: None,
            deepspeech_tts: None,
            tts_score: None,
            nst_score: None
        };
    }
}

/// Auxilary Struct for API Version replies
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionReply {
    pub version: String,
    pub pid: String,
}

/// Auxilary Struct for API STT replies
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct STTReply {
    pub text: String,
    pub time: f64,
    pub response_type: Option<String>,
}