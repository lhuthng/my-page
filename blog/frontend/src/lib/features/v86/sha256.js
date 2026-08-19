const K = new Uint32Array([
	0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
	0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
	0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
	0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
	0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
	0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
]);

/**
 * Incremental SHA-256. The browser's `crypto.subtle.digest` only accepts a
 * whole buffer, but the game disk can be ~1.5 GiB, so the digest must be
 * streamed chunk by chunk. Use `update()` repeatedly, then `digest()` /
 * `digestHex()` once.
 */
export class Sha256 {
	constructor() {
		this.h = new Uint32Array([
			0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19
		]);
		this.pending = new Uint8Array(0);
		this.totalBytes = 0;
	}

	update(data) {
		if (data.length === 0) return this;
		const merged = new Uint8Array(this.pending.length + data.length);
		merged.set(this.pending);
		merged.set(data, this.pending.length);
		this.totalBytes += data.length;
		let offset = 0;
		const fullBlocks = Math.max(0, Math.floor((merged.length - 8) / 64));
		if (fullBlocks > 0) {
			this._process(merged, offset, fullBlocks);
			offset = fullBlocks * 64;
		}
		this.pending = merged.slice(offset);
		return this;
	}

	_process(data, offset, count) {
		const words = new Uint32Array(64);
		const h = this.h;
		for (let block = 0; block < count; block++) {
			for (let i = 0; i < 16; i++) {
				const base = offset + block * 64 + i * 4;
				words[i] =
					(data[base] << 24) | (data[base + 1] << 16) | (data[base + 2] << 8) | data[base + 3];
			}
			for (let i = 16; i < 64; i++) {
				const w15 = words[i - 15];
				const w2 = words[i - 2];
				const s0 = ((w15 >>> 7) | (w15 << 25)) ^ ((w15 >>> 18) | (w15 << 14)) ^ (w15 >>> 3);
				const s1 = ((w2 >>> 17) | (w2 << 15)) ^ ((w2 >>> 19) | (w2 << 13)) ^ (w2 >>> 10);
				words[i] = (words[i - 16] + s0 + words[i - 7] + s1) | 0;
			}
			let a = h[0];
			let b = h[1];
			let c = h[2];
			let d = h[3];
			let e = h[4];
			let f = h[5];
			let g = h[6];
			let hh = h[7];
			for (let i = 0; i < 64; i++) {
				const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
				const ch = (e & f) ^ (~e & g);
				const temp1 = (hh + S1 + ch + K[i] + words[i]) | 0;
				const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
				const maj = (a & b) ^ (a & c) ^ (b & c);
				const temp2 = (S0 + maj) | 0;
				hh = g;
				g = f;
				f = e;
				e = (d + temp1) | 0;
				d = c;
				c = b;
				b = a;
				a = (temp1 + temp2) | 0;
			}
			h[0] = (h[0] + a) | 0;
			h[1] = (h[1] + b) | 0;
			h[2] = (h[2] + c) | 0;
			h[3] = (h[3] + d) | 0;
			h[4] = (h[4] + e) | 0;
			h[5] = (h[5] + f) | 0;
			h[6] = (h[6] + g) | 0;
			h[7] = (h[7] + hh) | 0;
		}
	}

	digest() {
		const bitLength = this.totalBytes * 8;
		const zeros = (64 - ((this.pending.length + 9) % 64)) % 64;
		const final = new Uint8Array(this.pending.length + 1 + zeros + 8);
		final.set(this.pending);
		final[this.pending.length] = 0x80;
		const hi = Math.floor(bitLength / 0x100000000);
		const lo = bitLength >>> 0;
		const view = new DataView(final.buffer);
		view.setUint32(final.length - 8, hi);
		view.setUint32(final.length - 4, lo);
		this._process(final, 0, final.length / 64);
		const out = new Uint8Array(32);
		const viewOut = new DataView(out.buffer);
		for (let i = 0; i < 8; i++) viewOut.setUint32(i * 4, this.h[i]);
		return out;
	}

	digestHex() {
		return Array.from(this.digest(), (b) => b.toString(16).padStart(2, '0')).join('');
	}
}

export async function sha256Hex(data) {
	const hasher = new Sha256();
	hasher.update(data);
	return hasher.digestHex();
}
