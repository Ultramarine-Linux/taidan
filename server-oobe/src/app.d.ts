// See https://svelte.dev/docs/kit/types#app.d.ts
declare global {
	namespace App {
		interface Locals {
			/** Tetra setup session token, set by hooks.server.ts for /api routes. */
			setupToken?: string;
		}
	}
}

export {};
