import { createAgent, registerProvider } from '@flue/runtime';
import { fauxAssistantMessage, registerFauxProvider } from '@flue/runtime/adapter-kit';

const provider = registerFauxProvider({ provider: 'rivet-example' });
provider.setResponses([
	fauxAssistantMessage('Hello from Rivet.'),
	fauxAssistantMessage('Hello from workflow dispatch.'),
]);
const model = provider.getModel();
registerProvider(model.provider, { api: provider.api, baseUrl: model.baseUrl });

export const route = async (_c, next) => next();

export default createAgent(() => ({
	model: `${model.provider}/${model.id}`,
}));
