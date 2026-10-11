/**
 * The client half of chunked audiobook audio.
 *
 * The backend stores a chapter as independent zstd chunks plus a table saying
 * where each chunk's raw bytes begin (see `storage/chunked.rs`). This module
 * turns that table into something a player can use, and it exists as plain JS
 * with no DOM and no Svelte so the arithmetic can be tested directly.
 *
 * Three decisions shape it:
 *
 * **Decompression is per chunk.** Each chunk is a standalone zstd frame, so a
 * chunk is fetched, decoded and forgotten on its own — nothing ever holds a
 * chapter in a decoder, and chunk 5 can be fetched without chunks 0 to 4.
 *
 * **The assembled bytes are a contiguous prefix.** `<audio>` given a Blob is
 * playing *that blob*: byte 0 of it is time 0, and the duration it reports is
 * the blob's. So the blob must always start at the chapter's first byte, or
 * every position and duration the element reports is wrong. Playback therefore
 * starts as soon as chunk 0 is decompressed and grows forward from there.
 *
 * **A seek costs the chunks it skipped, not a chapter download.** Seeking to
 * the middle extends the prefix up to the target, which for a chapter that is
 * a couple of megabytes is nothing — and is the price of not needing MSE,
 * which Safari does not offer for mp3.
 */
import { decompress } from 'fzstd';

/** The table format this understands. A newer one is refused, not guessed at. */
export const SUPPORTED_VERSION = 1;

/**
 * Reads the chunk table the backend sends.
 *
 * Returns `null` for anything this build cannot safely use — an unknown
 * version, a missing field, or offsets that do not line up with the chunk
 * lengths. The last one matters most: the offsets are what a seek is computed
 * against, so a table whose offsets disagree with its own lengths would place
 * every seek somewhere other than where the listener asked. The caller's
 * fallback is the plain audio URL, so `null` is a normal answer, not an error.
 */
export function readChunkTable(payload) {
	if (!payload || typeof payload !== 'object') return null;
	if (payload.version !== SUPPORTED_VERSION) return null;

	const { chunk_size: chunkSize, raw_size: rawSize, chunks } = payload;
	if (!Number.isFinite(chunkSize) || chunkSize <= 0) return null;
	if (!Number.isFinite(rawSize) || rawSize <= 0) return null;
	if (!Array.isArray(chunks) || chunks.length === 0) return null;

	let expectedOffset = 0;
	const normalized = [];
	for (const [position, chunk] of chunks.entries()) {
		if (!chunk || typeof chunk !== 'object') return null;
		if (chunk.index !== position) return null;
		if (!Number.isFinite(chunk.offset) || chunk.offset !== expectedOffset) return null;
		if (!Number.isFinite(chunk.raw) || chunk.raw <= 0) return null;
		if (!Number.isFinite(chunk.compressed) || chunk.compressed <= 0) return null;
		if (typeof chunk.url !== 'string' || chunk.url === '') return null;

		expectedOffset += chunk.raw;
		normalized.push({
			index: position,
			offset: chunk.offset,
			raw: chunk.raw,
			compressed: chunk.compressed,
			sha256: typeof chunk.sha256 === 'string' ? chunk.sha256 : null,
			url: chunk.url
		});
	}

	// The lengths must add up to the chapter, or the last chunk is short of
	// what the table claims and a position near the end would never resolve.
	if (expectedOffset !== rawSize) return null;

	return {
		version: payload.version,
		chunkSize,
		rawSize,
		contentType: typeof payload.content_type === 'string' ? payload.content_type : 'audio/mpeg',
		chunks: normalized
	};
}

/**
 * Byte offset for a playback position.
 *
 * The library is constant-bitrate mono mp3, so position and offset are the same
 * line and `raw_size / duration` is the slope. A duration the player does not
 * know yet (0, or not a number) means "the start" rather than a division by
 * zero, and the result is clamped to the chapter — a time past the end must
 * resolve to the final chunk, not past the table.
 */
export function offsetForTime(seconds, durationSeconds, rawSize) {
	if (!Number.isFinite(rawSize) || rawSize <= 0) return 0;
	if (!Number.isFinite(seconds) || seconds <= 0) return 0;
	if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) return 0;

	const offset = Math.floor((seconds / durationSeconds) * rawSize);
	return Math.min(Math.max(offset, 0), rawSize - 1);
}

/** The chunk holding `offset`; past the end this is the last chunk. */
export function chunkAtOffset(table, offset) {
	for (const chunk of table.chunks) {
		if (offset < chunk.offset + chunk.raw) return chunk.index;
	}
	return table.chunks.length - 1;
}

/**
 * The chunks a contiguous prefix must include to cover `offset`.
 *
 * Always starts at zero: the prefix is what the blob is, and a blob that began
 * mid-chapter would report its own timeline rather than the chapter's.
 */
export function prefixThrough(table, offset) {
	const last = chunkAtOffset(table, offset);
	return table.chunks.slice(0, last + 1).map((chunk) => chunk.index);
}

/** Hex SHA-256 of bytes, or `null` where the runtime has no WebCrypto. */
export async function sha256Hex(bytes) {
	const subtle = globalThis.crypto?.subtle;
	if (!subtle) return null;

	const digest = await subtle.digest('SHA-256', bytes);
	return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/**
 * Fetches and decompresses one chunk.
 *
 * When the table carries a hash and the runtime can hash, the decompressed
 * bytes are checked against it: a frame that decodes to the wrong audio is
 * worse than one that fails to decode, because nothing downstream can tell.
 */
export async function loadChunk(table, index, { fetchImpl = fetch, decode = decompress } = {}) {
	const chunk = table.chunks[index];
	if (!chunk) throw new Error(`There is no chunk ${index} in this chapter.`);

	const response = await fetchImpl(chunk.url);
	if (!response?.ok) {
		throw new Error(`Chunk ${index} could not be fetched (${response?.status ?? 'no response'}).`);
	}

	const frame = new Uint8Array(await response.arrayBuffer());
	const raw = decode(frame);

	if (raw.length !== chunk.raw) {
		throw new Error(`Chunk ${index} decoded to ${raw.length} bytes, the table says ${chunk.raw}.`);
	}

	if (chunk.sha256) {
		const digest = await sha256Hex(raw);
		if (digest && digest !== chunk.sha256) {
			throw new Error(`Chunk ${index} decoded to bytes that do not match its hash.`);
		}
	}

	return raw;
}

/**
 * A chapter's decompressed bytes, grown a chunk at a time.
 *
 * Holds whatever prefix has been loaded and hands out a Blob URL for it. The
 * caller revokes the previous URL when it swaps `src`; nothing here touches the
 * DOM or an object URL it did not create.
 */
export class ChunkedAudio {
	#table;
	#fetchImpl;
	#decode;
	#pieces = [];
	#pending = Promise.resolve();
	/** Requests served. Exposed so tests can prove a chunk is not refetched. */
	fetches = 0;

	constructor(table, { fetchImpl = fetch, decode = decompress } = {}) {
		this.#table = table;
		this.#fetchImpl = fetchImpl;
		this.#decode = decode;
	}

	/** The table this was built from. */
	get table() {
		return this.#table;
	}

	/** Raw bytes held so far. */
	get loadedBytes() {
		return this.#pieces.reduce((total, piece) => total + (piece?.length ?? 0), 0);
	}

	/** True once every chunk is decompressed. */
	get complete() {
		return this.#pieces.length === this.#table.chunks.length;
	}

	/**
	 * Loads every chunk from the start through `index`, in order.
	 *
	 * Serialized through a promise chain so two overlapping calls — a seek
	 * landing while the prefetch is still running — cannot fetch the same chunk
	 * twice or append pieces out of order.
	 */
	loadThrough(index) {
		this.#pending = this.#pending.then(async () => {
			for (let next = 0; next <= index; next += 1) {
				if (this.#pieces[next]) continue;
				const raw = await loadChunk(this.#table, next, {
					fetchImpl: this.#fetchImpl,
					decode: this.#decode
				});
				this.fetches += 1;
				this.#pieces[next] = raw;
			}
		});
		return this.#pending;
	}

	/** Loads up to the chunk holding `offset`. */
	loadThroughOffset(offset) {
		return this.loadThrough(chunkAtOffset(this.#table, offset));
	}

	/** Loads up to the position `seconds` of a chapter `durationSeconds` long. */
	loadThroughTime(seconds, durationSeconds) {
		return this.loadThroughOffset(offsetForTime(seconds, durationSeconds, this.#table.rawSize));
	}

	/**
	 * A Blob URL for the loaded prefix, typed as the chapter's own audio type so
	 * a browser that sniffs the type before playing still recognises it.
	 *
	 * `null` before anything is loaded: an empty blob has no audio in it, and a
	 * caller that sets it as a source gets a decode error rather than a wait.
	 */
	toBlobUrl() {
		const loaded = this.#pieces.filter(Boolean);
		if (loaded.length === 0) return null;

		return URL.createObjectURL(new Blob(loaded, { type: this.#table.contentType }));
	}
}
