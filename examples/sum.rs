fn main() {
    futures::executor::block_on(async {
        switch_resume::run(|task| async move {
            let three = task.switch(|resume| async move { resume(2).await }).await + 1;
            assert_eq!(three, 3);
        })
        .await;
    });
}
