use std::{future::Future, pin::Pin, task::Poll};

pub type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

type BoxPauseHandler<'a, T> = Box<dyn FnOnce(Continuation<'a, T>) -> Continuation<'a, T> + 'a>;

pub struct Task<'a, T: 'a> {
    pause_handler_sender: async_channel::Sender<BoxPauseHandler<'a, T>>,
}

impl<'a, T> Task<'a, T> {
    pub async fn pause<
        Value: 'a,
        Fut: Future<Output = T> + 'a,
        F: FnOnce(Box<dyn FnOnce(Value) -> Continuation<'a, T> + 'a>) -> Fut + 'a,
    >(
        &self,
        f: F,
    ) -> Value {
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
