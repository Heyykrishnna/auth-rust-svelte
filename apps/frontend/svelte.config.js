import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			out: 'build',
			precompress: true
		}),
		alias: {
			$lib: 'src/lib',
			$api: 'src/lib/api',
			$auth: 'src/lib/auth',
			$stores: 'src/lib/stores',
			$components: 'src/lib/components'
		}
	}
};

export default config;
