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

async fn simple(log: &impl Log, cancel: &impl Cancel<i32>) -> i32 {
    log.log("hello").await;
    log.log("world").await;
    cancel.cancel(4).await;
    log.log("this will never be logged").await;
    10
}

async fn combined(log: &impl Log, fs: &impl FileSystem) {
    let passwords = fs.read_file("passwords.txt").await.unwrap();
    log.log(&format!("passwords are: {passwords:?}")).await;

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
        simple(log, &cancel).await
    })
    .await;
    dbg!(x);
}

pub async fn main() {
    struct LogHandler;
    #[async_trait(?Send)]
    impl Log for LogHandler {
        async fn log(&self, message: &str) {
            println!("{message}");
        }
    }
    struct FileSystemHandler;
    #[async_trait(?Send)]
    impl FileSystem for FileSystemHandler {
        async fn read_file(&self, _path: impl AsRef<Path>) -> Result<String, std::io::Error> {
            Ok("hello, there!".to_owned())
        }
    }
    combined(&LogHandler, &FileSystemHandler).await;
}
