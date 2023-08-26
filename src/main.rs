#![allow(dead_code, unused)]

use async_trait::async_trait;
use std::{future::Future, pin::Pin, task::Poll};

type Continuation<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[async_trait(?Send)]
trait ResetHandler {
    type Output;
    async fn shift<
        F: for<'a> FnOnce(Continuation<'a, Self::Output>) -> Continuation<'a, Self::Output> + 'static,
    >(
        self,
        f: F,
    );
}

type ShiftArg<T> =
    Box<dyn for<'a> FnOnce(Continuation<'a, T>) -> Pin<Box<dyn Future<Output = T> + 'a>>>;

struct ResetExecutor<'a, T> {
    continuation: Continuation<'a, T>,
    receiver: async_oneshot::Receiver<ShiftArg<T>>,
}

struct ResetHandlerImpl<T: 'static> {
    sender: async_oneshot::Sender<ShiftArg<T>>,
}

#[async_trait(?Send)]
impl<T> ResetHandler for ResetHandlerImpl<T> {
    type Output = T;
    async fn shift<
        F: for<'a> FnOnce(Continuation<'a, Self::Output>) -> Continuation<'a, Self::Output> + 'static,
    >(
        self,
        f: F,
    ) {
        let mut sender = self.sender;
        sender.send(Box::new(|c| Box::pin(f(c)))).expect("WTF");
    }
}

impl<'a, T> ResetExecutor<'a, T> {
    pub fn new<Fut: Future<Output = T> + 'a>(
        f: impl 'static + for<'a> FnOnce(ResetHandlerImpl<T>) -> Fut,
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
            match Future::poll(
                self.continuation.as_mut(),
                &mut std::task::Context::from_waker(futures::task::noop_waker_ref()),
            ) {
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

async fn reset<T, Fut: Future<Output = T>>(f: impl FnOnce(ResetHandlerImpl<T>) -> Fut) -> T {
    ResetExecutor::new(f).run().await
}

async fn foo(ctx: impl ResetHandler<Output = ()>) {
    println!("begin");
    ctx.shift(|cc| {
        Box::pin(async move {
            cc.await;
            println!("hi");
        })
    })
    .await;
    println!("end");
}

async fn run() {
    reset(foo).await;
}

fn main() {}
