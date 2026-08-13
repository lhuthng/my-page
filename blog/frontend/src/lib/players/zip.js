// Minimal ZIP reader over a File. Entries are read straight out of the file
// with slice(), so a 500 MB archive is never held in memory — and DEFLATE goes
// through the browser's own DecompressionStream, so there is no library.

const EOCD_SIG = 0x06054b50;
const EOCD64_SIG = 0x06064b50;
const EOCD64_LOCATOR_SIG = 0x07064b50;
const CENTRAL_SIG = 0x02014b50;

const slice = async (file, start, end) =>
	new Uint8Array(await file.slice(start, end).arrayBuffer());

export async function readZipDirectory(file) {
	// The EOCD is last, but a trailing comment can push it up to 64 KiB back.
	const tailLength = Math.min(file.size, 65557 + 20);
	const tail = await slice(file, file.size - tailLength, file.size);
	const view = new DataView(tail.buffer);

	let eocd = -1;
	for (let i = tail.length - 22; i >= 0; i--) {
		if (view.getUint32(i, true) === EOCD_SIG) {
			eocd = i;
			break;
		}
	}
	if (eocd < 0) throw new Error('That file is not a zip.');

	let count = view.getUint16(eocd + 10, true);
	let size = view.getUint32(eocd + 12, true);
	let offset = view.getUint32(eocd + 16, true);

	if (offset === 0xffffffff || size === 0xffffffff || count === 0xffff) {
		let locator = -1;
		for (let i = eocd - 20; i >= 0; i--) {
			if (view.getUint32(i, true) === EOCD64_LOCATOR_SIG) {
				locator = i;
				break;
			}
		}
		if (locator < 0) throw new Error('That zip looks damaged.');
		const at = Number(view.getBigUint64(locator + 8, true));
		const record = new DataView((await slice(file, at, at + 56)).buffer);
		if (record.getUint32(0, true) !== EOCD64_SIG) throw new Error('That zip looks damaged.');
		count = Number(record.getBigUint64(32, true));
		size = Number(record.getBigUint64(40, true));
		offset = Number(record.getBigUint64(48, true));
	}

	const central = await slice(file, offset, offset + size);
	const centralView = new DataView(central.buffer);
	const decoder = new TextDecoder('utf-8');
	const entries = [];
	let p = 0;

	for (let i = 0; i < count && p + 46 <= central.length; i++) {
		if (centralView.getUint32(p, true) !== CENTRAL_SIG) break;
		const flags = centralView.getUint16(p + 8, true);
		const method = centralView.getUint16(p + 10, true);
		let compressedSize = centralView.getUint32(p + 20, true);
		let uncompressedSize = centralView.getUint32(p + 24, true);
		const nameLength = centralView.getUint16(p + 28, true);
		const extraLength = centralView.getUint16(p + 30, true);
		const commentLength = centralView.getUint16(p + 32, true);
		let localOffset = centralView.getUint32(p + 42, true);
		const name = decoder.decode(central.subarray(p + 46, p + 46 + nameLength));

		if (
			uncompressedSize === 0xffffffff ||
			compressedSize === 0xffffffff ||
			localOffset === 0xffffffff
		) {
			const extra = central.subarray(p + 46 + nameLength, p + 46 + nameLength + extraLength);
			const extraView = new DataView(extra.buffer, extra.byteOffset, extra.byteLength);
			for (let q = 0; q + 4 <= extra.length; ) {
				const tag = extraView.getUint16(q, true);
				const length = extraView.getUint16(q + 2, true);
				if (tag === 0x0001) {
					let r = q + 4;
					if (uncompressedSize === 0xffffffff) {
						uncompressedSize = Number(extraView.getBigUint64(r, true));
						r += 8;
					}
					if (compressedSize === 0xffffffff) {
						compressedSize = Number(extraView.getBigUint64(r, true));
						r += 8;
					}
					if (localOffset === 0xffffffff) localOffset = Number(extraView.getBigUint64(r, true));
					break;
				}
				q += 4 + length;
			}
		}

		entries.push({
			name,
			size: uncompressedSize,
			compressedSize,
			method,
			localOffset,
			encrypted: (flags & 1) !== 0
		});
		p += 46 + nameLength + extraLength + commentLength;
	}

	return entries;
}

export async function readZipEntry(file, entry) {
	// The central directory's extra field can differ in length from the local
	// one, so the data offset has to come from the local header.
	const header = await slice(file, entry.localOffset, entry.localOffset + 30);
	const view = new DataView(header.buffer);
	const start = entry.localOffset + 30 + view.getUint16(26, true) + view.getUint16(28, true);
	const blob = file.slice(start, start + entry.compressedSize);

	if (entry.method === 0) return new Uint8Array(await blob.arrayBuffer());
	if (entry.method !== 8) {
		throw new Error(
			`"${entry.name}" uses a zip format this cannot read. Re-zip it normally.`
		);
	}
	const stream = blob.stream().pipeThrough(new DecompressionStream('deflate-raw'));
	return new Uint8Array(await new Response(stream).arrayBuffer());
}
