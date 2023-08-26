mod effects;
mod shift_reset;

use futures::executor::block_on;
use shift_reset::*;

async fn foo(ctx: impl ResetHandler<'_, Output = ()>) {
    println!("begin");
    ctx.shift(|cc| async move {
        cc.await;
        println!("hi");
    })
    .await;
    println!("end");
}

async fn run() {
    reset(foo).await;
}

fn main() {
    // This is my code - It's your probem now - I'm out
    block_on(effects::main());
}
