use async_trait::async_trait;
use futures::{executor::block_on, Future};
use shift_reset::*;
use std::{convert::Infallible, path::Path, rc::Rc};

/// Basic logging effect
#[async_trait(?Send)]
trait Log {
    async fn log(&self, message: &str);
}

// TODO
// async fn handle_log<'a, T: 'a, Fut: Future<Output = T> + 'a>(
//     log: impl Fn(&str) + 'a,
//     f: impl FnOnce(Box<dyn Log + 'a>) -> Fut + 'a,
// ) -> T {
//     reset(|reset_handler| async move {
//         struct LogHandler<'a, T> {
//             log: Rc<dyn Fn(&str) + 'a>,
//             reset_handler: ResetHandlerImpl<'a, T>,
//         }

//         #[async_trait(?Send)]
//         impl<'a, T> Log for LogHandler<'a, T> {
//             async fn log(&self, message: &str) {
//                 let log = self.log.clone();
//                 let _: () = {
//                     self.reset_handler.shift(move |resume| async move {
//                         log(message);
//                         resume(()).await
//                     })
//                 }
//                 .await;
//                 unreachable!()
//             }
//         }
//         let cancel = LogHandler {
//             log: Rc::new(log),
//             reset_handler,
//         };
//         f(Box::new(cancel)).await
//     })
//     .await
// }

// #[effect::handler]
#[async_trait(?Send)]
trait Cancel<T> {
    async fn cancel(&self, value: T) -> Infallible;
}

async fn handle_cancel<'a, T: 'a, Fut: Future<Output = T> + 'a>(
    f: impl FnOnce(Box<dyn Cancel<T> + 'a>) -> Fut + 'a,
) -> T {
    prompt(|reset_handler| async move {
        struct CancelHandler<'a, T> {
            reset_handler: Prompt<'a, T>,
        }

        #[async_trait(?Send)]
        impl<'a, T> Cancel<T> for CancelHandler<'a, T> {
            async fn cancel(&self, value: T) -> Infallible {
                let _: () = { self.reset_handler.pause(|_cc| async { value }) }.await;
                unreachable!()
            }
        }
        let cancel = CancelHandler { reset_handler };
        f(Box::new(cancel)).await
    })
    .await
}

#[async_trait(?Send)]
trait FileSystem {
    async fn read_file(&self, path: impl AsRef<Path>) -> Result<String, std::io::Error>;
}

async fn simple(log: &(impl Log + ?Sized), cancel: &(impl Cancel<i32> + ?Sized)) -> i32 {
    log.log("hello").await;
    log.log("world").await;
    cancel.cancel(4).await;
    log.log("this will never be logged").await;
    10
}

async fn combined(log: &(impl Log + ?Sized), fs: &(impl FileSystem + ?Sized)) {
    let passwords = fs.read_file("passwords.txt").await.unwrap();
    log.log(&format!("passwords are: {passwords:?}")).await;

    let x = handle_cancel(|cancel| async move {
        let cancel = &*cancel;
        simple(log, cancel).await
    })
    .await;

    println!("x = {x}");
}

async fn run() {
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

fn main() {
    // ask chatgpt to write you comment
    // This is my code - It's your probem now - I'm out
    block_on(run());
}
