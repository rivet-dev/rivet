import { toNextHandler } from '@rivetkit/next-js';
import { registry } from '../../../../dist/server.mjs';

export const maxDuration = 300;

registry.config.runtime = 'native';
const handlers = toNextHandler(registry);
if (registry.config.configurePool) {
	registry.config.configurePool.drainGracePeriod = 299;
	registry.config.configurePool.name = process.env.RIVET_POOL || registry.config.configurePool.name;
}

export const { GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS } = handlers;
