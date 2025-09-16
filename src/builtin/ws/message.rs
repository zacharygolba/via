use bytes::{Buf, Bytes};
use std::io;
use std::ops::Deref;
use tokio_websockets::proto::ProtocolError;

pub use tokio_websockets::CloseCode;

#[derive(Debug)]
pub struct ByteStr(Bytes);

pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteStr>)>),
    Text(ByteStr),
}

impl Deref for ByteStr {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let slice = self.0.as_ref();
        unsafe { str::from_utf8_unchecked(slice) }
    }
}

impl TryFrom<Bytes> for ByteStr {
    type Error = tokio_websockets::Error;

    #[inline]
    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        if let Err(_) = str::from_utf8(&bytes) {
            Err(Self::Error::Io(io::ErrorKind::InvalidData.into()))
        } else {
            Ok(Self(bytes))
        }
    }
}

impl From<Bytes> for Message {
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<String> for Message {
    fn from(data: String) -> Self {
        Self::Text(ByteStr(data.into_bytes().into()))
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Self::from(Bytes::from(data))
    }
}

impl From<&'_ str> for Message {
    fn from(data: &'_ str) -> Self {
        Self::from(data.as_bytes())
    }
}

impl From<&'_ [u8]> for Message {
    fn from(data: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(data))
    }
}

impl TryFrom<tokio_websockets::Message> for Message {
    type Error = tokio_websockets::Error;

    fn try_from(message: tokio_websockets::Message) -> Result<Self, Self::Error> {
        if message.is_binary() {
            Ok(Self::Binary(message.into_payload().into()))
        } else {
            let is_text = message.is_text();
            let mut bytes = Bytes::from(message.into_payload());

            if is_text {
                Ok(Self::Text(bytes.try_into()?))
            } else if bytes.is_empty() {
                Ok(Self::Close(None))
            } else {
                match bytes.try_get_u16() {
                    Ok(0..=999) | Ok(4999..) | Err(_) => {
                        Err(ProtocolError::InvalidCloseCode.into())
                    }
                    Ok(u16code) => {
                        let code = u16code.try_into()?;
                        Ok(if bytes.remaining() > 0 {
                            let reason = Some(bytes.try_into()?);
                            Self::Close(Some((code, reason)))
                        } else {
                            Self::Close(Some((code, None)))
                        })
                    }
                }
            }
        }
    }
}

impl From<Message> for tokio_websockets::Message {
    #[inline]
    fn from(message: Message) -> Self {
        match message {
            Message::Binary(data) => Self::binary(data),
            Message::Text(ByteStr(data)) => Self::text(data),

            Message::Close(None) => Self::close(None, ""),
            Message::Close(Some((code, reason))) => {
                Self::close(Some(code), reason.as_deref().unwrap_or_default())
            }
        }
    }
}
