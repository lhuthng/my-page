export class SyntaxHandler {
	constructor({ key, type, meta = {} }) {
		this.key = key;
		this.type = type;
		this.meta = meta;
	}

	detect() {
		return null;
	}

	async search() {
		return { items: [], suggestions: [], error: null };
	}

	apply() {
		return null;
	}

	applySuggestion() {
		return null;
	}

	decoratePreview(content) {
		return content;
	}

	isActiveKey(key) {
		return this.key === key;
	}
}

export class RegexTriggerHandler extends SyntaxHandler {
	constructor({ key, type, regex, minQueryLength = 0, meta = {} }) {
		super({ key, type, meta });
		this.regex = regex;
		this.minQueryLength = minQueryLength;
	}

	getSafeCaret(value, caret) {
		return Number.isInteger(caret) ? Math.max(0, Math.min(caret, value.length)) : value.length;
	}

	detect(context) {
		if (typeof context?.value !== 'string') return null;
		const caret = this.getSafeCaret(context.value, context.caret);
		const before = context.value.slice(0, caret);
		const match = before.match(this.regex);
		if (!match) return null;

		const detected = this.createContext({ value: context.value, caret, before, match });
		if (!detected) return null;
		if ((detected.query?.length ?? 0) < this.minQueryLength) return null;
		return detected;
	}

	createContext() {
		return null;
	}
}

export class MentionHandler extends RegexTriggerHandler {
	constructor({ key = 'mention', searchProfiles, minQueryLength = 3, meta = {} }) {
		super({
			key,
			type: 'mention',
			regex: /(?:^|\s)@([A-Za-z0-9_-]+)$/,
			minQueryLength,
			meta
		});
		this.searchProfiles = searchProfiles;
	}

	createContext({ before, caret, match }) {
		const query = match[1];
		const fullMatch = match[0];
		const start = before.length - fullMatch.length + fullMatch.lastIndexOf('@');
		return { key: this.key, query, start, caret };
	}

	async search(context) {
		const items = await this.searchProfiles(context.query);
		return { items, suggestions: [], error: null };
	}

	apply(selection, item) {
		const username = item?.username;
		if (!username) return null;
		const left = selection.value.slice(0, selection.start + 1);
		const right = selection.value.slice(selection.caret);
		const value = `${left}${username} ${right}`;
		const next = left.length + username.length + 1;
		return {
			value,
			selectionStart: next,
			selectionEnd: next
		};
	}
}

export class SlashCommandHandler extends RegexTriggerHandler {
	constructor({ key, search, minQueryLength = 0, trigger = key, meta = {} }) {
		super({
			key,
			type: 'command',
			regex: new RegExp(`\\/(?:${trigger})\\s+(\\S*)$`),
			minQueryLength,
			meta: {
				loadingLabel: trigger,
				emptyText: `Type to search... (e.g. /${trigger} query)`,
				...meta
			}
		});
		this.searchImpl = search;
		this.trigger = trigger;
	}

	buildToken(query) {
		const normalizedQuery = String(query ?? '');
		return normalizedQuery ? `/${this.trigger} ${normalizedQuery}` : `/${this.trigger} `;
	}

	createContext({ value, before, caret }) {
		const lineStart = before.lastIndexOf('\n') + 1;
		const activeLine = before.slice(lineStart);
		const localMatch = activeLine.match(new RegExp(`\\/(?:${this.trigger})\\s+(\\S*)$`));
		if (!localMatch) return null;

		const query = localMatch[1] ?? '';
		const relativeStart = localMatch.index ?? activeLine.lastIndexOf(`/${this.trigger}`);
		const start = lineStart + relativeStart;
		let replaceEnd = caret;
		const afterCaret = value.slice(caret);
		const nextBreak = afterCaret.search(/[\s\n]/);
		if (nextBreak !== -1) {
			replaceEnd = caret + nextBreak;
		} else {
			replaceEnd = value.length;
		}

		return {
			key: this.key,
			query,
			start,
			caret,
			replaceEnd,
			trigger: this.trigger
		};
	}

	search(context) {
		return this.searchImpl(context.query);
	}
}
