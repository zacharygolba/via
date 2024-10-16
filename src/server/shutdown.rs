use tokio::signal;
use tokio::sync::watch;
use tokio::task::{self, JoinHandle};

use crate::error::AnyError;

pub fn wait_for_shutdown() -> (JoinHandle<Result<(), AnyError>>, watch::Receiver<bool>) {
    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let (tx, rx) = watch::channel(false);

    // Spawn a task to wait for a "Ctrl-C" signal to be sent to the process.
    let task = task::spawn(async move {
        match signal::ctrl_c().await {
            Ok(_) => tx.send(true).map_err(|_| {
                let message = "unable to notify connections to shutdown.";
                message.to_owned().into()
            }),
            Err(error) => {
                if cfg!(debug_assertions) {
                    eprintln!("unable to register the 'Ctrl-C' signal.");
                }

                Err(error.into())
            }
        }
    });

    (task, rx)
}
