# Effect System for Rust

## What is effect system?

Categories of effects:

### Tag effects

This is like a tag on a function, allowing it to be executed only in specific contexts, like:

- `unsafe`
- (`unchecked`)[https://sayan.blog/2022/02/rust-unchecked-keyword/]

### Unwind effects

Stop executing, instead unwind to the closest handler

- `break` from a loop
- `return` from a function
- Exceptions - `throw/catch`

### Linear effect

This is just a function call

### Delayed execution

Pause execution, and resume it at a later point in time:

- `await`

### Continuing execution multiple times

Not going to be covered

## My usecase



