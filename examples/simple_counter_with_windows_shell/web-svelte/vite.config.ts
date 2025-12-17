import wasm from 'vite-plugin-wasm-esm';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit(), wasm(['shared'])]
});
