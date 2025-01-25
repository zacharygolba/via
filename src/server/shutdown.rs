use tokio::signal;
use tokio::sync::watch::{self, Receiver, Sender};
use tokio::task::JoinHandle;

pub fn wait_for_shutdown() -> (JoinHandle<()>, Sender<Option<bool>>, Receiver<Option<bool>>) {
    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let (tx, rx) = watch::channel(None);

    // Spawn a task to wait for the ctrl_c signal.
    let task = tokio::spawn({
        let tx = tx.clone();

        async move {
            if let Err(_) = signal::ctrl_c().await {
                // Placeholder for tracing...
                eprintln!("unable to register the 'ctrl-c' signal.");
                return;
            }

            if let Err(_) = tx.send(Some(false)) {
                // Placeholder for tracing...
                eprintln!("unable to notify connections to shutdown.");
            }
        }
    });

    (task, tx, rx)
}
