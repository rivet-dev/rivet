import { createAgent, registerProvider } from '@flue/runtime';
import { fauxAssistantMessage, registerFauxProvider } from '@flue/runtime/adapter-kit';

const provider = registerFauxProvider({ provider: 'rivet-vercel-example' });
provider.setResponses([fauxAssistantMessage('Hello from the Vercel route.')]);
const model = provider.getModel();
registerProvider(model.provider, { api: provider.api, baseUrl: model.baseUrl });

export default createAgent(() => ({
	model: `${model.provider}/${model.id}`,
}));
