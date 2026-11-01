import MarkdownIt from 'markdown-it';
import { kaomojiPlugin } from '$lib/custom-rules/plugins/kaomoji.js';
import { mentionProfilePlugin } from '$lib/custom-rules/plugins/mention.js';

const PREVIEW_CHARS = 140;

const markdown = new MarkdownIt().use(mentionProfilePlugin).use(kaomojiPlugin);

const SPECIAL_LABELS = {
	image: '@img',
	link: '@url',
	code: '@code'
};

// Without a break at these closes, separate paragraphs, list items, headings,
// and table cells would run together as one word once the markup is gone.
const BLOCK_BREAKS = new Set([
	'paragraph_close',
	'heading_close',
	'blockquote_close',
	'list_item_close',
	'bullet_list_close',
	'ordered_list_close',
	'th_close',
	'td_close'
]);

function createCollector() {
	const parts = [];
	let needsSpace = false;

	return {
		parts,

		space() {
			needsSpace = true;
		},

		text(raw) {
			const text = raw.replace(/\s+/g, ' ');
			if (!text) return;
			const value = needsSpace && !text.startsWith(' ') ? ` ${text}` : text;
			needsSpace = false;
			const last = parts[parts.length - 1];
			if (last && !last.kind) last.text += value;
			else parts.push({ text: value });
		},

		special(kind, label = SPECIAL_LABELS[kind]) {
			const last = parts[parts.length - 1];
			if (last && !last.text.endsWith(' ')) {
				if (last.kind) parts.push({ text: ' ' });
				else last.text += ' ';
			}
			parts.push({ text: label, kind });
			needsSpace = true;
		}
	};
}

function walkInline(children, collector) {
	// A link is replaced by @url as a whole, so its label never reaches the
	// preview; nested inline markup inside that label is skipped by depth.
	let linkDepth = 0;

	for (const token of children) {
		if (linkDepth > 0) {
			if (token.type === 'link_open') linkDepth += 1;
			else if (token.type === 'link_close') linkDepth -= 1;
			continue;
		}

		switch (token.type) {
			case 'text':
			case 'entity':
			case 'escape':
				collector.text(token.content);
				break;
			case 'code_inline':
				collector.special('code');
				break;
			case 'image':
				collector.special('image');
				break;
			case 'link_open':
				collector.special('link');
				linkDepth = 1;
				break;
			case 'softbreak':
			case 'hardbreak':
				collector.text(' ');
				break;
			case 'kaomoji':
				collector.text(token.meta?.content ?? '');
				break;
			case 'mention_profile':
				collector.special('mention', token.meta?.username ?? '');
				break;
			default:
				// Emphasis, strong, and strike wrappers carry no text of their
				// own; their children are flat in this same list.
				break;
		}
	}
}

function finalize(parts) {
	const cleaned = [];

	for (const part of parts) {
		const text = part.text.replace(/\s+/g, ' ');
		if (!text) continue;

		if (part.kind) {
			cleaned.push({ text, kind: part.kind });
			continue;
		}

		const last = cleaned[cleaned.length - 1];
		if (last && !last.kind) last.text += text;
		else cleaned.push({ text });
	}

	const first = cleaned[0];
	if (first && !first.kind) first.text = first.text.replace(/^ /, '');
	const last = cleaned[cleaned.length - 1];
	if (last && !last.kind) last.text = last.text.replace(/ $/, '');

	return cleaned.filter((part) => part.text);
}

function trimTrailingSpace(parts) {
	const last = parts[parts.length - 1];
	if (!last || last.kind) return;
	last.text = last.text.replace(/ +$/, '');
	if (!last.text) parts.pop();
}

function truncate(parts, limit) {
	const out = [];
	let remaining = limit;

	for (const part of parts) {
		if (remaining <= 0) {
			out.push({ text: '…' });
			return out;
		}

		const chars = Array.from(part.text);
		if (chars.length <= remaining) {
			out.push(part);
			remaining -= chars.length;
			continue;
		}

		out.push({ ...part, text: chars.slice(0, remaining).join('') });
		trimTrailingSpace(out);
		out.push({ text: '…' });
		return out;
	}

	return out;
}

/**
 * Turn raw comment markdown into the plain, readable parts a notification
 * preview shows: formatting markers disappear, kaomoji keep only their inner
 * text, and media becomes an italic @img / @url / @code label.
 */
export function previewParts(content, limit = PREVIEW_CHARS) {
	const collector = createCollector();

	for (const token of markdown.parse(String(content ?? ''), {})) {
		if (token.type === 'inline') {
			walkInline(token.children ?? [], collector);
		} else if (token.type === 'fence' || token.type === 'code_block') {
			collector.special('code');
		} else if (BLOCK_BREAKS.has(token.type)) {
			collector.space();
		}
	}

	return truncate(finalize(collector.parts), limit);
}
