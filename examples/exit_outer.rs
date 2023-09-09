use futures::prelude::*;

fn main() {
    futures::executor::block_on(async {
        let result: Result<i32, &str> = switch_resume::run(|task| async move {
            let task = &task;
            let sum = futures::stream::iter([1, 2, 3])
                .then(|x| async move {
                    if x % 2 == 0 {
                        // return Err from that task
                        let _: () = task
                            .switch(|_resume| async move { Err("There was an even number!") })
                            .await;
                    }
                    x
                })
                .fold(0, |acc, x| async move { acc + x })
                .await;
            // this will not be executed, since we stopped the task
            // by returning Err from it when encountered 2
            Ok(sum)
        })
        .await;
        assert!(result.is_err());
    });
}
