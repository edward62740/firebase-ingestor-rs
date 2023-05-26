extern crate lru;

use ::std::collections::HashMap;
use coap::Server;
use coap_lite::{MessageClass, RequestType as Method};
use firebase_rs::*;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, Ipv4Addr};
use tokio::task;

use std::time::{SystemTime, UNIX_EPOCH};

mod dnssd;
mod util;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
struct CoapMessage {
    data: HashMap<String, String>,
    ipv4: Option<String>,
    unix: Option<u64>,
}

#[tokio::main]
#[allow(unused_must_use)] // do not do anything if firebase side has errors
async fn main() {
    env_logger::init();

    let regex_pattern_get = r"^/([^/]+/)*([^/]+)?/$";
    let regex_pattern_all = r"^/([^/]+/)*[^/]+$";

    println!(
        "System time: {:?}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );
    let args: Vec<String> = env::args().collect();

    let mut metadata: bool = true;
    let mut structure: bool = true;
    for arg in args.iter() {
        match arg.as_str() {
            "--no_metadata" => {
                println!("Metadata disabled");
                metadata = false},
            _ => {}
        }
    }
    for arg in args.iter() {
        match arg.as_str() {
            "--no_struct" => {
                println!("Structure disabled");
                structure = false},
            _ => {}
        }
    }
    let service_type = format!("{}.local.", get_argument(&args, 1));
    let instance_name = get_argument(&args, 2);
    let firebase_url = get_argument(&args, 3);
    let firebase_key = get_argument(&args, 4);

    let self_ip: Option<Ipv4Addr> = match local_ip().unwrap() {
        IpAddr::V4(ipv4) => Some(ipv4),
        _ => None,
    };

    let self_ipv4 = self_ip.expect("No IPv4 address found!");

    task::spawn(async move {
        let mdns: ServiceDaemon = ServiceDaemon::new().expect("Could not create service daemon");
        dnssd::register(&mdns, self_ipv4, 3456, service_type, instance_name);
    });

    let firebase = Firebase::auth(&firebase_url, &firebase_key).unwrap();
    let mut coap_server = Server::new(self_ipv4.to_string() + ":5683").unwrap();

    println!("Server up on {}", self_ipv4);

    coap_server
        .run(|request| async {
            let tmp = String::from_utf8(request.message.payload.clone()).unwrap();
            let regexg = Regex::new(regex_pattern_get).unwrap();
            let regexa = Regex::new(regex_pattern_all).unwrap();

            if regexa.is_match(&tmp[..])
                && (request.get_method() == &Method::Post || request.get_method() == &Method::Put)
            {
                // partialeq not impl
                println!("String matches the POST/PUT pattern");
            } else if regexg.is_match(&tmp[..])
                && (request.get_method() == &Method::Get || request.get_method() == &Method::Delete)
            {
                println!("String matches the GET/DELETE pattern");
            } else {
                println!("String does not match a valid pattern");
                return match request.response {
                    Some(mut message) => {
                        message.message.payload = b"Invalid Path".to_vec();
                        message.message.header.code =
                            MessageClass::Response(coap_lite::ResponseType::BadOption);
                        Some(message)
                    }
                    _ => None,
                };
            }

            let data: Vec<String> = tmp.rsplitn(2, '/').map(String::from).collect();

            let msg: CoapMessage = CoapMessage {
                data: util::parse_data(&data[0]),
                ipv4: metadata.then(|| request.source.expect("Invalid address").to_string()),
                unix: metadata.then(|| {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                }),
            };

            if request.get_method() == &Method::Put {
                // == PATCH
                if structure { firebase.at(&data[1]).update(&msg).await; }
                else { firebase.at(&data[1]).update(&util::parse_data(&data[0])).await; }
            } else if request.get_method() == &Method::Post {
                // creates a random ID in firebase, not useful
                if structure { firebase.at(&data[1]).set(&msg).await; }
                else { firebase.at(&data[1]).set(&util::parse_data(&data[0])).await; }
            } else if request.get_method() == &Method::Delete {
                // creates a random ID in firebase, not useful
                firebase.at(&data[1]).delete().await;
            } else if request.get_method() == &Method::Get {
                let result = firebase.at(&data[1]).get_as_string().await;
            
                match result {
                    Ok(response) => {
                        let res: Result<CoapMessage, serde_json::Error> =
                            serde_json::from_str(response.data.as_str());
            
                        match res {
                            Ok(coap_message) => {
                                println!("Deserialization successful as CoapMessage: {:?}", coap_message);
                                return match request.response {
                                    Some(mut message) => {
                                        let mut payload = String::new();
                                        for (key, value) in coap_message.data.iter() {
                                            payload.push_str(key);
                                            payload.push(':');
                                            payload.push_str(value);
                                            payload.push(',');
                                        }
            
                                        message.message.payload = payload.as_bytes().to_vec();
                                        println!("Sent reply with payload: {}", payload);
                                        message.message.header.code =
                                            MessageClass::Response(coap_lite::ResponseType::Valid);
                                        Some(message)
                                    }
                                    _ => None,
                                };
                            }
                            Err(_) => {
                                // Try deserializing as HashMap
                                let hashmap_result: Result<HashMap<String, String>, _> = serde_json::from_str(response.data.as_str());
                                match hashmap_result {
                                    Ok(hashmap_data) => {
                                        println!("Deserialization successful as HashMap: {:?}", hashmap_data);
                                        return match request.response {
                                            Some(mut message) => {
                                                let mut payload = String::new();
                                                for (key, value) in hashmap_data.iter() {
                                                    payload.push_str(key);
                                                    payload.push(':');
                                                    payload.push_str(value);
                                                    payload.push(',');
                                                }
                    
                                                message.message.payload = payload.as_bytes().to_vec();
                                                println!("Sent reply with payload: {}", payload);
                                                message.message.header.code =
                                                    MessageClass::Response(coap_lite::ResponseType::Valid);
                                                Some(message)
                                            }
                                            _ => None,
                                        };
                                    }
                                    Err(err) => {
                                        // Handle deserialization error
                                        println!("Deserialization error: {:?}", err);
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => println!("Error: {}", err),
                }
            
            }

            return match request.response {
                Some(mut message) => {
                    message.message.payload = b"".to_vec();
                    message.message.header.code =
                        MessageClass::Response(coap_lite::ResponseType::Valid);
                    Some(message)
                }
                _ => None,
            };
        })
        .await
        .unwrap();
}

fn get_argument(args: &[String], index: usize) -> String {
    match args.get(index) {
        Some(arg) => arg.to_string(),
        None => {
            util::print_usage();
            std::process::exit(1);
        }
    }
}
