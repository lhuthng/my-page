import { noop, run_all, safe_not_equal, subscribe_to_store } from "./utils2-B3oc7zZR.js";

//#region node_modules/@sveltejs/kit/src/runtime/server/constants.js
const IN_WEBCONTAINER = !!globalThis.process?.versions?.webcontainer;

//#endregion
//#region node_modules/@sveltejs/kit/src/exports/internal/event.js
/** @type {RequestStore | null} */
let sync_store = null;
/** @type {AsyncLocalStorage<RequestStore | null> | null} */
let als;
import("node:async_hooks").then((hooks) => als = new hooks.AsyncLocalStorage()).catch(() => {});
/**
* @template T
* @param {RequestStore | null} store
* @param {() => T} fn
*/
function with_request_store(store, fn) {
	try {
		sync_store = store;
		return als ? als.run(store, fn) : fn();
	} finally {
		if (!IN_WEBCONTAINER) sync_store = null;
	}
}

//#endregion
//#region node_modules/@sveltejs/kit/src/exports/internal/server.js
/**
* @template {{ tracing: { enabled: boolean, root: import('@opentelemetry/api').Span, current: import('@opentelemetry/api').Span } }} T
* @param {T} event_like
* @param {import('@opentelemetry/api').Span} current
* @returns {T}
*/
function merge_tracing(event_like, current) {
	return {
		...event_like,
		tracing: {
			...event_like.tracing,
			current
		}
	};
}

//#endregion
//#region .svelte-kit/adapter-bun/chunks/exports.js
const internal = new URL("sveltekit-internal://");
function resolve(base, path) {
	if (path[0] === "/" && path[1] === "/") return path;
	let url = new URL(base, internal);
	url = new URL(path, url);
	return url.protocol === internal.protocol ? url.pathname + url.search + url.hash : url.href;
}
function normalize_path(path, trailing_slash) {
	if (path === "/" || trailing_slash === "ignore") return path;
	if (trailing_slash === "never") return path.endsWith("/") ? path.slice(0, -1) : path;
	else if (trailing_slash === "always" && !path.endsWith("/")) return path + "/";
	return path;
}
function decode_pathname(pathname) {
	return pathname.split("%25").map(decodeURI).join("%25");
}
function decode_params(params) {
	for (const key in params) params[key] = decodeURIComponent(params[key]);
	return params;
}
function make_trackable(url, callback, search_params_callback, allow_hash = false) {
	const tracked = new URL(url);
	Object.defineProperty(tracked, "searchParams", {
		value: new Proxy(tracked.searchParams, { get(obj, key) {
			if (key === "get" || key === "getAll" || key === "has") return (param) => {
				search_params_callback(param);
				return obj[key](param);
			};
			callback();
			const value = Reflect.get(obj, key);
			return typeof value === "function" ? value.bind(obj) : value;
		} }),
		enumerable: true,
		configurable: true
	});
	const tracked_url_properties = [
		"href",
		"pathname",
		"search",
		"toString",
		"toJSON"
	];
	if (allow_hash) tracked_url_properties.push("hash");
	for (const property of tracked_url_properties) Object.defineProperty(tracked, property, {
		get() {
			callback();
			return url[property];
		},
		enumerable: true,
		configurable: true
	});
	{
		tracked[Symbol.for("nodejs.util.inspect.custom")] = (depth, opts, inspect) => {
			return inspect(url, opts);
		};
		tracked.searchParams[Symbol.for("nodejs.util.inspect.custom")] = (depth, opts, inspect) => {
			return inspect(url.searchParams, opts);
		};
	}
	if (!allow_hash) disable_hash(tracked);
	return tracked;
}
function disable_hash(url) {
	allow_nodejs_console_log(url);
	Object.defineProperty(url, "hash", { get() {
		throw new Error("Cannot access event.url.hash. Consider using `page.url.hash` inside a component instead");
	} });
}
function disable_search(url) {
	allow_nodejs_console_log(url);
	for (const property of ["search", "searchParams"]) Object.defineProperty(url, property, { get() {
		throw new Error(`Cannot access url.${property} on a page with prerendering enabled`);
	} });
}
function allow_nodejs_console_log(url) {
	url[Symbol.for("nodejs.util.inspect.custom")] = (depth, opts, inspect) => {
		return inspect(new URL(url), opts);
	};
}
const subscriber_queue = [];
function readable(value, start) {
	return { subscribe: writable(value, start).subscribe };
}
function writable(value, start = noop) {
	let stop = null;
	const subscribers = /* @__PURE__ */ new Set();
	function set(new_value) {
		if (safe_not_equal(value, new_value)) {
			value = new_value;
			if (stop) {
				const run_queue = !subscriber_queue.length;
				for (const subscriber of subscribers) {
					subscriber[1]();
					subscriber_queue.push(subscriber, value);
				}
				if (run_queue) {
					for (let i = 0; i < subscriber_queue.length; i += 2) subscriber_queue[i][0](subscriber_queue[i + 1]);
					subscriber_queue.length = 0;
				}
			}
		}
	}
	function update(fn) {
		set(fn(
			/** @type {T} */
			value
		));
	}
	function subscribe(run, invalidate = noop) {
		const subscriber = [run, invalidate];
		subscribers.add(subscriber);
		if (subscribers.size === 1) stop = start(set, update) || noop;
		run(
			/** @type {T} */
			value
		);
		return () => {
			subscribers.delete(subscriber);
			if (subscribers.size === 0 && stop) {
				stop();
				stop = null;
			}
		};
	}
	return {
		set,
		update,
		subscribe
	};
}
function derived(stores, fn, initial_value) {
	const single = !Array.isArray(stores);
	const stores_array = single ? [stores] : stores;
	if (!stores_array.every(Boolean)) throw new Error("derived() expects stores as input, got a falsy value");
	const auto = fn.length < 2;
	return readable(initial_value, (set, update) => {
		let started = false;
		const values = [];
		let pending = 0;
		let cleanup = noop;
		const sync = () => {
			if (pending) return;
			cleanup();
			const result = fn(single ? values[0] : values, set, update);
			if (auto) set(result);
			else cleanup = typeof result === "function" ? result : noop;
		};
		const unsubscribers = stores_array.map((store, i) => subscribe_to_store(store, (value) => {
			values[i] = value;
			pending &= ~(1 << i);
			if (started) sync();
		}, () => {
			pending |= 1 << i;
		}));
		started = true;
		sync();
		return function stop() {
			run_all(unsubscribers);
			cleanup();
			started = false;
		};
	});
}
function get(store) {
	let value;
	subscribe_to_store(store, (_) => value = _)();
	return value;
}
function validator(expected) {
	function validate(module, file) {
		if (!module) return;
		for (const key in module) {
			if (key[0] === "_" || expected.has(key)) continue;
			const values = [...expected.values()];
			const hint = hint_for_supported_files(key, file?.slice(file.lastIndexOf("."))) ?? `valid exports are ${values.join(", ")}, or anything with a '_' prefix`;
			throw new Error(`Invalid export '${key}'${file ? ` in ${file}` : ""} (${hint})`);
		}
	}
	return validate;
}
function hint_for_supported_files(key, ext = ".js") {
	const supported_files = [];
	if (valid_layout_exports.has(key)) supported_files.push(`+layout${ext}`);
	if (valid_page_exports.has(key)) supported_files.push(`+page${ext}`);
	if (valid_layout_server_exports.has(key)) supported_files.push(`+layout.server${ext}`);
	if (valid_page_server_exports.has(key)) supported_files.push(`+page.server${ext}`);
	if (valid_server_exports.has(key)) supported_files.push(`+server${ext}`);
	if (supported_files.length > 0) return `'${key}' is a valid export in ${supported_files.slice(0, -1).join(", ")}${supported_files.length > 1 ? " or " : ""}${supported_files.at(-1)}`;
}
const valid_layout_exports = /* @__PURE__ */ new Set([
	"load",
	"prerender",
	"csr",
	"ssr",
	"trailingSlash",
	"config"
]);
const valid_page_exports = /* @__PURE__ */ new Set([...valid_layout_exports, "entries"]);
const valid_layout_server_exports = /* @__PURE__ */ new Set([...valid_layout_exports]);
const valid_page_server_exports = /* @__PURE__ */ new Set([
	...valid_layout_server_exports,
	"actions",
	"entries"
]);
const valid_server_exports = /* @__PURE__ */ new Set([
	"GET",
	"POST",
	"PATCH",
	"PUT",
	"DELETE",
	"OPTIONS",
	"HEAD",
	"fallback",
	"prerender",
	"trailingSlash",
	"config",
	"entries"
]);
const validate_layout_exports = validator(valid_layout_exports);
const validate_page_exports = validator(valid_page_exports);
const validate_layout_server_exports = validator(valid_layout_server_exports);
const validate_page_server_exports = validator(valid_page_server_exports);
const validate_server_exports = validator(valid_server_exports);

//#endregion
//#region .svelte-kit/adapter-bun/chunks/utils.js
const text_encoder = new TextEncoder();
const text_decoder = new TextDecoder();
function get_relative_path(from, to) {
	const from_parts = from.split(/[/\\]/);
	const to_parts = to.split(/[/\\]/);
	from_parts.pop();
	while (from_parts[0] === to_parts[0]) {
		from_parts.shift();
		to_parts.shift();
	}
	let i = from_parts.length;
	while (i--) from_parts[i] = "..";
	return from_parts.concat(to_parts).join("/");
}
function base64_encode(bytes) {
	if (globalThis.Buffer) return globalThis.Buffer.from(bytes).toString("base64");
	let binary = "";
	for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
	return btoa(binary);
}
function base64_decode(encoded) {
	if (globalThis.Buffer) {
		const buffer = globalThis.Buffer.from(encoded, "base64");
		return new Uint8Array(buffer);
	}
	const binary = atob(encoded);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

//#endregion
export { base64_decode, base64_encode, decode_params, decode_pathname, derived, disable_search, get, get_relative_path, make_trackable, merge_tracing, normalize_path, readable, resolve, text_decoder, text_encoder, validate_layout_exports, validate_layout_server_exports, validate_page_exports, validate_page_server_exports, with_request_store, writable };
//# sourceMappingURL=utils-B7r1aIrD.js.map