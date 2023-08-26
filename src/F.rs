use async_trait::async_trait;
use std::{convert::Infallible, future::Future, path::Path};

#[async_trait]
trait Log {
    async fn log(&self, message: &str);
}

// #[effect::handler]
#[async_trait]
trait Cancel {
    async fn cancel(&self) -> Infallible;
}

#[async_trait(?Send)]
trait CancelHandler<T> {
    async fn cancel<Fut: Future<Output = T>>(
        &self,
        continuation: impl FnOnce(Infallible) -> Fut,
    ) -> T;
}

#[async_trait]
trait FileRead {
    async fn read_file(&self, path: impl AsRef<Path>) -> Result<String, std::io::Error>;
}

async fn simple(ctx: &(impl Log + Cancel)) {
    ctx.log("hello").await;
    ctx.log("world").await;
    ctx.cancel().await;
    ctx.log("this will never be logged").await;
}

async fn combined(ctx: &(impl Log + FileRead)) {
    let passwords = ctx.read_file("passwords.txt").await.unwrap();
    ctx.log(&format!("passwords are: {passwords:?}")).await;

    struct ThisCancelHandler;

    #[async_trait(?Send)]
    impl CancelHandler<()> for ThisCancelHandler {
        async fn cancel<Fut: Future<Output = ()>>(
            &self,
            _continuation: impl FnOnce(Infallible) -> Fut,
        ) -> () {
            return ();
        }
    }
    call_cc();
    handle_cancel(handler, |ctx /* &(impl Log + FileRead + Cancel) */| async {
        simple(ctx).await;
    })
    .await
}

fn run_effects<T>(f: impl Future<Output = T>) -> T {
    todo!()
}

fn main() {
    // run_effects(combined(&ctx))
}
