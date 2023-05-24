# Firebase Ingestor

Firebase requires REST API over HTTPS which is too expensive for many power constrained IoT devices.<br>
This project allows for low-power IoT devices to read/write data in a Firebase RTDB instance, by forwarding CoAP messages from the IoT devices.



The program does three things:
- Advertises a service over DNS-SD (RFC 6763) along with its IP address
- Sets up a CoAP server binded to abovementioned IP
- Acts as a bridge : IoT Device <--- CoAP ---> Firebase Ingestor <--- REST API ---> Firebase

This project was used with Thread networks. The DNS-SD lib only supports IPv4, so NAT64 (RFC 6146) may be needed to communicate from IPv6 IoT devices.

The CoAP payload is simple, in the form /path/../to/data for POST/PUT. For GET, use the form /path/../to/ .