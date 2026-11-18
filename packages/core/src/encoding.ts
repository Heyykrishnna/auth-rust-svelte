import type { EcPublicJwk } from './types.js';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export function utf8ToBytes(value: string): Uint8Array {
	return encoder.encode(value);
}

export function bytesToUtf8(bytes: Uint8Array): string {
	return decoder.decode(bytes);
}

export function bytesToHex(bytes: Uint8Array): string {
	let out = '';
	for (let i = 0; i < bytes.length; i += 1) {
		const b = bytes[i] ?? 0;
		out += b.toString(16).padStart(2, '0');
	}
	return out;
}

export function bytesToBase64Url(bytes: Uint8Array): string {
	let binary = '';
	for (let i = 0; i < bytes.length; i += 1) {
		binary += String.fromCharCode(bytes[i] ?? 0);
	}
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/g, '');
}

export function base64UrlToBytes(input: string): Uint8Array {
	const normalized = input.replace(/-/g, '+').replace(/_/g, '/');
	const padLen = (4 - (normalized.length % 4)) % 4;
	const padded = normalized + '='.repeat(padLen);
	const binary = atob(padded);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i += 1) {
		bytes[i] = binary.charCodeAt(i);
	}
	return bytes;
}

export function constantTimeEqual(a: Uint8Array, b: Uint8Array): boolean {
	if (a.length !== b.length) {
		return false;
	}
	let diff = 0;
	for (let i = 0; i < a.length; i += 1) {
		diff |= (a[i] ?? 0) ^ (b[i] ?? 0);
	}
	return diff === 0;
}

function getWebCrypto(): Crypto {
	const c = globalThis.crypto;
	if (!c || !c.subtle) {
		throw new Error('Web Crypto API is unavailable in this runtime');
	}
	return c;
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
	const copy = new Uint8Array(bytes.byteLength);
	copy.set(bytes);
	return copy.buffer;
}

export function randomBase64Url(numBytes = 32): string {
	const c = getWebCrypto();
	const buf = new Uint8Array(numBytes);
	c.getRandomValues(buf);
	return bytesToBase64Url(buf);
}

export async function sha256Base64Url(value: string): Promise<string> {
	const c = getWebCrypto();
	const digest = await c.subtle.digest('SHA-256', toArrayBuffer(utf8ToBytes(value)));
	return bytesToBase64Url(new Uint8Array(digest));
}

export async function hmacSha256Hex(secret: string, data: Uint8Array): Promise<string> {
	const c = getWebCrypto();
	const key = await c.subtle.importKey(
		'raw',
		toArrayBuffer(utf8ToBytes(secret)),
		{ name: 'HMAC', hash: 'SHA-256' },
		false,
		['sign']
	);
	const sig = await c.subtle.sign('HMAC', key, toArrayBuffer(data));
	return bytesToHex(new Uint8Array(sig));
}

export async function verifyEs256SignatureWithJwk(
	jwk: EcPublicJwk,
	signingInput: string,
	signatureBytes: Uint8Array
): Promise<boolean> {
	const c = getWebCrypto();
	const publicKey = await c.subtle.importKey(
		'jwk',
		{
			kty: jwk.kty,
			crv: jwk.crv,
			x: jwk.x,
			y: jwk.y,
			ext: true
		},
		{ name: 'ECDSA', namedCurve: 'P-256' },
		false,
		['verify']
	);

	return c.subtle.verify(
		{ name: 'ECDSA', hash: 'SHA-256' },
		publicKey,
		toArrayBuffer(signatureBytes),
		toArrayBuffer(utf8ToBytes(signingInput))
	);
}

