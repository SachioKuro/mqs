extern crate message_queue_service;
extern crate tokio_proto;

use tokio_proto::TcpServer;
use std::collections::HashMap;

pub fn main() {
    // Specify the localhost address
    let addr = "0.0.0.0:12345".parse().unwrap();

    // The builder requires a protocol and an address
    let server = TcpServer::new(message_queue_service::mqs::MessageProto, addr);

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    server.serve(|| Ok(message_queue_service::mqs::MessageQueue{queues: HashMap::new()}));
}