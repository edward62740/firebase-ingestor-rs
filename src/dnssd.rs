use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::net::{Ipv4Addr};

pub fn register(sd: &ServiceDaemon, ip: Ipv4Addr, port: u16, serv_type: String, serv_name: String) {

    let service_hostname = "firebase-ingestor.local";

    // The key string in TXT properties is case insensitive. Only the first
    // (key, val) pair will take effect.
    let properties = vec![("ipv4", ip.to_string())];

    // Register a service.
    let service_info = ServiceInfo::new(
        &serv_type,
        &serv_name,
        service_hostname,
        ip,
        port,
        &properties[..],
    )
    .expect("valid service info")
    .enable_addr_auto();

    let monitor = sd.monitor().expect("Failed to monitor the daemon");

    sd.register(service_info)
        .expect("Failed to register mDNS service");

    println!("Registered service {}.{}", &serv_name, &serv_type);

    // Monitor the daemon events.
    while let Ok(event) = monitor.recv() {
        println!("Daemon event: {:?}", &event);
    }
}
