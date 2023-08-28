use futures::{future::LocalBoxFuture, prelude::*};
use shift_reset::*;

#[derive(Debug)]
enum Action {
    Deeper,
    Back,
}

async fn transition<T>(desc: String, into: impl Future<Output = T>) -> T {
    let transition = async {
        for _ in 0..10 {
            println!("Transition: {desc}");
            tokio::time::sleep(tokio::time::Duration::from_secs_f64(0.1)).await;
        }
    };
    let transition = std::pin::pin!(transition);
    let into = std::pin::pin!(into);
    match futures::future::select(transition, into).await {
        future::Either::Left(((), into)) => into.await,
        future::Either::Right((value, transition)) => {
            #[allow(clippy::let_underscore_future)]
            let _ = transition; // Dont show the rest of transition
            value
        }
    }
}

async fn enter_impl<'a>(
    ctx: &'a ResetHandlerImpl<'_, ()>,
    actions: &'a mut dyn Iterator<Item = Action>,
    depth: u16,
) {
    println!("You have now entered depth {depth}");
    while let Some(Action::Deeper) = actions.next() {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        transition(
            format!("entering {}", depth + 1),
            enter(ctx, actions, depth + 1),
        )
        .await;
        println!("Now back to depth {depth}");
        ctx.shift(move |cc| transition(format!("back to {depth}"), cc(())))
            .await;
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    println!("You have now exited depth {depth}");
}

fn enter<'a>(
    ctx: &'a ResetHandlerImpl<'_, ()>,
    actions: &'a mut dyn Iterator<Item = Action>,
    depth: u16,
) -> LocalBoxFuture<'a, ()> {
    enter_impl(ctx, actions, depth).boxed_local()
}

#[tokio::main]
async fn main() {
    reset(|ctx| async move {
        enter(
            &ctx,
            &mut [Action::Deeper, Action::Deeper, Action::Back, Action::Deeper]
                .into_iter()
                .inspect(|action| println!("Performing {action:?}")),
            0,
        )
        .await
    })
    .await;
}
