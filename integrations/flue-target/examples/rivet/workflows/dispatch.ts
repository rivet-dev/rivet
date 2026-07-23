import { defineWorkflow, dispatch } from '@flue/runtime';
import * as v from 'valibot';
import agent from '../agents/assistant.ts';

export const route = async (_c, next) => next();
export const runs = async (_c, next) => next();

export default defineWorkflow({
	agent,
	input: v.object({ source: v.string(), dispatchedInstanceId: v.string() }),
	async run({ input }) {
		const receipt = await dispatch({
			agent: 'assistant',
			id: input.dispatchedInstanceId,
			message: { kind: 'user', body: 'Hello from workflow' },
		});
		return { input, dispatchId: receipt.dispatchId };
	},
});
