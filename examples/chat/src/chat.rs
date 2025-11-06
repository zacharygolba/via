use bb8::Pool;
use bytestring::ByteString;
use cookie::Key;
use http::header;
use serde::Serialize;
use std::env::{self, VarError};
use tokio::sync::broadcast;
use uuid::Uuid;
use via::response::{Finalize, ResponseBuilder};
use via::{raise, ws};

use crate::models::ConnectionManager;
use crate::models::message::Message;
use crate::models::reaction::Reaction;

type Sender = broadcast::Sender<(EventContext, EventPayload)>;
type Receiver = broadcast::Receiver<(EventContext, EventPayload)>;

#[derive(Clone)]
pub struct EventPayload(ByteString);

#[derive(Serialize)]
#[serde(content = "data", rename_all = "lowercase", tag = "type")]
pub enum Event {
    Message(Message),
    Reaction(Reaction),
}

pub struct Chat {
    database: Pool<ConnectionManager>,
    channel: (Sender, Receiver),
    secret: Key,
}

#[derive(Clone, Debug)]
pub struct EventContext {
    thread_id: Option<Uuid>,
    user_id: Uuid,
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

    pub fn publish(&self, context: EventContext, event: Event) -> via::Result<EventPayload> {
        let payload = EventPayload(serde_json::to_string(&event)?.into());

        if self.channel.0.send((context, payload.clone())).is_err() {
            raise!(message = "pubsub channel closed.");
        }

        Ok(payload)
    }

    pub fn secret(&self) -> &Key {
        &self.secret
    }

    pub fn subscribe(&self) -> Receiver {
        self.channel.0.subscribe()
    }
}

impl Finalize for EventPayload {
    fn finalize(self, builder: ResponseBuilder) -> via::Result {
        let bytes = self.0.into_bytes();

        builder
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .header(header::CONTENT_LENGTH, bytes.len())
            .body(bytes)
    }
}

impl From<EventPayload> for ws::Message {
    fn from(payload: EventPayload) -> Self {
        ws::Message::Text(payload.0)
    }
}

impl EventContext {
    pub fn new(thread_id: Option<Uuid>, user_id: Uuid) -> Self {
        Self { thread_id, user_id }
    }

    pub fn thread_id(&self) -> Option<&Uuid> {
        self.thread_id.as_ref()
    }

    pub fn user_id(&self) -> &Uuid {
        &self.user_id
    }
}
