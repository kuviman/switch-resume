# Delimited async continuations

[switch-resume](https://crates.io/crates/switch-resume)
is an experimental crate I have released recently which provides [delimited continuation](https://en.wikipedia.org/wiki/Delimited_continuation) functionality in async Rust.

If you are not familiar with the concept of continuations, here I'll try my best to explain what it is.

Here's an example:

```rs
let three = task.switch(|resume| async move {
    resume(1).await
}).await + 2;
assert_eq!(three, 3);
```

In this example, when we call `task.switch`, we are switching the current task to the provided async closure, which gets current continuation as its argument `resume`.
In this case resume is something like this:

```rs
let resume = |one| async move {
    let three = one + 2;
    assert_eq!(three, 3);
};
```

As you can see, the body of resume represents the code that is supposed to run after awaiting `task.switch`. This is why it is called a continuation.

Now, how much of the code that is running after `switch` are we talking about here?
This is what `task` is about.
It is a handle to the task that is currently being executed.
You can get this handle using `switch_resume::run` fn.
Here's a full example:

```rs
switch_resume::run(|task| async move {
    let three = task.switch(|resume| async move {
        resume(1).await
    }).await + 2;
    assert_eq!(three, 3);
})
.await;
```

Basically, `resume` becomes the code between `task.switch` and the end of the future supplied to `switch_resume::run`.
Thus the continuation is called delimited.

## Why

Why is this useful?
Originally I was looking into implementing an [algebraic effect system](https://en.wikipedia.org/wiki/Effect_system), which also is kinda hard to explain imo.
Here's links to [Koka](https://koka-lang.github.io/koka/doc/index.html) and [Ante](https://antelang.org/) programming languages that implement the functionality natively.
And here's some Rust libraries that tried the idea: [effing-mad](https://github.com/rosefromthedead/effing-mad), [eff](https://crates.io/crates/eff).

My understanding is that delimited continuations can be the first building block in implementing effect systems, so this is what I started with.

Right now, in Rust async/await is implemented on top of generators,
and this library is built on top of async/await.
But it could in theory be the other way around, and this functionality could be used to implement language features such as async/await, generators, returning from functions, breaking from loops, throwing exceptions, etc.

But since we already have most of those in Rust, it has been hard for me to come up with simple use cases, but still here's what I actually used it for myself:

## Game state transitions

I am currently actively working on a game called **linksider**, originally [made for bevy jam #3](https://kuviman.itch.io/linksider), has since been rewritten into my own custom engine.

So, I have been using async functions for my state management like this:

```rs
async fn main_menu() {
    loop {
        if play_button_was_pressed() {
            play().await;
        }
    }
}
async fn play() {
    ...
}
```

Then, I wanted to have a transition between game states -
from main menu into the play state, and then after exiting the play state back into the main menu.

Going into the play state is kind of easy, we just call a `transition_into` fn with `play()` as its argument:

```rs
transition_into(play()).await;
```

Here, `transition_into` is running the `play()` game state future at the same time as rendering the transition effect, allowing me to interact with the state I transitioned into like if no visual effect was applied. It looks something like this:

```rs
async fn transition_into<T>(f: impl Future<Output = T>) -> T {
    let transition_vfx = async {
        // Render the transition visual effect
        ...
    };
    match select(transition_vfx, f).await {
        Left(((), f)) => {
            // Transition effect finished,
            // continue without visual effect
            f.await
        }
        Right((result, _transition_vfx)) => {
            // State we transitioned into exited before
            // visual effect finished,
            // don't show the rest of transition effect
            result
        }
    }
}
```

But in order to go back into the main menu, we need to transition into the continuation of the `main_menu` fn.

So in order to get that continuation, we use the `switch_resume` crate:

```rs
main_menu.switch(|resume| async move {
    transition_into(resume()).await;
}).await;
```

I have omitted a lot of details here, but that is in general how it actually works now in the game.
