use bb8::Pool;
use bytestring::ByteString;
use cookie::Key;
use serde::Serialize;
use std::env::{self, VarError};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::ConnectionManager;
use crate::models::message::Message;
use crate::models::reaction::Reaction;

type Sender = broadcast::Sender<(Uuid, Uuid, ByteString)>;
type Receiver = broadcast::Receiver<(Uuid, Uuid, ByteString)>;

pub struct Chat {
    database: Pool<ConnectionManager>,
    channel: (Sender, Receiver),
    secret: Key,
}

#[derive(Serialize)]
#[serde(content = "data", tag = "type")]
pub enum Event<'a> {
    Message(&'a Message),
    Reaction(&'a Reaction),
}

pub async fn establish_pg_connection() -> Pool<ConnectionManager> {
    let database_url = require_env("DATABASE_URL");
    let manager = ConnectionManager::new(&database_url);
    let result = Pool::builder().build(manager).await;

    result.unwrap_or_else(|error| {
        panic!(
            "failed to establish database connection: url = {}, error = {}",
            database_url, error
        );
    })
}

pub fn load_session_secret() -> Key {
    let secret = require_env("VIA_SECRET_KEY");
    let result = secret.as_bytes().try_into();

    result.expect("unexpected end of input while parsing VIA_SECRET_KEY")
}

fn require_env(var: &str) -> String {
    env::var(var).unwrap_or_else(|error| match error {
        VarError::NotPresent => panic!("missing required env var: {}", var),
        VarError::NotUnicode(_) => panic!("env var \"{}\" is not valid UTF-8", var),
    })
}

impl Chat {
    pub fn new(database: Pool<ConnectionManager>, secret: Key) -> Self {
        let channel = broadcast::channel(1024);

        Self {
            database,
            channel,
            secret,
        }
    }

    pub fn pool(&self) -> &Pool<ConnectionManager> {
        &self.database
    }

    pub fn publish(&self, user_id: Uuid, thread_id: Uuid, event: Event) -> via::Result<()> {
        let json = serde_json::to_string(&event)?.into();
        self.channel.0.send((user_id, thread_id, json))?;
        Ok(())
    }

    pub fn secret(&self) -> &Key {
        &self.secret
    }

    pub fn subscribe(&self) -> Receiver {
        self.channel.0.subscribe()
    }
}
