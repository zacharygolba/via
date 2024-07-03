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
        let state = request.state();
        let errors = Arc::clone(&state.errors);
        let sucesses = Arc::clone(&state.sucesses);
        let response = next.call(request).await?;

        if response.status().is_success() {
            sucesses.fetch_add(1, Ordering::Relaxed);
        } else {
            errors.fetch_add(1, Ordering::Relaxed);
        }

        Ok::<_, Error>(response)
    });

    app.at("/counter").respond(via::get(|request: Request, _: Next| async move {
        let state = request.state();
        let errors = state.errors.load(Ordering::Relaxed);
        let sucesses = state.sucesses.load(Ordering::Relaxed);

        Response::text(format!("Errors: {}\nSucesses: {}", errors, sucesses)).end()
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
