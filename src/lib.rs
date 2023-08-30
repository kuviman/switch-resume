//! This crates provides functionality for running switchable tasks (futures).
//!
//! Switching is a control flow mechanism that stops normal execution of current task (current function),
//! captures current task continuation and passes it as an argument to the provided async fn.
//! The task then proceeds by evaluating that fn, instead of resuming normally.
//!
//! In order to resume normal execution, the passed resumption object can be called explicitly.
//!
//! This is an implementation of [delimited continuations](https://en.m.wikipedia.org/wiki/Delimited_continuation) in Rust using async that works on stable.
//!
//! # Examples
//! ```
//! # futures::executor::block_on(async {
//! async fn bar(resume: switch_resume::Resume<'_, i32, i32>) -> i32 {
//!     println!("foo has been paused, started bar");
//!     let resume_result = resume(69).await;
//!     assert_eq!(resume_result, -1); // This is the result of foo
//!     420 // This is the final result of task
//! }
//! async fn foo(task: switch_resume::Task<'_, i32>) -> i32 {
//!     println!("foo started");
//!     let value = task.switch(bar).await;
//!     println!("foo was resumed with {value}. Nice!");
//!     -1 // This is not the final task result since we switched to bar
//! }
//! let task_result = switch_resume::run(foo).await;
//! assert_eq!(task_result, 420);
//! # });
//! ```
use std::{future::Future, pin::Pin, task::Poll};

pub type Resume<'a, Arg, T> = Box<dyn FnOnce(Arg) -> Continuation<'a, T> + 'a>;

type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

type BoxPauseHandler<'a, T> = Box<dyn FnOnce(Continuation<'a, T>) -> Continuation<'a, T> + 'a>;

pub struct Task<'a, T: 'a> {
    switch_sender: async_channel::Sender<BoxPauseHandler<'a, T>>,
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
            }) as BoxPauseHandler<'a, T>)
            .expect("WTF");
        resume_arg_receiver.await.expect("HUH")
    }
}

/// Run a task with switch capability.
///
/// Provided async function will be called with a handle to the [Task],
/// and will be able to use switch operation using that handle.
pub async fn run<'a, T: 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(Task<'a, T>) -> Fut + 'a,
) -> T {
    let (switch_sender, switch_receiver) = async_channel::bounded(1);
    let task = Task { switch_sender };
    let mut continuation: Continuation<'a, T> = Box::pin(f(task));
    loop {
        let poll = Future::poll(
            continuation.as_mut(),
            &mut std::task::Context::from_waker(futures::task::noop_waker_ref()),
        );
        match poll {
            Poll::Ready(result) => return result,
            Poll::Pending => {
                match switch_receiver.try_recv() {
                    Ok(pause_handler) => {
                        continuation = pause_handler(continuation);
                    }
                    Err(async_channel::TryRecvError::Empty) => {
                        // TODO wait for waker
                    }
                    Err(_) => unreachable!(),
                };
            }
        }
    }
}
