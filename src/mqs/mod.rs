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
use serde::Serialize;
use std::sync::Mutex;


/*
 *  Defines the MessageQueue operations
 *
 *  EnqueueAny     - Enqueue a message on all channels
 *  GetAllMessages - Receive all messages from any channel 
 */
#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    EnqueueAny,     
    GetAllMessages
}

/*
 *  Represents a Message,
 *  holding its own type and the message, 
 *  which can be any structure which implements Serialize and Deserialize
 *  from serde
 */
#[derive(Serialize, Debug)]
pub struct Message<T: Serialize> {
    pub msg_type: MessageType,
    pub msg: T,
}

impl<T: Serialize> Message<T> {
    /*
     *  Creates a Message and gives back a json representation of itself
     */
    pub fn create_message(msg_type: MessageType, msg: T) -> String {
        serde_json::to_string(&Message::<T>{ msg_type, msg }).unwrap()
    }
}

// Coder- / Decoderpair
pub struct MessageCodec;

/*
 *  Decodes a string
 */
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
                Ok(s) => Ok(Some(String::from(s))),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }
}

/*
 *  Encodes a string
 */
impl Encoder for MessageCodec {
    type Item = String;
    type Error = io::Error;

    fn encode(&mut self, msg: String, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(msg.as_bytes());
        buf.extend(b"\n");
        Ok(())
    }
}

/*
 *  The server protocol which hooks up the codec
 */
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

/*
 *  The MessageQueue itself.
 *  Channel -> List of Messages
 */
lazy_static! {
    static ref QUEUES: Mutex<HashMap<&'static str, Vec<String>>> = Mutex::new(HashMap::new());
}

/*
 *  A Service which the queue provides.
 */
pub struct MessageQueue;

impl Service for MessageQueue {
    // JSON Object {a:b, c:d}
    type Request = String;
    // JSON Object {"result": [{a:b, c:d}, {e:f, g:h}]}
    type Response = String;
    type Error = io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ok(req).boxed()
    }
}

impl MessageQueue {
    pub fn enqueue_any(body: String) {
        for (_, v) in QUEUES.lock().unwrap().iter_mut() {
            v.push(body.clone());
        };
    }

    pub fn get_all_messages() -> Vec<(String, String)> {
        let mut msgs = vec!();
        for (&k, vs) in QUEUES.lock().unwrap().iter() {
            for & ref v in vs {
                msgs.push((String::from(k), String::from(v.clone())));
            } 
        }
        msgs
    }
}