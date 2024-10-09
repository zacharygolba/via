use bytes::Bytes;
use futures_core::Stream;
use http_body::Frame;
use std::error::Error as StdError;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};
use tokio::{runtime::Handle, sync::mpsc, task};
use via::Result;

/// The amount of `ReadChunkResult` that can be stored in the channel buffer.
const CHANNEL_CAPACITY: usize = 16;

/// The high watermark for the buffer size used to read the file in chunks.
const HIGH_WATERMARK: usize = 32768; // 32KB

/// The duration in microseconds for which the `stream_file_blocking` task will
/// sleep while the channel has no capacity.
const SLEEP_DURATION: u64 = 25;

type ReadChunkResult = Result<Vec<u8>, io::Error>;

pub struct StreamFile {
    receiver: mpsc::Receiver<ReadChunkResult>,
}

/// Calculate the elapsed time in seconds since the provided `SystemTime`. If we
/// are unable to calculate the elapsed time, we return a value greater than the
/// configured timeout.
fn elapsed_as_secs(from: SystemTime, timeout: u64) -> u64 {
    SystemTime::now()
        .duration_since(from)
        .map_or(timeout + 1, |elapsed| elapsed.as_secs())
}

/// Sleep until the channel has capacity. This function should only be called
/// within a blocking task using `block_in_place`. This will prevent the task
/// from blocking the entire runtime while the receiver catches up to the
/// sender.
async fn wait_for_capacity(sender: &mpsc::Sender<ReadChunkResult>) {
    let duration = Duration::from_micros(SLEEP_DURATION);

    while sender.capacity() == 0 {
        tokio::time::sleep(duration).await;
    }
}

/// Read the file at `path` in chunks and send the chunks to the receiver. This
/// function should only be called within a blocking task.
fn stream_file_blocking(sender: mpsc::Sender<ReadChunkResult>, path: PathBuf, timeout: u64) {
    let start_time = SystemTime::now();
    let mut file = match File::open(path) {
        Ok(opened) => opened,
        Err(error) => {
            // There was an error opening the file. Send the error to the
            // receiver and return. Since we didn't allocate our buffer
            // yet, there is no need to zero it out.
            let _ = sender.blocking_send(Err(error));
            return;
        }
    };
    let mut buf = vec![0; HIGH_WATERMARK];

    loop {
        // Check if we have been reading for more than the configured timeout.
        // If so, send a TimedOut error to the receiver, zero out the buffer,
        // and return.
        if elapsed_as_secs(start_time, timeout) > timeout {
            use std::io::{Error, ErrorKind};

            let _ = sender.blocking_send(Err(Error::new(
                ErrorKind::TimedOut,
                "Timed out while reading file",
            )));
            buf.fill(0);
            return;
        }

        if sender.capacity() == 0 {
            // Wait for the channel to have capacity.
            task::block_in_place(|| {
                Handle::current().block_on(wait_for_capacity(&sender));
            });
        }

        match file.read(&mut buf) {
            Err(error) => {
                // An error occurred while reading the file. Send the error to
                // the receiver, zero out the buffer, and return.
                let _ = sender.blocking_send(Err(error));
                buf.fill(0);
                return;
            }
            Ok(0) => {
                // We reached the end of the file. Zero out the buffer and return.
                buf.fill(0);
                return;
            }
            Ok(n) => {
                // Copy the slice of bytes that were read into the buffer and
                // send it as a vec to the receiver.
                if sender.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                    // The receiver has been dropped. This may happen if the
                    // client disconnects before the file has been fully read.
                    // Zero out the buffer and return.
                    buf.fill(0);
                    return;
                }
            }
        }
    }
}

impl StreamFile {
    pub fn new(path: PathBuf, timeout: u64) -> Self {
        let (sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);

        task::spawn_blocking(move || {
            // Spawn a blocking task to read the file in chunks.
            stream_file_blocking(sender, path, timeout)
        });

        Self { receiver }
    }
}

impl Stream for StreamFile {
    type Item = Result<Frame<Bytes>, Box<dyn StdError + Send + Sync>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        this.receiver
            .poll_recv(context)
            .map_ok(|data| Frame::data(data.into()))
            .map_err(|error| error.into())
    }
}
