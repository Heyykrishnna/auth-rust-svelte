import { describe, expect, it } from 'vitest';
import { controlApi } from './control';

const workspace = { name: 'Example Workspace', slug: 'example-workspace' };

describe('Developer Console API errors', () => {
	it('shows an HTTP error instead of reading a non-JSON response twice', async () => {
		const response = new Response('<!doctype html><title>Not Found</title>', {
			status: 404,
			statusText: 'Not Found',
			headers: { 'Content-Type': 'text/html' }
		});

		await expect(
			controlApi.createWorkspace(workspace, { customFetch: async () => response })
		).rejects.toMatchObject({ name: 'ApiError', status: 404, message: 'HTTP 404 Not Found' });
	});

	it('keeps the message from a plain-text rejection', async () => {
		const response = new Response('Too many requests', { status: 429 });

		await expect(
			controlApi.createWorkspace(workspace, { customFetch: async () => response })
		).rejects.toMatchObject({ name: 'ApiError', status: 429, message: 'Too many requests' });
	});

	it('preserves structured API errors and their codes', async () => {
		const response = Response.json(
			{ error: { code: 'CSRF_TOKEN_INVALID', message: 'Missing or invalid CSRF token' } },
			{ status: 403 }
		);

		await expect(
			controlApi.createWorkspace(workspace, { customFetch: async () => response })
		).rejects.toMatchObject({
			name: 'ApiError',
			status: 403,
			code: 'CSRF_TOKEN_INVALID',
			message: 'Missing or invalid CSRF token'
		});
	});
});
