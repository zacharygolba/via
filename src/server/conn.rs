use std::collections::VecDeque;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task;

use crate::error::DynError;

pub type TaskResult = Result<(), DynError>;
pub type JoinHandle = task::JoinHandle<TaskResult>;

pub struct JoinQueue {
    queue: VecDeque<JoinHandle>,
    turns: usize,
}

impl JoinQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(4096),
            turns: 0,
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
    pub fn spawn<F>(&mut self, future: F)
    where
        F: Future<Output = TaskResult> + Send + 'static,
    {
        self.queue.push_back(task::spawn(future));
    }

    #[inline]
    pub async fn join_next(&mut self) -> Option<TaskResult> {
        poll_fn(|context| self.poll_join_next(context)).await
    }
}

impl JoinQueue {
    fn poll_join_next(&mut self, context: &mut Context) -> Poll<Option<TaskResult>> {
        let queue = &mut self.queue;
        let task = match queue.front_mut() {
            Some(front) => front,
            None => return Poll::Ready(None),
        };

        match Pin::new(task).poll(context) {
            Poll::Pending => {
                if self.turns > 9 {
                    let deprioritized = queue.pop_front().unwrap();
                    queue.push_back(deprioritized);
                    self.turns = 0;
                } else {
                    self.turns += 1;
                }

                Poll::Pending
            }
            Poll::Ready(result) => {
                let joined = queue.pop_front();
                debug_assert!(joined.is_some());

                Poll::Ready(match result {
                    Ok(output) => Some(output),
                    Err(error) => {
                        if error.is_panic() {
                            panic!("{}", error);
                        }

                        Some(Ok(()))
                    }
                })
            }
        }
    }
}
