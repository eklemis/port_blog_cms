import adapter from '@sveltejs/adapter-auto';

/** @type {import('@sveltejs/kit').Config} */
const config = {
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
