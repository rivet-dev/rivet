use gas::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateTestInput {
	pub initial_value: i32,
}

#[workflow(StateTestWorkflow)]
pub async fn state_test_workflow(ctx: &mut WorkflowCtx, input: &StateTestInput) -> Result<i32> {
	// First activity sets state
	ctx.activity(SetStateActivityInput {
		value: input.initial_value,
	})
	.await?;

	// Second activity reads state
	let result = ctx.activity(GetStateActivityInput {}).await?;

	assert_eq!(result, input.initial_value);

	Ok(result)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct SetStateActivityInput {
	pub value: i32,
}

#[activity(SetStateActivity)]
pub async fn set_state_activity(ctx: &ActivityCtx, input: &SetStateActivityInput) -> Result<()> {
	let mut state = ctx.state::<Option<TestState>>()?;

	// Initialize state if it doesn't exist
	if state.is_none() {
		*state = Some(TestState::default());
	}

	// Update the value
	if let Some(ref mut s) = *state {
		s.value = input.value;
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct GetStateActivityInput {}

#[activity(GetStateActivity)]
pub async fn get_state_activity(ctx: &ActivityCtx, _input: &GetStateActivityInput) -> Result<i32> {
	let state = ctx.state::<Option<TestState>>()?;

	if let Some(s) = state.as_ref() {
		Ok(s.value)
	} else {
		// Return default value if state not initialized
		Ok(TestState::default().value)
	}
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TestState {
	pub value: i32,
}
