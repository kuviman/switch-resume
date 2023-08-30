# switch-resume

This crates provides functionality for running switchable tasks (futures).

Switching is a control flow mechanism that pauses normal execution of current task (current function),
captures current task continuation and passes it as an argument to the provided async fn.
The task then proceeds by evaluating that fn, instead of resuming normally.

In order to resume normal execution, the passed resumption object can be called explicitly.

This is an implementation of [delimited continuations](https://en.m.wikipedia.org/wiki/Delimited_continuation) in Rust using async that works on stable.
