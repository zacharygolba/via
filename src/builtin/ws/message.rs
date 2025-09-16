use bytes::{Buf, Bytes};
use bytestring::ByteString;
use tokio_websockets::proto::ProtocolError;

pub use tokio_websockets::CloseCode;

pub enum Message {
    Binary(Bytes),
    Close(Option<(CloseCode, Option<ByteString>)>),
    Text(ByteString),
}

#[inline]
fn validate_utf8(bytes: Bytes) -> Result<ByteString, ProtocolError> {
    bytes.try_into().or(Err(ProtocolError::InvalidUtf8))
}

impl From<Bytes> for Message {
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

impl From<String> for Message {
    fn from(data: String) -> Self {
        let bytes = data.into_bytes().into();
        // Safety: String is guaranteed to be UTF-8
        Self::Text(unsafe { ByteString::from_bytes_unchecked(bytes) })
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
                Ok(Self::Text(validate_utf8(bytes)?))
            } else if bytes.is_empty() {
                Ok(Self::Close(None))
            } else {
                match bytes.try_get_u16() {
                    Ok(0..=999) | Ok(4999..) | Err(_) => {
                        Err(ProtocolError::InvalidCloseCode.into())
                    }
                    Ok(u16code) => {
                        let code = u16code.try_into()?;

                        Ok(if bytes.remaining() == 0 {
                            Self::Close(Some((code, None)))
                        } else {
                            let reason = validate_utf8(bytes)?;
                            Self::Close(Some((code, Some(reason))))
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
            Message::Binary(binary) => Self::binary(binary),
            Message::Text(text) => Self::text(text.into_bytes()),

            Message::Close(None) => Self::close(None, ""),
            Message::Close(Some((code, reason))) => {
                Self::close(Some(code), reason.as_deref().unwrap_or_default())
            }
        }
    }
}
