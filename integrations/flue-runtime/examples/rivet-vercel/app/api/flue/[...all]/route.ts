import { toFlueNextHandler } from '@rivet-dev/flue/next';
import { flueApp } from '../../../../dist/server.mjs';

export const maxDuration = 300;

export const { GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS } = toFlueNextHandler(flueApp);
