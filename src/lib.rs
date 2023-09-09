#![doc = include_str!("../README.md")]
//! # Examples
//! ```
#![doc = include_str!("../examples/simple.rs")]
//! ```
//!
//! ```
#![doc = include_str!("../examples/foobar.rs")]
//! ```

use std::{future::Future, pin::Pin, task::Poll};

/// Async function passed to [Task::switch]. Represents the paused continuation.
pub type Resume<'a, Arg, T> = Box<dyn FnOnce(Arg) -> Continuation<'a, T> + 'a>;

type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

type Switch<'a, T> = Box<dyn FnOnce(Continuation<'a, T>) -> Continuation<'a, T> + 'a>;

/// Handle to the running task.
pub struct Task<'a, T: 'a> {
    switch_sender: async_channel::Sender<Switch<'a, T>>,
}

impl<'a, T> Task<'a, T> {
    /// Pause current task execution, switching to a new future.
    /// Current continuation is captured and passed as argument.
    pub async fn switch<
        ResumeArg: 'a,
        Fut: Future<Output = T> + 'a,
        F: FnOnce(Resume<'a, ResumeArg, T>) -> Fut + 'a,
    >(
        &self,
        f: F,
    ) -> ResumeArg {
        let (mut resume_arg_sender, resume_arg_receiver) = async_oneshot::oneshot();
        self.switch_sender
            .try_send(Box::new(move |continuation: Continuation<'a, T>| {
                Box::pin(f(Box::new(move |arg| {
                    resume_arg_sender.send(arg).expect("WTF");
                    continuation
                }))) as Continuation<'a, T>
            }) as Switch<'a, T>)
            .expect("WTF");
        resume_arg_receiver.await.expect("HUH")
    }
}

/// Run a task with switch capability.
///
/// Provided async function will be called with a handle to the [Task],
/// and will be able to use switch operation using that handle.
pub async fn run<'a, T: std::fmt::Debug + 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(Task<'a, T>) -> Fut + 'a,
) -> T {
    let (switch_sender, switch_receiver) = async_channel::bounded(1);
    let task = Task { switch_sender };
    let mut continuation: Option<Continuation<'a, T>> = Some(Box::pin(f(task)));
    std::future::poll_fn(move |cx| loop {
        let poll = Future::poll(continuation.as_mut().unwrap().as_mut(), cx);
        match poll {
            Poll::Ready(result) => return Poll::Ready(result),
            Poll::Pending => {
                match switch_receiver.try_recv() {
                    Ok(switch) => {
                        continuation = Some(switch(continuation.take().unwrap()));
                        continue;
                    }
                    Err(
                        async_channel::TryRecvError::Empty | async_channel::TryRecvError::Closed,
                    ) => {}
                };
                return Poll::Pending;
            }
        }
    })
    .await
}
