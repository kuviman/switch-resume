use async_trait::async_trait;
use std::{future::Future, pin::Pin, task::Poll};

pub type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[async_trait(?Send)]
pub trait ResetHandler<'a> {
    type Output;
    async fn shift<
        Value: 'a,
        Fut: Future<Output = Self::Output> + 'a,
        F: FnOnce(Box<dyn FnOnce(Value) -> Continuation<'a, Self::Output> + 'a>) -> Fut + 'a,
    >(
        &self,
        f: F,
    ) -> Value;
}

pub type ShiftArg<'a, T> =
    Box<dyn FnOnce(Continuation<'a, T>) -> Pin<Box<dyn Future<Output = T> + 'a>> + 'a>;

pub struct ResetHandlerImpl<'a, T: 'a> {
    sender: async_channel::Sender<ShiftArg<'a, T>>,
}

#[async_trait(?Send)]
impl<'a, T> ResetHandler<'a> for ResetHandlerImpl<'a, T> {
    type Output = T;
    async fn shift<
        Value: 'a,
        Fut: Future<Output = T> + 'a,
        F: FnOnce(Box<dyn FnOnce(Value) -> Continuation<'a, T> + 'a>) -> Fut + 'a,
    >(
        &self,
        f: F,
    ) -> Value {
        let (mut sender, receiver) = async_oneshot::oneshot();
        self.sender
            .try_send(Box::new(move |c: Pin<Box<dyn Future<Output = T> + 'a>>| {
                Box::pin(f(Box::new(move |value| {
                    sender.send(value).expect("WTF");
                    c
                }))) as Continuation<'a, T>
            }) as ShiftArg<'a, T>)
            .expect("WTF");
        receiver.await.expect("HUH")
    }
}

pub async fn reset<'a, T: 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(ResetHandlerImpl<'a, T>) -> Fut + 'a,
) -> T {
    let (sender, receiver) = async_channel::bounded(1);
    let handler = ResetHandlerImpl { sender };
    let mut continuation: Continuation<'a, T> = Box::pin(f(handler));
    loop {
        let poll = Future::poll(
            continuation.as_mut(),
            &mut std::task::Context::from_waker(futures::task::noop_waker_ref()),
        );
        match poll {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                match receiver.try_recv() {
                    Ok(shift_arg) => {
                        continuation = shift_arg(continuation);
                    }
                    Err(async_channel::TryRecvError::Empty) => {}
                    Err(_) => unreachable!(),
                };
            }
        }
    }
}
