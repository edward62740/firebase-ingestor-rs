extern crate lru;

use ::std::collections::HashMap;
use coap::Server;
use coap_lite::{CoapResponse, MessageClass, RequestType as Method};
use firebase_rs::*;
use local_ip_address::local_ip;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::option;
use std::{env, thread, time::Duration};
use tokio::runtime::Runtime;

use lru::LruCache;
use std::num::NonZeroUsize;

mod util;

#[derive(Serialize, Deserialize, Debug)]
struct CoapMessage {
    data: HashMap<String, String>,
    ipv4: String,
}

#[tokio::main]
#[allow(unused_must_use)] // do not do anything if firebase side has errors
async fn main() {
    env_logger::init();

    let self_ipv4 = local_ip().unwrap();

    let firebase = Firebase::auth(

    )
    .unwrap();

    let regex_pattern_get = r"^/([^/]+/)*([^/]+)?/$";
    let regex_pattern_all = r"^/([^/]+/)*[^/]+$";

    let mut server = Server::new(self_ipv4.to_string() + ":5683").unwrap();
    println!("Server up on {}", self_ipv4);

    let mut lru: LruCache<String, String> = LruCache::new(NonZeroUsize::new(50).unwrap());

    server
        .run(|request| async {
            let tmp = String::from_utf8(request.message.payload.clone()).unwrap();
            let regexg = Regex::new(regex_pattern_get).unwrap();
            let regexa = Regex::new(regex_pattern_all).unwrap();

            if regexa.is_match(&tmp[..]) && (request.get_method() == &Method::Post || request.get_method() == &Method::Put){ // partialeq not impl
                println!("String matches the POST/PUT pattern");
            }  
            else if regexg.is_match(&tmp[..]) && request.get_method() == &Method::Get {
                println!("String matches the GET pattern");
            }
            else {
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
                ipv4: request.source.expect("Invalid address").to_string(),
            };

            if request.get_method() == &Method::Put {
                // == PATCH
                firebase.at(&data[1]).update(&msg).await;
            } else if request.get_method() == &Method::Post {
                // creates a random ID in firebase, not useful
                firebase.at(&data[1]).set(&msg).await;
            } else if request.get_method() == &Method::Get {

                let result = firebase.at(&data[1]).get_as_string().await;
                let res: CoapMessage;
                match result {
                    Ok(response) => {
                        let res: Result<CoapMessage, serde_json::Error> =
                            serde_json::from_str(response.data.as_str());

                        match res {
                            Ok(coap_message) => {
                                // Handle successful deserialization
                                println!("Deserialization successful: {:?}", coap_message);
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

    /*
    // Please use env vars to change the logging level.
    // For example in Linux: `RUST_LOG=debug <program>`.
    env_logger::init();

    // Simple command line options.
    let args: Vec<String> = env::args().collect();
    let mut should_unreg = false;
    for arg in args.iter() {
        match arg.as_str() {
            "--unregister" => should_unreg = true,
            _ => {}
        }
    }

    // Create a new mDNS daemon.
    let mdns = ServiceDaemon::new().expect("Could not create service daemon");
    let service_type = match args.get(1) {
        Some(arg) => format!("{}.local.", arg),
        None => {
            print_usage();
            return;
        }
    };
    let instance_name = match args.get(2) {
        Some(arg) => arg,
        None => {
            print_usage();
            return;
        }
    };

    // With `enable_addr_auto()`, we can give empty addrs and let the lib find them.
    // If the caller knows specific addrs to use, then assign the addrs here.
    let my_addrs = "10.10.1.158";
    let service_hostname = "mdns-example.local.";
    let port = 3456;

    // The key string in TXT properties is case insensitive. Only the first
    // (key, val) pair will take effect.
    let properties = vec![("PATH", "one"), ("Path", "two"), ("PaTh", "three")];

    // Register a service.
    let service_info = ServiceInfo::new(
        &service_type,
        &instance_name,
        service_hostname,
        my_addrs,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    // Optionally, we can monitor the daemon events.
    let monitor = mdns.monitor().expect("Failed to monitor the daemon");
    let service_fullname = service_info.get_fullname().to_string();
    mdns.register(service_info)
        .expect("Failed to register mDNS service");

    println!("Registered service {}.{}", &instance_name, &service_type);

    if should_unreg {
        let wait_in_secs = 20;
        println!("Sleeping {} seconds before unregister", wait_in_secs);
        thread::sleep(Duration::from_secs(wait_in_secs));

        let receiver = mdns.unregister(&service_fullname).unwrap();
        while let Ok(event) = receiver.recv() {
            println!("unregister result: {:?}", &event);
        }
    } else {
        // Monitor the daemon events.
        while let Ok(event) = monitor.recv() {
            println!("Daemon event: {:?}", &event);
        }
    }*/
}
