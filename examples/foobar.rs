fn main() {
    futures::executor::block_on(async {
        async fn bar(resume: switch_resume::Resume<'_, i32, i32>) -> i32 {
            println!("foo has been paused, started bar");
            let resume_result = resume(69).await;
            assert_eq!(resume_result, -1); // This is the result of foo
            420 // This is the final result of task
        }
        async fn foo(task: switch_resume::Task<'_, i32>) -> i32 {
            println!("foo started");
            let value = task.switch(bar).await;
            println!("foo was resumed with {value}. Nice!");
            -1 // This is not the final task result since we switched to bar
        }
        let task_result = switch_resume::run(foo).await;
        assert_eq!(task_result, 420);
    });
}
