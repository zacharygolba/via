use bytes::{Buf, Bytes, TryGetError};
use bytestring::ByteString;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio_websockets::proto::ProtocolError;

use crate::error::Error;
use crate::payload::{Payload, deserialize_json};

pub use tokio_websockets::CloseCode;

pub(super) type Sender = mpsc::Sender<Message>;
pub(super) type Receiver = mpsc::Receiver<Message>;

pub struct Channel(Sender, Receiver);

#[derive(Debug)]
#[non_exhaustive]
pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteString>)>),
    Text(ByteString),
}

impl Channel {
    pub(super) fn new() -> (Self, Sender, Receiver) {
        let (sender, rx) = mpsc::channel(1);
        let (tx, receiver) = mpsc::channel(1);

        (Self(tx, rx), sender, receiver)
    }

    pub async fn send(&mut self, message: impl Into<Message>) -> Result<(), Error> {
        if self.0.send(message.into()).await.is_err() {
            Err(tokio_websockets::Error::AlreadyClosed.into())
        } else {
            Ok(())
        }
    }

    pub async fn next(&mut self) -> Option<Message> {
        self.1.recv().await
    }
}

impl Message {
    pub fn json(data: &impl Serialize) -> Result<Self, Error> {
        Ok(serde_json::to_string(data)?.into())
    }
}

impl From<Bytes> for Message {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<ByteString> for Message {
    #[inline]
    fn from(data: ByteString) -> Self {
        Self::Text(data)
    }
}

impl From<Vec<u8>> for Message {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::from(Bytes::from(data))
    }
}

impl From<&'_ [u8]> for Message {
    #[inline]
    fn from(data: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(data))
    }
}

impl From<String> for Message {
    #[inline]
    fn from(data: String) -> Self {
        ByteString::from(data).into()
    }
}

impl From<&'_ str> for Message {
    #[inline]
    fn from(data: &'_ str) -> Self {
        ByteString::from(data).into()
    }
}

impl Payload for Message {
    fn copy_to_bytes(self) -> Bytes {
        match self {
            Self::Binary(bytes) => Payload::copy_to_bytes(bytes),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                Payload::copy_to_bytes(utf8.into_bytes())
            }
        }
    }

    fn into_utf8(self) -> Result<String, Error> {
        match self {
            Self::Binary(bytes) => bytes.into_utf8(),
            Self::Close(None) | Self::Close(Some((_, None))) => Ok(Default::default()),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                let vec = utf8.into_bytes().into_vec();
                // Safety: ValidUtf8 is only constructed from valid UTF-8 byte sequences.
                unsafe { Ok(String::from_utf8_unchecked(vec)) }
            }
        }
    }

    fn into_vec(self) -> Vec<u8> {
        match self {
            Self::Binary(bytes) => bytes.into_vec(),
            Self::Close(None) | Self::Close(Some((_, None))) => Default::default(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => utf8.into_bytes().into_vec(),
        }
    }

    fn serde_json_untagged<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let detached = match self {
            Self::Binary(mut bytes) => bytes.split_to(bytes.len()),
            Self::Close(None) | Self::Close(Some((_, None))) => Bytes::new(),
            Self::Close(Some((_, Some(utf8)))) | Self::Text(utf8) => {
                let mut bytes = utf8.into_bytes();
                bytes.split_to(bytes.len())
            }
        };

        // Allocation not required when json is sourced from a ws message.
        deserialize_json(detached.as_ref())
    }
}

impl TryFrom<tokio_websockets::Message> for Message {
    type Error = tokio_websockets::Error;

    fn try_from(message: tokio_websockets::Message) -> Result<Self, Self::Error> {
        let is_binary = message.is_binary();
        let is_text = !is_binary && message.is_text();

        let mut bytes = Bytes::from(message.into_payload());

        if is_binary {
            Ok(Self::Binary(bytes))
        } else if is_text {
            let utf8 = bytes.try_into().or(Err(ProtocolError::InvalidUtf8))?;
            Ok(Self::Text(utf8))
        } else {
            // Continuation, Ping, and Pong messages are handled by
            // tokio_websockets. The message opcode must be close.
            match bytes.try_get_u16() {
                // The payload is empty and therefore, valid.
                Err(TryGetError { available: 0, .. }) => Ok(Self::Close(None)),

                // The payload starts with an invalid close code.
                Ok(0..=999) | Ok(4999..) | Err(_) => Err(ProtocolError::InvalidCloseCode.into()),

                // The payload contains a valid close code and reason.
                Ok(u16) => {
                    let code = u16.try_into()?;

                    Ok(if bytes.remaining() == 0 {
                        Self::Close(Some((code, None)))
                    } else {
                        let reason = bytes.try_into().or(Err(ProtocolError::InvalidUtf8))?;
                        Self::Close(Some((code, Some(reason))))
                    })
                }
            }
        }
    }
}

impl From<Message> for tokio_websockets::Message {
    #[inline]
    fn from(message: Message) -> Self {
        match message {
            Message::Binary(binary) => Self::binary(binary),
            Message::Text(text) => Self::text(text.into_bytes()),

            Message::Close(None) => Self::close(None, ""),
            Message::Close(Some((code, reason))) => {
                Self::close(Some(code), reason.as_deref().unwrap_or_default())
            }
        }
    }
}
