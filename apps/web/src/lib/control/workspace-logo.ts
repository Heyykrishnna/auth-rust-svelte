export const workspaceLogoAccept = 'image/png,image/jpeg,image/webp';

const maxSourceBytes = 5 * 1024 * 1024;
const maxEncodedLength = 60_000;
const maxLogoEdge = 192;

export async function prepareWorkspaceLogo(file: File): Promise<string> {
	if (!workspaceLogoAccept.split(',').includes(file.type)) {
		throw new Error('Choose a PNG, JPEG, or WebP image.');
	}
	if (file.size > maxSourceBytes) {
		throw new Error('Choose an image that is 5 MB or smaller.');
	}

	const bitmap = await createImageBitmap(file).catch(() => {
		throw new Error('This image could not be opened. Choose another file.');
	});
	try {
		const canvas = document.createElement('canvas');
		const scale = Math.min(
			1,
			maxLogoEdge / Math.max(bitmap.width, bitmap.height)
		);
		canvas.width = Math.max(1, Math.round(bitmap.width * scale));
		canvas.height = Math.max(1, Math.round(bitmap.height * scale));
		const context = canvas.getContext('2d');
		if (!context) throw new Error('This browser could not prepare your image.');
		context.fillStyle = '#181818';
		context.fillRect(0, 0, canvas.width, canvas.height);
		context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);

		for (const quality of [0.88, 0.72, 0.5]) {
			const jpeg = canvas.toDataURL('image/jpeg', quality);
			const encoded = jpeg.slice('data:image/jpeg;base64,'.length);
			if (
				jpeg.startsWith('data:image/jpeg;base64,') &&
				encoded.length <= maxEncodedLength
			) {
				return encoded;
			}
		}
		throw new Error(
			'This image could not be made small enough. Choose another file.'
		);
	} finally {
		bitmap.close();
	}
}
