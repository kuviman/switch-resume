fn main() {
    futures::executor::block_on(async {
        switch_resume::run(|task| async move {
            println!("begin");
            task.switch(|resume| async move {
                println!("before");
                resume(()).await;
                println!("after");
            })
            .await;
            println!("end");
        })
        .await;
    });
    // begin
    // before
    // end
    // after
}
