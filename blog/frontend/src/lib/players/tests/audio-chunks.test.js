import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	ChunkedAudio,
	SUPPORTED_VERSION,
	chunkAtOffset,
	loadChunk,
	offsetForTime,
	prefixThrough,
	readChunkTable,
	sha256Hex
} from '../audio-chunks.js';

/**
 * A real zstd frame, produced by `zstd -19`, holding the bytes below. Real
 * rather than injected on purpose: the decode that ships is `fzstd`, and a
 * fake decoder would let a broken frame path pass this suite.
 */
const RAW = 'ID3-chunk-zero-of-a-chapter-the-client-decompresses';
const FRAME = Buffer.from(
	'KLUv/QRomQEASUQzLWNodW5rLXplcm8tb2YtYS1jaGFwdGVyLXRoZS1jbGllbnQtZGVjb21wcmVzc2VzO/k7lg==',
	'base64'
);
const RAW_SHA = 'daa8831d6f73b992c5ec7702bd7fc95038937f6138ba175f9ac485f36f953480';
const RAW_BYTES = new TextEncoder().encode(RAW);

/** A table of `count` chunks, every one of them the same real frame. */
function tableWith(count, { sha = true, rawSize } = {}) {
	return readChunkTable({
		version: SUPPORTED_VERSION,
		chunk_size: RAW_BYTES.length,
		raw_size: rawSize ?? RAW_BYTES.length * count,
		content_type: 'audio/mpeg',
		chunks: Array.from({ length: count }, (_, index) => ({
			index,
			offset: index * RAW_BYTES.length,
			raw: RAW_BYTES.length,
			compressed: FRAME.length,
			sha256: sha ? RAW_SHA : null,
			url: `/media/i/chapter/chunk/${index}`
		}))
	});
}

function fakeFetch(calls, { ok = true, status = 200 } = {}) {
	return async (url) => {
		calls.push(url);
		return { ok, status, arrayBuffer: async () => FRAME };
	};
}

test('the hash of the known bytes is the hash the table carries', async () => {
	// Guards the rest of the suite: if this drifts, every hash assertion below
	// would be checking something else.
	assert.equal(await sha256Hex(RAW_BYTES), RAW_SHA);
});

test('a real zstd frame decodes to the bytes the table promised', async () => {
	const table = tableWith(1);
	const raw = await loadChunk(table, 0, { fetchImpl: fakeFetch([]) });

	assert.equal(new TextDecoder().decode(raw), RAW);
	assert.equal(raw.length, RAW_BYTES.length);
});

test('a chunk that decodes to the wrong length is refused', async () => {
	// The table claiming the wrong size is the shape a truncated write takes.
	const table = tableWith(1, { rawSize: RAW_BYTES.length, sha: false });
	table.chunks[0].raw = RAW_BYTES.length + 1;

	await assert.rejects(
		() => loadChunk(table, 0, { fetchImpl: fakeFetch([]) }),
		/decoded to .* the table says/
	);
});

test('a chunk whose bytes do not match its hash is refused', async () => {
	const table = tableWith(1);
	table.chunks[0].sha256 = 'f'.repeat(64);

	await assert.rejects(
		() => loadChunk(table, 0, { fetchImpl: fakeFetch([]) }),
		/do not match its hash/
	);
});

test('a failed fetch is an error rather than a short chapter', async () => {
	const table = tableWith(2);
	await assert.rejects(
		() => loadChunk(table, 0, { fetchImpl: fakeFetch([], { ok: false, status: 403 }) }),
		/could not be fetched \(403\)/
	);
});

test('the table is refused when its offsets do not line up with its lengths', () => {
	// The property a seek depends on. A table whose second chunk claims to
	// start where the first one ends is usable; one that claims otherwise must
	// not be, because every seek would land somewhere the listener did not ask
	// for.
	const good = tableWith(2);
	assert.ok(good);
	assert.deepEqual(
		good.chunks.map((chunk) => chunk.offset),
		[0, RAW_BYTES.length]
	);

	const shifted = {
		version: SUPPORTED_VERSION,
		chunk_size: RAW_BYTES.length,
		raw_size: RAW_BYTES.length * 2,
		chunks: [
			{ index: 0, offset: 0, raw: RAW_BYTES.length, compressed: FRAME.length, url: '/a' },
			// Starts 100 bytes late.
			{ index: 1, offset: 100, raw: RAW_BYTES.length, compressed: FRAME.length, url: '/b' }
		]
	};
	assert.equal(readChunkTable(shifted), null);
});

test('an unusable table reads as absent rather than throwing', () => {
	const base = {
		version: SUPPORTED_VERSION,
		chunk_size: RAW_BYTES.length,
		raw_size: RAW_BYTES.length,
		chunks: [{ index: 0, offset: 0, raw: RAW_BYTES.length, compressed: FRAME.length, url: '/a' }]
	};

	assert.ok(readChunkTable(base));
	assert.equal(readChunkTable(null), null);
	assert.equal(readChunkTable('not json'), null);
	assert.equal(readChunkTable({ ...base, version: 99 }), null);
	assert.equal(readChunkTable({ ...base, chunks: [] }), null);
	assert.equal(readChunkTable({ ...base, raw_size: 0 }), null);
	// Lengths that do not add up to the chapter.
	assert.equal(readChunkTable({ ...base, raw_size: RAW_BYTES.length + 1 }), null);
	// A chunk with nowhere to fetch it from.
	assert.equal(readChunkTable({ ...base, chunks: [{ ...base.chunks[0], url: '' }] }), null);
	// Out of order.
	assert.equal(
		readChunkTable({
			...base,
			chunks: [{ ...base.chunks[0], index: 1 }]
		}),
		null
	);
});

test('a position maps onto an offset, clamped to the chapter', () => {
	const rawSize = 1000;

	assert.equal(offsetForTime(0, 100, rawSize), 0);
	assert.equal(offsetForTime(50, 100, rawSize), 500);
	assert.equal(offsetForTime(99.9, 100, rawSize), 999);
	assert.equal(offsetForTime(100, 100, rawSize), 999, 'the end cannot fall past the chapter');
	assert.equal(offsetForTime(1000, 100, rawSize), 999);
	// A duration the player does not know yet means the start, not a division
	// by zero or a NaN offset.
	assert.equal(offsetForTime(30, 0, rawSize), 0);
	assert.equal(offsetForTime(30, undefined, rawSize), 0);
	assert.equal(offsetForTime(NaN, 100, rawSize), 0);
	assert.equal(offsetForTime(-5, 100, rawSize), 0);
});

test('an offset resolves to its chunk, and past the end to the last one', () => {
	const table = tableWith(3);
	const size = RAW_BYTES.length;

	assert.equal(chunkAtOffset(table, 0), 0);
	assert.equal(chunkAtOffset(table, size - 1), 0);
	assert.equal(chunkAtOffset(table, size), 1);
	assert.equal(chunkAtOffset(table, size * 2 - 1), 1);
	assert.equal(chunkAtOffset(table, size * 2), 2);
	assert.equal(chunkAtOffset(table, size * 3 - 1), 2);
	assert.equal(chunkAtOffset(table, size * 99), 2, 'past the end clamps to the last chunk');
});

test('the prefix a seek needs always starts at the first chunk', () => {
	const table = tableWith(4);
	const size = RAW_BYTES.length;

	// A blob that began mid-chapter would report its own timeline rather than
	// the chapter's, so the prefix never starts anywhere but zero.
	assert.deepEqual(prefixThrough(table, 0), [0]);
	assert.deepEqual(prefixThrough(table, size * 2), [0, 1, 2]);
	assert.deepEqual(prefixThrough(table, size * 4 - 1), [0, 1, 2, 3]);
});

test('decompression is per chunk: only the prefix up to the position is fetched', async () => {
	const table = tableWith(4);
	const calls = [];
	const audio = new ChunkedAudio(table, { fetchImpl: fakeFetch(calls) });

	// 30% of the way in lands in the second chunk of four.
	await audio.loadThroughTime(30, 100);

	assert.deepEqual(calls, [table.chunks[0].url, table.chunks[1].url]);
	assert.equal(audio.loadedBytes, RAW_BYTES.length * 2);
	assert.equal(audio.complete, false);
});

test('a chunk already loaded is never fetched twice', async () => {
	const table = tableWith(3);
	const calls = [];
	const audio = new ChunkedAudio(table, { fetchImpl: fakeFetch(calls) });

	await audio.loadThrough(1);
	await audio.loadThrough(1);
	await audio.loadThrough(2);

	assert.deepEqual(calls, [table.chunks[0].url, table.chunks[1].url, table.chunks[2].url]);
	assert.equal(audio.fetches, 3);
	assert.equal(audio.complete, true);
});

test('overlapping loads cannot fetch the same chunk twice', async () => {
	// A seek landing while the chapter's first load is still running is the
	// ordinary case, not an edge one.
	const table = tableWith(3);
	const calls = [];
	const audio = new ChunkedAudio(table, { fetchImpl: fakeFetch(calls) });

	await Promise.all([audio.loadThrough(2), audio.loadThrough(2), audio.loadThroughTime(50, 100)]);

	assert.deepEqual(calls, [table.chunks[0].url, table.chunks[1].url, table.chunks[2].url]);
	assert.equal(audio.loadedBytes, RAW_BYTES.length * 3);
});

test('a chunk that fails leaves the load rejected rather than quietly short', async () => {
	const table = tableWith(3);
	let served = 0;
	const audio = new ChunkedAudio(table, {
		fetchImpl: async (url) => {
			served += 1;
			if (served === 2) return { ok: false, status: 500, arrayBuffer: async () => FRAME };
			return { ok: true, status: 200, arrayBuffer: async () => FRAME };
		}
	});

	await assert.rejects(() => audio.loadThrough(2), /could not be fetched \(500\)/);
	assert.equal(audio.complete, false, 'a refused chunk must not count as loaded');
});

test('the loaded prefix becomes a blob of the chapter bytes', async (t) => {
	if (typeof URL.createObjectURL !== 'function') {
		t.skip('this runtime has no URL.createObjectURL');
		return;
	}

	const table = tableWith(2);
	const audio = new ChunkedAudio(table, { fetchImpl: fakeFetch([]) });

	assert.equal(audio.toBlobUrl(), null, 'an empty blob has no audio in it');

	await audio.loadThrough(1);
	const url = audio.toBlobUrl();
	assert.match(url, /^blob:/);

	const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
	assert.equal(new TextDecoder().decode(bytes), RAW + RAW, 'the prefix is the chapter from byte 0');
	URL.revokeObjectURL(url);
});
