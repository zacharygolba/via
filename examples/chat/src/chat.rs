use bytestring::ByteString;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, broadcast};

pub type Channel = (
    broadcast::Sender<(u64, usize)>,
    broadcast::Receiver<(u64, usize)>,
);

pub struct Chat {
    rooms: RwLock<HashMap<String, Room>>,
    id: AtomicU64,
}

pub struct Room {
    channel: Channel,
    messages: Vec<ByteString>,
}

impl Chat {
    pub fn new() -> Self {
        Chat {
            rooms: RwLock::new(HashMap::new()),
            id: AtomicU64::new(1),
        }
    }

    pub async fn all<F, R>(&self, slug: &str, with: F) -> Option<R>
    where
        F: FnOnce(&[ByteString]) -> R,
    {
        let guard = self.rooms.read().await;
        let room = guard.get(slug)?;

        Some(with(&room.messages))
    }

    pub async fn get(&self, slug: &str, index: usize) -> Option<ByteString> {
        let guard = self.rooms.read().await;
        let room = guard.get(slug)?;

        room.messages.get(index).cloned()
    }

    pub async fn push(&self, slug: &str, message: &str) -> Option<usize> {
        let mut guard = self.rooms.write().await;
        let room_mut = &mut guard.get_mut(slug)?;
        let index = room_mut.messages.len();

        room_mut.messages.push(message.into());

        Some(index)
    }

    pub async fn join(&self, slug: &str) -> (u64, Channel) {
        let mut guard = self.rooms.write().await;
        let id = self.id.fetch_add(1, Ordering::Relaxed);

        loop {
            if let Some(room) = guard.get(slug) {
                let (tx, _) = &room.channel;
                break (id, (tx.clone(), tx.subscribe()));
            }

            guard.insert(
                slug.to_owned(),
                Room {
                    channel: broadcast::channel(1024),
                    messages: Vec::new(),
                },
            );
        }
    }
}
