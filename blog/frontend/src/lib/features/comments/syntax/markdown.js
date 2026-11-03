import MarkdownIt from 'markdown-it';
import { mentionProfilePlugin } from '../../../custom-rules/plugins/mention.js';
import { kaomojiPlugin } from '../../../custom-rules/plugins/kaomoji.js';

// Single definition of comment markdown so the client VM and the profile
// server load cannot drift apart on mentions, kaomoji, or code blocks.
export function createCommentMarkdown({ mentionDictionary = {}, highlightPlugin } = {}) {
	const markdown = new MarkdownIt();
	if (highlightPlugin) markdown.use(highlightPlugin);
	markdown.use(mentionProfilePlugin, { mentionDictionary }).use(kaomojiPlugin);
	return markdown;
}

export function extractMentionUsernames(value) {
	if (!value) return [];
	const usernames = new Set();
	const scan = /(^|[\s(>])@([A-Za-z0-9_-]{3,32})\b/g;
	let match;
	while ((match = scan.exec(value)) !== null) {
		if (match[2]) usernames.add(match[2]);
	}
	return [...usernames];
}
