pub struct RunTaskJsonOutput {
	pub success: bool,
}

/// Used to auto-generate code for each task in order to dynamically dispatch.
///
/// Used to run tasks with raw input/output string. This is useful for binding tasks to non-Rust
/// environments, such as raw dylibs or odd engines.
#[macro_export]
macro_rules! task_registry {
    ( $( $task:ty ),* $(,)? ) => {
        pub async fn run_task_json(run_config: $crate::util::task::RunConfig, name: &str, input_json: &str) -> $crate::util::task::RunTaskJsonOutput {
            $(
                if name == <$task as $crate::util::task::Task>::name() {
                    let input = serde_json::from_str::<<$task as $crate::util::task::Task>::Input>(&input_json)
                        .expect("deserialize task input");
                    let output = $crate::util::task::run_task::<$task>(run_config, input).await;
                    let success = output.is_ok();
                    return $crate::util::task::RunTaskJsonOutput {
                        success,
                    };
                }
            )*

            panic!("unknown task {name}")
        }
    };
}
