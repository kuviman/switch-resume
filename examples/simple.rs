#[tokio::main]
async fn main() {
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
    println!("outside")
}
