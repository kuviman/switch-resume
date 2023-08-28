use shift_reset::*;

#[tokio::main]
async fn main() {
    prompt(|pause| async move {
        println!("begin");
        pause
            .pause(|resume| async move {
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
