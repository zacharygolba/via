use bytestring::ByteString;
use cookie::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;
use via::{Error, raise};

type Sender = broadcast::Sender<(Uuid, ByteString)>;
type Receiver = broadcast::Receiver<(Uuid, ByteString)>;

pub trait Insert {
    type Error;
    type Returning;

    fn insert(self, into: &mut Database) -> Result<Self::Returning, Self::Error>;
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

    pub async fn thread<F, R>(&self, id: &Uuid, select: F) -> Option<R>
    where
        F: FnOnce(ThreadWithEvents) -> R,
    {
        self.database.read().await.thread(id).map(select)
    }

    pub async fn insert<T: Insert>(&self, value: T) -> Result<T::Returning, T::Error> {
        let mut guard = self.database.write().await;
        value.insert(&mut guard)
    }

    pub async fn insert_and_notify<'a, T>(&self, from: &'a Uuid, value: T) -> Result<(), Error>
    where
        (&'a Uuid, T): Insert<Returning = Event>,
        Error: From<<(&'a Uuid, T) as Insert>::Error>,
    {
        let mut guard = self.database.write().await;
        let event = match &(from, value).insert(&mut guard)? {
            Event::Message(id) => EventWithContext::Message(guard.message(id)?),
            Event::Reaction(id) => EventWithContext::Reaction(guard.reaction(id)?),
        };

        let json = serde_json::to_string(&event)?.into();
        self.channel().send((*from, json))?;

        Ok(())
    }

    pub async fn subscribe(&self, id: &Uuid) -> Option<Receiver> {
        let guard = self.database.read().await;

        if guard.users.contains_key(id) {
            Some(self.channel().subscribe())
        } else {
            None
        }
    }

    #[inline]
    fn channel(&self) -> &Sender {
        &self.channel.0
    }
}

impl Database {
    fn message(&self, id: &Uuid) -> Result<MessageWithUser<'_>, Error> {
        let Some(message) = self.messages.get(id) else {
            raise!(message = format!("message with id \"{}\" does not exist", id));
        };

        Ok(MessageWithUser {
            id: &message.id,
            from: self.user(&message.from_id)?,
            content: &message.content,
        })
    }

    fn reaction(&self, id: &Uuid) -> Result<ReactionWithUser<'_>, Error> {
        let Some(reaction) = self.reactions.get(id) else {
            raise!(message = format!("reaction with id \"{}\" does not exist", id));
        };

        Ok(ReactionWithUser {
            id: &reaction.id,
            from: self.user(&reaction.from_id)?,
            value: &reaction.value,
            message_id: &reaction.message_id,
        })
    }

    fn thread(&self, id: &Uuid) -> Option<ThreadWithEvents<'_>> {
        let thread = self.threads.get(id)?;
        let events = thread.events.iter().filter_map(|event| match event {
            Event::Message(id) => self.message(id).map(EventWithContext::Message).ok(),
            Event::Reaction(id) => self.reaction(id).map(EventWithContext::Reaction).ok(),
        });

        Some(ThreadWithEvents {
            id: &thread.id,
            name: &thread.name,
            events: events.collect(),
        })
    }

    fn user(&self, id: &Uuid) -> Result<&User, Error> {
        self.users.get(id).map_or_else(
            || raise!(message = format!("user with id \"{}\" does not exist", id)),
            Ok,
        )
    }
}

impl Insert for (&'_ Uuid, EventParams) {
    type Error = Infallible;
    type Returning = Event;

    fn insert(self, into: &mut Database) -> Result<Self::Returning, Self::Error> {
        match self.1 {
            EventParams::Message(message) => (self.0, message).insert(into),
            EventParams::Reaction(reaction) => (self.0, reaction).insert(into),
        }
    }
}

impl Message {
    fn new(content: String, from_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            from_id,
        }
    }
}

impl Insert for (&'_ Uuid, MessageParams) {
    type Error = Infallible;
    type Returning = Event;

    fn insert(self, database: &mut Database) -> Result<Self::Returning, Self::Error> {
        let (from_id, params) = self;
        let message = Message::new(params.content, *from_id);
        let event = database
            .threads
            .entry(params.thread_id)
            .or_insert_with(|| Thread::new(params.thread_id))
            .push(Event::Message(message.id));

        database.messages.insert(message.id, message);
        Ok(event)
    }
}

impl Reaction {
    fn new(value: String, from_id: Uuid, message_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            value,
            from_id,
            message_id,
        }
    }
}

impl Insert for (&'_ Uuid, ReactionParams) {
    type Error = Infallible;
    type Returning = Event;

    fn insert(self, database: &mut Database) -> Result<Self::Returning, Self::Error> {
        let (from_id, params) = self;
        let reaction = Reaction::new(params.value, *from_id, params.message_id);
        let event = database
            .threads
            .entry(params.thread_id)
            .or_insert_with(|| Thread::new(params.thread_id))
            .push(Event::Reaction(reaction.id));

        database.reactions.insert(reaction.id, reaction);
        Ok(event)
    }
}

impl Insert for UserParams {
    type Error = Error;
    type Returning = User;

    fn insert(self, database: &mut Database) -> Result<Self::Returning, Self::Error> {
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
