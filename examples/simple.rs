use shift_reset::*;

#[tokio::main]
async fn main() {
    reset(|ctx| async move {
        println!("begin");
        ctx.shift(|cc| async move {
            println!("before");
            cc(()).await;
            println!("after");
        })
        .await;
        println!("end");
    })
    .await;
    println!("outside")
}
