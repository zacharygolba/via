use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use via::{Error, Event, Response, Result};

type Request = via::Request<Counter>;
type Next = via::Next<Counter>;

struct Counter {
    sucesses: Arc<AtomicU32>,
    errors: Arc<AtomicU32>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = via::app(Counter {
        errors: Arc::new(AtomicU32::new(0)),
        sucesses: Arc::new(AtomicU32::new(0)),
    });

    app.include(|request: Request, next: Next| async {
        let state = Arc::clone(request.state());
        let response = next.call(request).await?;

        if response.status().is_success() {
            state.sucesses.fetch_add(1, Ordering::Relaxed);
        } else {
            state.errors.fetch_add(1, Ordering::Relaxed);
        }

        Ok::<_, Error>(response)
    });

    app.at("/counter").respond(via::get(|request: Request, _: Next| async move {
        let state = request.state();
        let body = format!(
            "Errors: {}\nSucesses: {}",
            state.errors.load(Ordering::Relaxed),
            state.sucesses.load(Ordering::Relaxed)
        );

        Response::text(body).finish()
    }));

    app.listen(("127.0.0.1", 8080), |event| match event {
        Event::ConnectionError(error) | Event::UncaughtError(error) => {
            eprintln!("Error: {}", error);
        }
        Event::ServerReady(address) => {
            println!("Server listening at http://{}", address);
        }
    })
    .await
}
