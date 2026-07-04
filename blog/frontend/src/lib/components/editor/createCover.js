export const allowedCreateCoverTypes = [
	'image/jpeg',
	'image/png',
	'image/gif',
	'image/webp',
	'video/mp4',
	'video/webm'
];

export const maxCreateCoverFileSize = 5 * 1024 * 1024;

export function selectCreateCover(file) {
	if (!file) {
		return {
			file: undefined,
			previewUrl: '',
			mediaType: '',
			ogImageSeconds: 0,
			error: ''
		};
	}

	if (!allowedCreateCoverTypes.includes(file.type)) {
		return {
			file: undefined,
			previewUrl: '',
			mediaType: '',
			ogImageSeconds: 0,
			error: 'Only JPEG, PNG, GIF, WEBP, MP4, or WebM are allowed.'
		};
	}

	if (file.size > maxCreateCoverFileSize) {
		return {
			file: undefined,
			previewUrl: '',
			mediaType: '',
			ogImageSeconds: 0,
			error: `File size exceeds 5MB (${file.size} bytes)`
		};
	}

	return {
		file,
		previewUrl: URL.createObjectURL(file),
		mediaType: file.type,
		ogImageSeconds: file.type.startsWith('video/') ? 1 : 0,
		error: ''
	};
}

export function appendCreateCover(formData, file, ogImageSeconds) {
	if (!file) return;
	formData.append('cover_file', file, file.name);
	if (file.type.startsWith('video/')) {
		formData.append('cover_og_image_seconds', ogImageSeconds.toString());
	}
}
