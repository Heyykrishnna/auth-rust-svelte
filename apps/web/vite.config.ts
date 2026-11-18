import tailwindcss from '@tailwindcss/vite';
import adapter from '@sveltejs/adapter-node';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		sveltekit({
			csp: {
				mode: 'auto',
				directives: {
					'default-src': ['self'],
					'script-src': ['self'],
					'style-src': ['self', 'unsafe-inline'],
					'img-src': ['self', 'data:', 'https:'],
					'connect-src': ['self'],
					'object-src': ['none'],
					'base-uri': ['self'],
					'frame-ancestors': ['none'],
					'form-action': ['self']
				}
			},
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
