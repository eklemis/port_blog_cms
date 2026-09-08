import adapter from '@sveltejs/adapter-auto';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Runes are mandatory project-wide. This turns Svelte 4 syntax — `export let`,
	// `$:`, stores in components — into a COMPILE ERROR rather than a convention
	// an agent can drift from. It is the strongest enforcement available here.
	compilerOptions: {
		runes: true
	},
	vitePlugin: {
		/**
		 * Runes stay mandatory for everything in `src`. They cannot be mandatory
		 * for `node_modules`: `lucide-svelte` is a Svelte 4 library that uses
		 * `$$props`, which is a compile error under runes, and it is the icon set
		 * the Figma components name by import. Without this the console cannot
		 * draw a single icon.
		 *
		 * This narrows the rule to the code the rule is about. The longer-term fix
		 * is `@lucide/svelte`, the Svelte 5 package — a dependency change, so it is
		 * someone's call rather than mine.
		 */
		dynamicCompileOptions({ filename }) {
			if (filename.includes('node_modules')) return { runes: false };
		}
	},
	kit: {
		// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
		// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
		// See https://svelte.dev/docs/kit/adapters for more information about adapters.
		adapter: adapter(),
		files: {
			routes: 'src/app/routes', // move routing inside the app layer
			lib: 'src',
			appTemplate: 'src/app/app.html', // Move the application entry point inside the app layer
			assets: 'public'
		},
		alias: {
			'@/*': 'src/*' // Create an alias for the src directory
		}
	}
};

export default config;
