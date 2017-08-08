use std::io;
use std::str;
use std::any::Any;
use std::marker::PhantomData;
use bytes::BytesMut;
use tokio_io::codec::{Encoder, Decoder, Framed};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;
use futures::{future, Future, BoxFuture};
use serde_json;
use serde;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    EnqueueAny(String),
    Enqueue(String, Vec<MessageGroup>),
    Request(MessageGroup)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageGroup(u32);

pub struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Message>> {
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
    type Item = Message;
    type Error = io::Error;

    fn encode(&mut self, msg: Message, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(serde_json::to_string(&msg).expect("Could not serialize!").as_bytes());
        buf.extend(b"\n");
        Ok(())
    }
}

pub struct MessageProto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for MessageProto {
    type Request = Message;
    type Response = Message;
    type Transport = Framed<T, MessageCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(MessageCodec))
    }
}

pub struct MessageQueue {
    pub queue: Vec<Message>,
}

impl Service for MessageQueue {
    type Request = Message;
    type Response = Message;
    type Error = io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ok(req).boxed()
    }
}