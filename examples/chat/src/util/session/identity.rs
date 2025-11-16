use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use std::str::FromStr;
use time::{Duration, OffsetDateTime};
use via::{error::BoxError, raise};

use crate::util::Id;

const BYTE_LEN: usize = 24;
const STR_LEN: usize = 32;

#[derive(Clone)]
pub struct Identity {
    uuid: Id,
    expires_at: i64,
}

fn decode(input: &str) -> Result<Identity, BoxError> {
    let mut buf = [0u8; BYTE_LEN];

    URL_SAFE_NO_PAD.decode_slice(input, &mut buf)?;

    Ok(Identity {
        uuid: Id::try_from(&buf[..16])?,
        expires_at: i64::from_be_bytes(buf[16..BYTE_LEN].try_into()?),
    })
}

impl Identity {
    pub(super) fn new(uuid: Id) -> Self {
        let expires_at = OffsetDateTime::now_utc() + Duration::days(1);

        Self {
            uuid,
            expires_at: expires_at.unix_timestamp(),
        }
    }

    pub fn encode(&self) -> String {
        let mut buf = [0u8; BYTE_LEN];

        buf[..16].copy_from_slice(self.uuid.as_ref());
        buf[16..].copy_from_slice(&self.expires_at.to_be_bytes());

        URL_SAFE_NO_PAD.encode(buf)
    }

    pub fn id(&self) -> &Id {
        &self.uuid
    }

    pub fn is_expired(&self) -> bool {
        OffsetDateTime::now_utc().unix_timestamp() > self.expires_at
    }
}

impl FromStr for Identity {
    type Err = via::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if input.len() == STR_LEN {
            decode(input).or_else(|error| raise!(401, boxed = error))
        } else {
            raise!(401, message = "invalid session cookie.");
        }
    }
}
