use std::collections::VecDeque;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::{task, time};

use crate::error::DynError;

pub type TaskResult = Result<(), DynError>;

pub struct JoinQueue {
    queue: VecDeque<task::JoinHandle<TaskResult>>,
    idle: bool,
}

impl JoinQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(4096),
            idle: true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    #[inline]
    pub fn push<F>(&mut self, future: F)
    where
        F: Future<Output = TaskResult> + Send + 'static,
    {
        self.queue.push_back(task::spawn(future));
    }

    pub async fn join_next(&mut self) -> Option<TaskResult> {
        poll_fn(|context| self.poll_join_next(context)).await
    }

    pub async fn try_join_next_in(&mut self, timeout: Duration) -> Option<TaskResult> {
        tokio::select! {
            joined = self.join_next() => joined,
            _ = time::sleep(timeout) => {
                if self.queue.len() > 1 {
                    // If the connection task can't be joined in `timeout`,
                    // move it to the back of the queue. It might be using
                    // a websocket.
                    match self.queue.pop_front() {
                        Some(handle) => self.queue.push_back(handle),
                        None => panic!("join_next was not canceled safely"),
                    }
                }

                self.idle = true;

                None
            }
        }
    }
}

impl JoinQueue {
    fn poll_join_next(&mut self, context: &mut Context) -> Poll<Option<TaskResult>> {
        let next = match self.queue.front_mut() {
            Some(handle) => handle,
            None => return Poll::Ready(None),
        };

        match Pin::new(next).poll(context) {
            Poll::Ready(ready) => {
                self.idle = true;
                self.queue.pop_front();
                Poll::Ready(match ready {
                    Ok(result) => Some(result),
                    Err(error) => {
                        if error.is_panic() {
                            panic!("{}", error);
                        }

                        Some(Ok(()))
                    }
                })
            }
            Poll::Pending => {
                if self.idle {
                    self.idle = false;
                    context.waker().wake_by_ref();
                }

                Poll::Pending
            }
        }
    }
}
