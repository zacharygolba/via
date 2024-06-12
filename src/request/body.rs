use bytes::Buf;
use http_body_util::{BodyExt, Empty};
use hyper::body::Incoming;
use std::io::Read;

use crate::{Error, Result};

#[derive(Debug)]
pub struct Body {
    value: Option<Incoming>,
}

impl Body {
    pub(super) fn new(value: Incoming) -> Self {
        Body { value: Some(value) }
    }

    pub async fn read_bytes(&mut self) -> Result<Vec<u8>> {
        let buf = self.aggregate().await?;
        let mut bytes = Vec::with_capacity(buf.remaining());

        buf.reader().read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    pub async fn read_text(&mut self) -> Result<String> {
        let bytes = self.read_bytes().await?;
        Ok(String::from_utf8(bytes)?)
    }

    #[cfg(feature = "serde")]
    pub async fn read_json<T>(&mut self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let reader = self.aggregate().await?.reader();

        match serde_json::from_reader(reader) {
            Ok(json) => Ok(json),
            Err(e) => Err(Error::from(e).status(400)),
        }
    }

    async fn aggregate(&mut self) -> Result<impl Buf> {
        if let Some(value) = self.value.take() {
            Ok(value.collect().await?.aggregate())
        } else {
            Ok(Empty::new().collect().await?.aggregate())
        }
    }
}
