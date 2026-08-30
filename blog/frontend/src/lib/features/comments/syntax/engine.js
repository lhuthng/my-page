import MarkdownIt from 'markdown-it';
import { codeHighlightLazyPlugin } from '$lib/custom-rules/plugins/code-highlight-lazy.js';
import { mentionProfilePlugin, kaomojiPlugin } from '$lib/custom-rules';

export function createCommentSyntaxEngine({ mentionDictionary, plugins }) {
	const markdown = new MarkdownIt()
		.use(codeHighlightLazyPlugin)
		.use(mentionProfilePlugin, { mentionDictionary })
		.use(kaomojiPlugin);

	const pluginMap = new Map(plugins.map((plugin) => [plugin.key, plugin]));
	const pluginsByType = new Map();

	for (const plugin of plugins) {
		const list = pluginsByType.get(plugin.type) ?? [];
		list.push(plugin);
		pluginsByType.set(plugin.type, list);
	}

	function detectActiveByType(type, context) {
		const candidates = pluginsByType.get(type) ?? [];
		for (const plugin of candidates) {
			const detected = plugin.detect(context);
			if (detected) {
				return { plugin, context: detected };
			}
		}
		return null;
	}

	return {
		markdown,
		createContext(value, caret, extra = {}) {
			return { value, caret, ...extra };
		},
		detectActive(type, value, caret, extra = {}) {
			return detectActiveByType(type, this.createContext(value, caret, extra));
		},
		getPlugin(key) {
			return pluginMap.get(key) ?? null;
		},
		getPluginsByType(type) {
			return pluginsByType.get(type) ?? [];
		}
	};
}
