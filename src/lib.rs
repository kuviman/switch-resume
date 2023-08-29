//! This crates provides a functionality for running pausible tasks (futures).
//!
//! When pausing, current task continuation is captured
//! and passed as an argument to your pause handler.
//!
//! The pause handler in turn becomes the new continuation -
//! its result becomes the result of the task.
//!
//! This means that the pause handler can completely ignore the resumption,
//! replacing it with anything else,
//! or run it and use its result to produce something new.
//!
//! # Examples
//! ```
//! # futures::executor::block_on(async {
//! async fn on_pause(resume: pausible::Resume<'_, i32, i32>) -> i32 {
//!     println!("Task has been paused");
//!     let resume_result = resume(69).await;
//!     assert_eq!(resume_result, -1);
//!     420
//! }
//! async fn run(task: pausible::Task<'_, i32>) -> i32 {
//!     println!("Task has been started");
//!     let value = task.pause(on_pause).await;
//!     println!("Task has been resumed with {value}. Nice!");
//!     -1
//! }
//! let task_result = pausible::run(run).await;
//! assert_eq!(task_result, 420);
//! # });
//! ```
use std::{future::Future, pin::Pin, task::Poll};

pub type Resume<'a, Arg, T> = Box<dyn FnOnce(Arg) -> Continuation<'a, T> + 'a>;

type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

type BoxPauseHandler<'a, T> = Box<dyn FnOnce(Continuation<'a, T>) -> Continuation<'a, T> + 'a>;

pub struct Task<'a, T: 'a> {
    pause_handler_sender: async_channel::Sender<BoxPauseHandler<'a, T>>,
}

impl<'a, T> Task<'a, T> {
    pub async fn pause<
        ResumeArg: 'a,
        Fut: Future<Output = T> + 'a,
        F: FnOnce(Resume<'a, ResumeArg, T>) -> Fut + 'a,
    >(
        &self,
        f: F,
    ) -> ResumeArg {
        let (mut resume_arg_sender, resume_arg_receiver) = async_oneshot::oneshot();
        self.pause_handler_sender
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

/// Run a pausible task.
///
/// Provided function will be called with a handle to the [Task],
/// and the future returned will be able
/// to be paused and resumed when needed using that handle.
pub async fn run<'a, T: 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(Task<'a, T>) -> Fut + 'a,
) -> T {
    let (pause_handler_sender, pause_handler_receiver) = async_channel::bounded(1);
    let handler = Task {
        pause_handler_sender,
    };
    let mut continuation: Continuation<'a, T> = Box::pin(f(handler));
    loop {
        let poll = Future::poll(
            continuation.as_mut(),
            &mut std::task::Context::from_waker(futures::task::noop_waker_ref()),
        );
        match poll {
            Poll::Ready(result) => return result,
            Poll::Pending => {
                match pause_handler_receiver.try_recv() {
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
