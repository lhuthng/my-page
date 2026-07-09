import { MentionHandler } from '../handlers.js';

export function createMentionPlugin({ searchProfiles }) {
	return new MentionHandler({
		searchProfiles
	});
}
