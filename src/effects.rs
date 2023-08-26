use crate::shift_reset::*;
use async_trait::async_trait;
use std::{cell::RefCell, convert::Infallible, path::Path};

#[async_trait(?Send)]
trait Log {
    async fn log(&self, message: &str);
}

// #[effect::handler]
#[async_trait(?Send)]
trait Cancel<T> {
    async fn cancel(&self, value: T) -> Infallible;
}

#[async_trait(?Send)]
trait FileSystem {
    async fn read_file(&self, path: impl AsRef<Path>) -> Result<String, std::io::Error>;
}

async fn simple(ctx: &(impl Log + Cancel<i32>)) -> i32 {
    ctx.log("hello").await;
    ctx.log("world").await;
    ctx.cancel(4).await;
    ctx.log("this will never be logged").await;
    10
}

fn extend_context_with_cancel<'a, T: 'a>(
    ctx: &'a (impl Log + FileSystem),
    cancel: &'a impl Cancel<T>,
) -> impl Log + FileSystem + Cancel<T> + 'a {
    struct ContextWithCancel<'a, Inner, T> {
        inner: &'a Inner,
        cancel: &'a T,
    }

    #[async_trait(?Send)]
    impl<T, Inner: Log> Log for ContextWithCancel<'_, Inner, T> {
        async fn log(&self, message: &str) {
            self.inner.log(message).await
        }
    }

    #[async_trait(?Send)]
    impl<Inner: FileSystem, T> FileSystem for ContextWithCancel<'_, Inner, T> {
        async fn read_file(&self, path: impl AsRef<Path>) -> Result<String, std::io::Error> {
            self.inner.read_file(path).await
        }
    }

    #[async_trait(?Send)]
    impl<'a, T: 'a, C: Cancel<T>, Inner> Cancel<T> for ContextWithCancel<'a, Inner, C> {
        async fn cancel(&self, value: T) -> Infallible {
            self.cancel.cancel(value).await
        }
    }
    ContextWithCancel { inner: ctx, cancel }
}

async fn combined(ctx: &(impl Log + FileSystem)) {
    let passwords = ctx.read_file("passwords.txt").await.unwrap();
    ctx.log(&format!("passwords are: {passwords:?}")).await;

    let x: _ = reset(|reset_handler| async move {
        struct CancelHandler<'a, T> {
            reset_handler: RefCell<Option<ResetHandlerImpl<'a, T>>>,
        }

        #[async_trait(?Send)]
        impl<'a, T> Cancel<T> for CancelHandler<'a, T> {
            async fn cancel(&self, value: T) -> Infallible {
                self.reset_handler
                    .borrow_mut()
                    .take()
                    .unwrap()
                    .shift(|_cc| async { value })
                    .await;
                unreachable!()
            }
        }
        let cancel = CancelHandler {
            reset_handler: RefCell::new(Some(reset_handler)),
        };
        let ctx = extend_context_with_cancel(ctx, &cancel);
        let ctx = &ctx;

        simple(ctx).await
    })
    .await;
    dbg!(x);
}

pub async fn main() {
    struct Context;
    #[async_trait(?Send)]
    impl Log for Context {
        async fn log(&self, message: &str) {
            println!("{message}");
        }
    }
    #[async_trait(?Send)]
    impl FileSystem for Context {
        async fn read_file(&self, _path: impl AsRef<Path>) -> Result<String, std::io::Error> {
            Ok("hello, there!".to_owned())
        }
    }
    combined(&Context).await;
}
