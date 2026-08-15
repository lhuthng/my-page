// Vendored zstd compressor for the v86 snapshot studio.
//
// Bundled from @bokuweb/zstd-wasm@0.0.27 (MIT) which wraps facebook/zstd
// (BSD-3-Clause / GPL-2.0). Rebuild: see static/zstd/NOTICE.md.
//
// Loaded by URL as a module worker, matching how libv86.js is shipped from
// static/ rather than bundled. Only the one-shot ZSTD_compress API exists in
// this build, so compression reports start/done rather than progress.
var Y = typeof Y < 'u' ? Y : {},
	j = {},
	H;
for (H in Y) if (Y.hasOwnProperty(H)) j[H] = Y[H];
var i = [],
	U = Y.printErr || console.warn.bind(console);
for (H in j) if (j.hasOwnProperty(H)) Y[H] = j[H];
var P = (Q, $) => {
	throw $;
};
j = null;
if (Y.arguments) i = Y.arguments;
if (Y.thisProgram) thisProgram = Y.thisProgram;
if (Y.quit) P = Y.quit;
if (typeof WebAssembly !== 'object') T('no native wasm support detected');
var B,
	D = !1,
	O;
var S, s;
function E() {
	var Q = B.buffer;
	((Y.HEAP8 = s = new Int8Array(Q)), (Y.HEAPU8 = S = new Uint8Array(Q)));
}
var h = [],
	z = [],
	v = [],
	t = !1;
function a() {
	if (Y.preRun) {
		if (typeof Y.preRun == 'function') Y.preRun = [Y.preRun];
		while (Y.preRun.length) u(Y.preRun.shift());
	}
	R(h);
}
function l() {
	((t = !0), R(z));
}
function o() {
	if (Y.postRun) {
		if (typeof Y.postRun == 'function') Y.postRun = [Y.postRun];
		while (Y.postRun.length) e(Y.postRun.shift());
	}
	R(v);
}
function u(Q) {
	h.unshift(Q);
}
function r(Q) {
	z.unshift(Q);
}
function e(Q) {
	v.unshift(Q);
}
var q = 0,
	I = null;
function Q9(Q) {
	var $;
	(q++, ($ = Y.monitorRunDependencies) === null || $ === void 0 || $.call(Y, q));
}
function Y9(Q) {
	var $;
	if ((q--, ($ = Y.monitorRunDependencies) === null || $ === void 0 || $.call(Y, q), q == 0)) {
		if (I) {
			var J = I;
			((I = null), J());
		}
	}
}
function T(Q) {
	var $;
	(($ = Y.onAbort) === null || $ === void 0 || $.call(Y, Q),
		(Q = 'Aborted(' + Q + ')'),
		U(Q),
		(D = !0),
		(Q += '. Build with -sASSERTIONS for more info.'));
	var J = new WebAssembly.RuntimeError(Q);
	throw J;
}
function $9() {
	return { a: C9 };
}
function J9(Q) {
	return fetch(Q, { credentials: 'same-origin' }).then(function ($) {
		if (!$.ok) throw "failed to load wasm binary file at '" + Q + "'";
		return $.arrayBuffer();
	});
}
function W9(Q) {
	var $ = $9();
	function J(X, N) {
		return ((G = X.exports), (B = G.f), E(), r(G.g), Y9('wasm-instantiate'), G);
	}
	Q9('wasm-instantiate');
	function W(X) {
		J(X.instance);
	}
	function V(X) {
		return J9(Q)
			.then(function (N) {
				var y = WebAssembly.instantiate(N, $);
				return y;
			})
			.then(X, function (N) {
				(U('failed to asynchronously prepare wasm: ' + N), T(N));
			});
	}
	function K() {
		if (Q && Q.byteLength > 0)
			return WebAssembly.instantiate(Q, $).then(W, function (X) {
				U('wasm compile failed: ' + X);
			});
		else if (
			typeof WebAssembly.instantiateStreaming === 'function' &&
			typeof Q === 'string' &&
			typeof fetch === 'function'
		)
			return fetch(Q, { credentials: 'same-origin' }).then(function (X) {
				var N = WebAssembly.instantiateStreaming(X, $);
				return N.then(W, function (y) {
					return (
						U('wasm streaming compile failed: ' + y),
						U('falling back to ArrayBuffer instantiation'),
						V(W)
					);
				});
			});
		else return V(W);
	}
	if (Y.instantiateWasm)
		try {
			var L = Y.instantiateWasm($, J);
			return L;
		} catch (X) {
			return (U('Module.instantiateWasm callback failed with error: ' + X), !1);
		}
	return (K(), {});
}
class g {
	constructor(Q) {
		((this.name = 'ExitStatus'),
			(this.message = `Program terminated with exit(${Q})`),
			(this.status = Q));
	}
}
var R = (Q) => {
		while (Q.length > 0) Q.shift()(Y);
	},
	w = Y.noExitRuntime || !0,
	K9 = () => T(''),
	b = 0,
	V9 = () => {
		((w = !1), (b = 0));
	},
	C = {},
	f = (Q) => {
		if (Q instanceof g || Q == 'unwind') return O;
		P(1, Q);
	},
	M = () => w || b > 0,
	m = (Q) => {
		var $;
		if (((O = Q), !M())) (($ = Y.onExit) === null || $ === void 0 || $.call(Y, Q), (D = !0));
		P(Q, new g(Q));
	},
	X9 = (Q, $) => {
		((O = Q), m(Q));
	},
	L9 = X9,
	G9 = () => {
		if (!M())
			try {
				L9(O);
			} catch (Q) {
				f(Q);
			}
	},
	N9 = (Q) => {
		if (D) return;
		try {
			(Q(), G9());
		} catch ($) {
			f($);
		}
	},
	F9 = () => performance.now(),
	y9 = (Q, $) => {
		if (C[Q]) (clearTimeout(C[Q].id), delete C[Q]);
		if (!$) return 0;
		var J = setTimeout(() => {
			(delete C[Q], N9(() => _(Q, F9())));
		}, $);
		return ((C[Q] = { id: J, timeout_ms: $ }), 0);
	},
	H9 = () => 2147483648,
	Z9 = (Q, $) => Math.ceil(Q / $) * $,
	q9 = (Q) => {
		var $ = B.buffer,
			J = ((Q - $.byteLength + 65535) / 65536) | 0;
		try {
			return (B.grow(J), E(), 1);
		} catch (W) {}
	},
	U9 = (Q) => {
		var $ = S.length;
		Q >>>= 0;
		var J = H9();
		if (Q > J) return !1;
		for (var W = 1; W <= 4; W *= 2) {
			var V = $ * (1 + 0.2 / W);
			V = Math.min(V, Q + 100663296);
			var K = Math.min(J, Z9(Math.max(Q, V), 65536)),
				L = q9(K);
			if (L) return !0;
		}
		return !1;
	},
	C9 = { c: K9, b: V9, d: y9, e: U9, a: m },
	G;
var j9 = (Y._ZSTD_isError = (Q) => (j9 = Y._ZSTD_isError = G.h)(Q)),
	I9 = (Y._ZSTD_compressBound = (Q) => (I9 = Y._ZSTD_compressBound = G.i)(Q)),
	k9 = (Y._ZSTD_createCCtx = () => (k9 = Y._ZSTD_createCCtx = G.j)()),
	B9 = (Y._ZSTD_freeCCtx = (Q) => (B9 = Y._ZSTD_freeCCtx = G.k)(Q)),
	A9 = (Y._ZSTD_compress_usingDict = (Q, $, J, W, V, K, L, X) =>
		(A9 = Y._ZSTD_compress_usingDict = G.l)(Q, $, J, W, V, K, L, X)),
	D9 = (Y._ZSTD_compress = (Q, $, J, W, V) => (D9 = Y._ZSTD_compress = G.m)(Q, $, J, W, V)),
	O9 = (Y._ZSTD_createDCtx = () => (O9 = Y._ZSTD_createDCtx = G.n)()),
	P9 = (Y._ZSTD_freeDCtx = (Q) => (P9 = Y._ZSTD_freeDCtx = G.o)(Q)),
	T9 = (Y._ZSTD_getFrameContentSize = (Q, $) => (T9 = Y._ZSTD_getFrameContentSize = G.p)(Q, $)),
	g9 = (Y._ZSTD_decompress_usingDict = (Q, $, J, W, V, K, L) =>
		(g9 = Y._ZSTD_decompress_usingDict = G.q)(Q, $, J, W, V, K, L)),
	R9 = (Y._ZSTD_decompress = (Q, $, J, W) => (R9 = Y._ZSTD_decompress = G.r)(Q, $, J, W)),
	x9 = (Y._malloc = (Q) => (x9 = Y._malloc = G.s)(Q)),
	S9 = (Y._free = (Q) => (S9 = Y._free = G.t)(Q)),
	_ = (Q, $) => (_ = G.v)(Q, $),
	A;
I = function Q() {
	if (!A) p();
	if (!A) I = Q;
};
function p() {
	if (q > 0) return;
	if ((a(), q > 0)) return;
	function Q() {
		var $;
		if (A) return;
		if (((A = !0), (Y.calledRun = !0), D)) return;
		(l(), ($ = Y.onRuntimeInitialized) === null || $ === void 0 || $.call(Y), o());
	}
	if (Y.setStatus)
		(Y.setStatus('Running...'),
			setTimeout(() => {
				(setTimeout(() => Y.setStatus(''), 1), Q());
			}, 1));
	else Q();
}
Y.run = p;
if (Y.preInit) {
	if (typeof Y.preInit == 'function') Y.preInit = [Y.preInit];
	while (Y.preInit.length > 0) Y.preInit.pop()();
}
Y.init = W9;
var E9 = function (Q, $, J, W) {
		function V(K) {
			return K instanceof J
				? K
				: new J(function (L) {
						L(K);
					});
		}
		return new (J || (J = Promise))(function (K, L) {
			function X(F) {
				try {
					y(W.next(F));
				} catch (Z) {
					L(Z);
				}
			}
			function N(F) {
				try {
					y(W.throw(F));
				} catch (Z) {
					L(Z);
				}
			}
			function y(F) {
				F.done ? K(F.value) : V(F.value).then(X, N);
			}
			y((W = W.apply(Q, $ || [])).next());
		});
	},
	h9 = (() =>
		new Promise((Q) => {
			Y.onRuntimeInitialized = Q;
		}))(),
	n = () =>
		E9(void 0, void 0, void 0, function* () {
			yield h9;
		});
var k = (Q) => {
	let $ = Y._ZSTD_isError;
	return $(Q);
};
var z9 = (Q) => {
		let $ = Y._ZSTD_compressBound;
		return $(Q);
	},
	c = (Q, $) => {
		let J = z9(Q.byteLength),
			W = Y._malloc,
			V = W(J),
			K = W(Q.byteLength);
		Y.HEAP8.set(Q, K);
		let L = Y._free;
		try {
			let X = Y._ZSTD_compress,
				N = X(V, J, K, Q.byteLength, $ !== null && $ !== void 0 ? $ : 3);
			if (k(N)) throw Error(`Failed to compress with code ${N}`);
			let y = new Uint8Array(Y.HEAPU8.buffer, V, N).slice();
			return (L(V, J), L(K, Q.byteLength), y);
		} catch (X) {
			throw (L(V, J), L(K, Q.byteLength), X);
		}
	};
var v9 = function (Q, $, J, W) {
		function V(K) {
			return K instanceof J
				? K
				: new J(function (L) {
						L(K);
					});
		}
		return new (J || (J = Promise))(function (K, L) {
			function X(F) {
				try {
					y(W.next(F));
				} catch (Z) {
					L(Z);
				}
			}
			function N(F) {
				try {
					y(W.throw(F));
				} catch (Z) {
					L(Z);
				}
			}
			function y(F) {
				F.done ? K(F.value) : V(F.value).then(X, N);
			}
			y((W = W.apply(Q, $ || [])).next());
		});
	},
	d = (Q) =>
		v9(void 0, void 0, void 0, function* () {
			let $ = new URL('./zstd.wasm', import.meta.url).href;
			(Y.init(Q !== null && Q !== void 0 ? Q : $), yield n());
		});
var x;
self.onmessage = async (Q) => {
	let { buffer: $, level: J = 19, wasmUrl: W = '/zstd/zstd.wasm' } = Q.data ?? {};
	if (!($ instanceof ArrayBuffer)) {
		self.postMessage({ type: 'error', message: 'Expected an ArrayBuffer to compress.' });
		return;
	}
	try {
		((x = x ?? d(W)), await x);
		let V = new Uint8Array($),
			K = V.byteLength;
		self.postMessage({ type: 'started', rawSize: K });
		let L = performance.now(),
			X = c(V, J);
		self.postMessage(
			{
				type: 'done',
				compressed: X.buffer,
				compressedSize: X.byteLength,
				rawSize: K,
				elapsedMs: performance.now() - L
			},
			[X.buffer]
		);
	} catch (V) {
		self.postMessage({ type: 'error', message: V?.message ?? 'zstd compression failed.' });
	}
};
