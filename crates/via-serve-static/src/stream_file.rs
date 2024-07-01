use futures_util::Stream;
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, SystemTime},
};
use tokio::{runtime::Handle, sync::mpsc, task};
use via::{Error, Result};

const HIGH_WATERMARK: usize = 32768; // 32KB

type ReadChunkedResult = Result<Vec<u8>, io::Error>;

pub struct StreamFile {
    receiver: mpsc::Receiver<ReadChunkedResult>,
}

fn stream_file_blocking(sender: mpsc::Sender<ReadChunkedResult>, path: PathBuf) -> Result<()> {
    let start_time = SystemTime::now();
    let mut file = File::open(&path)?;
    let mut buf = vec![0; HIGH_WATERMARK];

    loop {
        match file.read(&mut buf) {
            Err(e) => {
                let _ = sender.blocking_send(Err(e));
                break;
            }
            Ok(0) => {
                // We reached the end of the file.
                break;
            }
            Ok(n) => {
                if sender.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                    // Receiver has been dropped.
                    break;
                }
            }
        }

        // Check if we have been reading for more than 60 seconds. If so, break the loop.
        // In the future we may want to make this value configurable.
        if start_time.elapsed()?.as_secs() > 60 {
            let error = io::Error::new(io::ErrorKind::TimedOut, "Timed out while reading file");
            let _ = sender.blocking_send(Err(error));
            break;
        }

        if sender.capacity() > 0 {
            continue;
        }

        task::block_in_place(|| {
            // Wait for the receiver to have capacity.
            Handle::current().block_on(async {
                while sender.capacity() == 0 {
                    // Sleep for 25 microseconds. The sleep duration is arbitrary
                    // and may need to be adjusted.
                    tokio::time::sleep(Duration::from_micros(25)).await;
                }
            });
        });
    }

    // Zero out the buffer.
    buf.fill(0);

    Ok(())
}

impl StreamFile {
    pub fn new(path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel(16);

        // Spawn a blocking task to read the file in chunks.
        task::spawn_blocking(|| stream_file_blocking(sender, path));

        Self { receiver }
    }
}

impl Stream for StreamFile {
    type Item = Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver
            .poll_recv(context)
            .map_err(Error::from_io_error)
    }
}
