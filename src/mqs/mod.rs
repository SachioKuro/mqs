use std::io;
use std::str;
use std::collections::HashMap;
use bytes::BytesMut;
use tokio_io::codec::{Encoder, Decoder, Framed};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;
use futures::{future, Future, BoxFuture};
use serde_json;
use serde;
use serde::de::DeserializeOwned;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageGroup(String);

pub struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            // remove the serialized frame from the buffer.
            let line = buf.split_to(i);

            // Also remove the '\n'
            buf.split_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            match str::from_utf8(&line) {
                Ok(s) => Ok(Some(serde_json::from_str(s)
                            .expect("Could not deserialize"))),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for MessageCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: String, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(serde_json::to_string(&msg).expect("Could not serialize!").as_bytes());
        buf.extend(b"\n");
        Ok(())
    }
}

pub struct MessageProto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for MessageProto {
    type Request = String;
    type Response = String;
    type Transport = Framed<T, MessageCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(MessageCodec))
    }
}

lazy_static! {
    static ref QUEUES: Mutex<HashMap<&'static str, Vec<String>>> = Mutex::new(HashMap::new());
}

pub struct MessageQueue;

impl Service for MessageQueue {
    type Request = String;
    type Response = String;
    type Error = io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ok(req).boxed()
    }
}

impl MessageQueue {
    pub fn enqueue_any<T: serde::Serialize>(body: T) -> Result<(), serde_json::Error> {
        match serde_json::to_string(&body) {
            Ok(s) => {
                for (_, v) in QUEUES.lock().unwrap().iter_mut() {
                    v.push(s.clone());
                };
                Ok(())
            }
            Err(e) => Err(e)
        }
    }

    pub fn get_all_messages<T: DeserializeOwned>() -> Vec<(String, T)> {
        let mut msgs = vec!();
        for (&k, vs) in QUEUES.lock().unwrap().iter() {
            for v in vs {
                msgs.push((String::from(k), serde_json::from_str(&v).unwrap()));
            } 
        }
        msgs
    }
}