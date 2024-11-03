use tokio::signal;
use tokio::sync::watch;
use tokio::task::{self, JoinHandle};

pub type ShutdownTx = watch::Sender<Option<bool>>;
pub type ShutdownRx = watch::Receiver<Option<bool>>;

pub fn wait_for_shutdown() -> (ShutdownTx, ShutdownRx, JoinHandle<()>) {
    // Create a watch channel to notify the main thread to shutdown if
    // a connection task encounters an unrecoverable error.
    let (conn_error_tx, mut conn_error_rx) = watch::channel(None);

    // Create a watch channel to notify the connections to initiate a
    // graceful shutdown process when the `ctrl_c` future resolves.
    let (shutdown_tx, shutdown_rx) = watch::channel(None);

    let shutdown_task = task::spawn(async move {
        //
        // TODO:
        //
        // Replace `eprintln` this with tracing. Once tracing is implemented,
        // the `debug_assertions` flags can be removed.
        //
        tokio::select! {
            ctrl_c_result = signal::ctrl_c() => match ctrl_c_result {
                // A scheduled shutdown was requested.
                //
                Ok(_) => match shutdown_tx.send(Some(true)) {
                    Err(send_error) if cfg!(debug_assertions) => {
                        let _ = send_error; // Placeholder for tracing...
                        eprintln!("unable to notify connections to shutdown.");
                    },
                    Err(_) | Ok(_) => {},
                },

                // Placeholder for tracing...
                //
                // Ok(_) => if let Err(send_error) = shutdown_tx.send(Some(true)) {
                //     error!("unable to notify connections to shutdown.");
                //     error!("reason: {}", send_error);
                // },

                // Graceful shutdown is either not supported on the host os or
                // an unlikely error occurred at the os level when attempting to
                // register the signal. The server can continue to operate as
                // usual but connections may be closed before responses are
                // sent.
                //
                Err(ctrl_c_error) => {
                    let _ = ctrl_c_error; // Placeholder for tracing...
                    if cfg!(debug_assertions) {
                        eprintln!("unable to register the 'Ctrl-C' signal.");
                    }
                },
            },

            // An error occurred in a connection task that should result in
            // shutting down or restarting the server. If you deploy your app in
            // a cluster with a load balancer, we recommend that you immutably
            // replace the node. Otherwise, you can run Via as a daemon with the
            // appropriate configuration to restart if the main process exits
            // with an error.
            //
            conn_error_result = conn_error_rx.changed() => match conn_error_result {
                // A connection task encountered an unrecoverable error.
                Ok(()) => match *conn_error_rx.borrow_and_update() {
                    None => {
                        // We are only concerned with Some(_) values here.
                    },

                    Some(true) => match shutdown_tx.send(Some(true)) {
                        Err(send_error) if cfg!(debug_assertions) => {
                            let _ = send_error; // Placeholder for tracing...
                            eprintln!("unable to notify connections to shutdown.");
                        }
                        Err(_) | Ok(_) => {}
                    },

                    Some(false) => {
                        // TODO: Replace this with tracing...
                        if cfg!(debug_assertions) {
                            eprintln!("connections cannot request a scheduled shutdown");
                        }
                    },
                },

                // The sender was dropped. This branch is likely unreachable on
                // most platforms.
                //
                Err(recv_error) => {
                    let _ = recv_error; // Placeholder for tracing...
                },
            }
        }
    });

    (conn_error_tx, shutdown_rx, shutdown_task)
}
