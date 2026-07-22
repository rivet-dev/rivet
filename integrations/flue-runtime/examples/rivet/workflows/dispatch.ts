import { dispatch } from '@flue/runtime';

export async function run(ctx) {
	const receipt = await dispatch({
		agent: 'assistant',
		id: ctx.payload.dispatchedInstanceId,
		input: { message: 'Hello from workflow' },
	});
	return { payload: ctx.payload, dispatchId: receipt.dispatchId };
}
