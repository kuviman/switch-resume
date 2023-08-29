#[tokio::main]
async fn main() {
    pausible::run(|task| async move {
        println!("begin");
        task.pause(|resume| async move {
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
