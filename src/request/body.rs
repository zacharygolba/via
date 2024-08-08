use http_body_util::{BodyExt, Empty};
use hyper::body::{Buf, Incoming};
use std::io::Read;

use crate::Result;

#[derive(Debug)]
pub struct Body {
    inner: Option<Box<Incoming>>,
}

impl Body {
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
        use crate::{http::StatusCode, Error};

        let reader = self.aggregate().await?.reader();

        serde_json::from_reader(reader).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }
}

impl Body {
    pub(crate) fn new(inner: Incoming) -> Self {
        Self {
            inner: Some(Box::new(inner)),
        }
    }

    async fn aggregate(&mut self) -> Result<impl Buf> {
        if let Some(value) = self.inner.take() {
            Ok(value.collect().await?.aggregate())
        } else {
            Ok(Empty::new().collect().await?.aggregate())
        }
    }
}
