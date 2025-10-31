use bytestring::ByteString;
use cookie::Key;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;
use via::Error;

type Sender = broadcast::Sender<(Uuid, ByteString)>;
type Receiver = broadcast::Receiver<(Uuid, ByteString)>;

pub trait Insert {
    type Error;
    type Output;

    fn insert(self, into: &mut Database) -> Result<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "id")]
pub enum Event {
    Message(Uuid),
    Reaction(Uuid),
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum EventParams {
    Message(MessageParams),
    Reaction(ReactionParams),
}

#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "lowercase")]
pub enum EventWithContext<'a> {
    Message(MessageWithUser<'a>),
    Reaction(ReactionWithUser<'a>),
}

pub struct Chat {
    database: RwLock<Database>,
    channel: (Sender, Receiver),
    secret: Key,
}

#[derive(Default)]
pub struct Database {
    messages: HashMap<Uuid, Message>,
    reactions: HashMap<Uuid, Reaction>,
    threads: HashMap<Uuid, Thread>,
    users: HashMap<Uuid, User>,
}

#[derive(Deserialize)]
pub struct Message {
    id: Uuid,
    content: String,
    from_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageParams {
    content: String,
    thread_id: Uuid,
}

#[derive(Serialize)]
pub struct MessageWithUser<'a> {
    id: &'a Uuid,
    from: &'a User,
    content: &'a str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    id: Uuid,
    value: String,
    from_id: Uuid,
    message_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionParams {
    value: String,
    thread_id: Uuid,
    message_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionWithUser<'a> {
    id: &'a Uuid,
    from: &'a User,
    value: &'a str,
    message_id: &'a Uuid,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Thread {
    id: Uuid,
    name: String,
    events: Vec<Event>,
}

#[derive(Serialize)]
pub struct ThreadPreview<'a> {
    id: &'a Uuid,
    name: &'a str,
}

#[derive(Serialize)]
pub struct ThreadWithEvents<'a> {
    id: &'a Uuid,
    name: &'a str,
    events: Vec<EventWithContext<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct User {
    id: Uuid,
    username: ByteString,
}

#[derive(Clone, Deserialize)]
pub struct UserParams {
    username: String,
}

impl Chat {
    pub fn new(secret: Key) -> Self {
        Chat {
            database: Default::default(),
            channel: broadcast::channel(1024),
            secret,
        }
    }

    pub fn secret(&self) -> &Key {
        &self.secret
    }

    pub async fn broadcast(&self, user: &User, event: Event) -> Result<(), Error> {
        let message = {
            let guard = self.database.read().await;
            let Some(data) = guard.event(&event) else {
                // TODO: log warning message.
                return Ok(());
            };

            ByteString::from(serde_json::to_string(&data)?)
        };

        let (tx, _) = &self.channel;
        tx.send((user.id, message))?;

        Ok(())
    }

    pub async fn join(&self, id: &Uuid) -> Option<(User, Receiver)> {
        let (tx, _) = &self.channel;
        let guard = self.database.read().await;
        let user = guard.users.get(id)?.clone();

        Some((user, tx.subscribe()))
    }

    pub async fn insert<T: Insert>(&self, value: T) -> Result<T::Output, T::Error> {
        value.insert(&mut *self.database.write().await)
    }
}

impl Database {
    fn event(&self, event: &Event) -> Option<EventWithContext<'_>> {
        match event {
            Event::Message(id) => self.message(id).map(EventWithContext::Message),
            Event::Reaction(id) => self.reaction(id).map(EventWithContext::Reaction),
        }
    }

    fn message(&self, id: &Uuid) -> Option<MessageWithUser<'_>> {
        let message = self.messages.get(id)?;

        Some(MessageWithUser {
            id: &message.id,
            from: self.users.get(&message.from_id)?,
            content: &message.content,
        })
    }

    fn thread(&self, id: &Uuid) -> Option<ThreadWithEvents<'_>> {
        let thread = self.threads.get(id)?;
        let events = thread.events.iter().filter_map(|event| match event {
            Event::Message(id) => self.message(id).map(EventWithContext::Message),
            Event::Reaction(id) => self.reaction(id).map(EventWithContext::Reaction),
        });

        Some(ThreadWithEvents {
            id: &thread.id,
            name: &thread.name,
            events: events.collect(),
        })
    }

    fn reaction(&self, id: &Uuid) -> Option<ReactionWithUser<'_>> {
        let reaction = self.reactions.get(id.as_ref())?;

        Some(ReactionWithUser {
            id: &reaction.id,
            from: self.users.get(&reaction.from_id)?,
            value: &reaction.value,
            message_id: &reaction.message_id,
        })
    }
}

impl Insert for (&'_ User, EventParams) {
    type Error = Infallible;
    type Output = Event;

    fn insert(self, into: &mut Database) -> Result<Self::Output, Self::Error> {
        match self.1 {
            EventParams::Message(message) => (self.0, message).insert(into),
            EventParams::Reaction(reaction) => (self.0, reaction).insert(into),
        }
    }
}

impl Insert for (&'_ User, MessageParams) {
    type Error = Infallible;
    type Output = Event;

    fn insert(self, database: &mut Database) -> Result<Self::Output, Self::Error> {
        let id = Uuid::new_v4();
        let (from, params) = self;

        let thread = database
            .threads
            .entry(params.thread_id)
            .or_insert_with(|| Thread::new(params.thread_id));

        let message = Message {
            id,
            content: params.content,
            from_id: from.id,
        };

        database.messages.insert(id, message);
        Ok(thread.push(Event::Message(id)))
    }
}

impl Insert for (&'_ User, ReactionParams) {
    type Error = Infallible;
    type Output = Event;

    fn insert(self, database: &mut Database) -> Result<Self::Output, Self::Error> {
        let id = Uuid::new_v4();
        let (from, params) = self;

        let thread = database
            .threads
            .entry(params.thread_id)
            .or_insert_with(|| Thread::new(params.thread_id));

        let reaction = Reaction {
            id,
            value: params.value,
            from_id: from.id,
            message_id: params.message_id,
        };

        database.reactions.insert(id, reaction);
        Ok(thread.push(Event::Reaction(id)))
    }
}

impl Insert for UserParams {
    type Error = Error;
    type Output = User;

    fn insert(self, database: &mut Database) -> Result<Self::Output, Self::Error> {
        let user = User::new(Uuid::new_v4(), self.username.into());
        let output = user.clone();

        database.users.insert(user.id, user);
        Ok(output)
    }
}

impl Thread {
    fn new(id: Uuid) -> Self {
        Self {
            id,
            name: "New Thread".to_owned(),
            events: Vec::new(),
        }
    }

    fn push(&mut self, event: Event) -> Event {
        self.events.push(event.clone());
        event
    }
}

impl User {
    fn new(id: Uuid, username: ByteString) -> Self {
        Self { id, username }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }
}
