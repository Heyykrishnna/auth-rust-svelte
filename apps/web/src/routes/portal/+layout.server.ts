import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ setHeaders }) => {
	setHeaders({
		'Content-Security-Policy': "frame-ancestors 'none'",
		'X-Frame-Options': 'DENY',
		'Referrer-Policy': 'no-referrer'
	});
	return {};
};

