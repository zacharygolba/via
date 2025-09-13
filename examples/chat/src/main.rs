use std::collections::HashMap;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, broadcast};
use via::builtin::rescue;
use via::ws::Message;
use via::{BoxError, Request, Response, error};

struct Chat {
    rooms: RwLock<HashMap<String, Room>>,
    id: Arc<AtomicU64>,
}

struct Room {
    channel: (
        broadcast::Sender<(u64, String)>,
        broadcast::Receiver<(u64, String)>,
    ),
    messages: Vec<String>,
}

impl Room {
    fn new() -> Self {
        Self {
            channel: broadcast::channel(1024),
            messages: vec![],
        }
    }
}

#[tokio::main]
async fn main() -> Result<ExitCode, BoxError> {
    // Create a new application.
    let mut app = via::app(Chat {
        rooms: RwLock::new(HashMap::new()),
        id: Arc::new(AtomicU64::new(0)),
    });

    // Capture errors from downstream, log them, and map them into responses.
    // Upstream middleware remains unaffected and continues execution.
    app.include(rescue::inspect(|error| eprintln!("error: {}", error)));

    // Define a route that listens on /hello/:name.
    app.at("/chat/:room").scope(|chat| {
        #[rustfmt::skip]
        chat.at("/all").respond(via::get(async |request: Request<Chat>, _| {
            let name = request.param("room").into_result()?;
            let state = request.state();

            match state.rooms.read().await.get(name.as_ref()) {
                Some(room) => Response::build().json(&room.messages),
                None => Err(error!(404, "Unknown room \"{}\".", name)),
            }
        }));

        chat.ws(async |mut socket, param| {
            let room = param.ok_or("missing room param")?;
            let state = socket.state().clone();
            let rooms = &state.rooms;
            let id = state.id.fetch_add(1, Ordering::Relaxed);
            let (tx, mut rx) = {
                let mut guard = state.rooms.write().await;
                let room = guard.entry(room.to_owned()).or_insert_with(Room::new);
                let tx = room.channel.0.clone();
                let rx = tx.subscribe();

                (tx, rx)
            };

            loop {
                tokio::select! {
                    next = socket.next() => match next {
                        Some(Ok(message)) => {
                            let text = match message.as_text() {
                                Some(text) => text.to_owned(),
                                None => continue,
                            };

                            'guard: {
                                let mut guard = rooms.write().await;
                                let room_mut = match guard.get_mut(&room) {
                                    Some(existing) => existing,
                                    None => break 'guard,
                                };

                                room_mut.messages.push(text.clone());
                                tx.send((id, text))?;
                            }
                        }
                        Some(Err(error)) => {
                            eprintln!("error(room: {}): {}", room, error);
                            continue;
                        }
                        None => {
                            break Ok(());
                        }
                    },
                    result = rx.recv() => {
                        if let Ok((sender, update)) = result
                            && sender != id
                        {
                            socket.send(Message::text(update)).await?;
                        }
                    }
                }
            }
        });
    });

    via::start(app).listen(("127.0.0.1", 8080)).await
}
