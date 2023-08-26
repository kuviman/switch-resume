use async_trait::async_trait;
use std::{future::Future, pin::Pin, task::Poll};

pub type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[async_trait(?Send)]
pub trait ResetHandler<'a> {
    type Output;
    async fn shift<
        Fut: Future<Output = Self::Output> + 'a,
        F: FnOnce(Continuation<'a, Self::Output>) -> Fut + 'a,
    >(
        self,
        f: F,
    );
}

pub type ShiftArg<'a, T> =
    Box<dyn FnOnce(Continuation<'a, T>) -> Pin<Box<dyn Future<Output = T> + 'a>> + 'a>;

struct ResetExecutor<'a, T> {
    continuation: Continuation<'a, T>,
    receiver: async_oneshot::Receiver<ShiftArg<'a, T>>,
}

pub struct ResetHandlerImpl<'a, T: 'a> {
    sender: async_oneshot::Sender<ShiftArg<'a, T>>,
}

#[async_trait(?Send)]
impl<'a, T> ResetHandler<'a> for ResetHandlerImpl<'a, T> {
    type Output = T;
    async fn shift<
        Fut: Future<Output = Self::Output> + 'a,
        F: FnOnce(Continuation<'a, Self::Output>) -> Fut + 'a,
    >(
        self,
        f: F,
    ) {
        let mut sender = self.sender;
        sender
            .send(Box::new(move |c: Pin<Box<dyn Future<Output = T> + 'a>>| {
                Box::pin(f(c)) as Continuation<'a, T>
            }) as ShiftArg<'a, T>)
            .expect("WTF");
        let mut first = true;
        std::future::poll_fn(|_| {
            if first {
                first = false;
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
    }
}

impl<'a, T: 'a> ResetExecutor<'a, T> {
    pub fn new<Fut: Future<Output = T> + 'a>(
        f: impl 'a + FnOnce(ResetHandlerImpl<'a, T>) -> Fut,
    ) -> Self {
        let (sender, receiver) = async_oneshot::oneshot();
        let handler = ResetHandlerImpl { sender };
        Self {
            continuation: Box::pin(f(handler)),
            receiver,
        }
    }
    pub async fn run(mut self) -> T {
        let mut receiver = self.receiver;
        loop {
            let poll = Future::poll(
                self.continuation.as_mut(),
                &mut std::task::Context::from_waker(futures::task::noop_waker_ref()),
            );
            match poll {
                Poll::Ready(value) => return value,
                Poll::Pending => {
                    receiver = match receiver.try_recv() {
                        Ok(shift_arg) => {
                            return shift_arg(self.continuation).await;
                        }
                        Err(async_oneshot::TryRecvError::Empty(receiver)) => receiver,
                        Err(_) => unreachable!(),
                    };
                }
            }
        }
    }
}

pub async fn reset<'a, T: 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(ResetHandlerImpl<'a, T>) -> Fut + 'a,
) -> T {
    ResetExecutor::new(f).run().await
}
