import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		sveltekit({
			adapter: adapter({ out: 'build', precompress: true }),
			alias: {
				$lib: 'src/lib',
				$api: 'src/lib/api',
				$auth: 'src/lib/auth',
				$stores: 'src/lib/stores',
				$components: 'src/lib/components'
			}
		})
	],
	css: {
		postcss: {}
	},
	server: {
		port: 3000,
		proxy: {
			'/api': {
				target: process.env.PUBLIC_API_BASE_URL || 'http://localhost:8080',
				changeOrigin: true
			}
		}
	}
});
