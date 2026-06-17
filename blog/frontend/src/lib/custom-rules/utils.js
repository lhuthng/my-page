export function slugify(text) {
	return text
		.toLowerCase()
		.normalize('NFD')
		.replace(/[\u0300-\u036f]/g, '')
		.replace(/[^a-z0-9\s-]/g, '')
		.trim()
		.replace(/\s+/g, '-')
		.replace(/-+/g, '-');
}

export function findHeaders(root) {
	const headers = root.querySelectorAll('h1, h2, h3');
	const next = { H1: 'H2', H2: 'H3', H3: 'H4' };
	const result = [];
	let queue = [{ tagName: 'H1', target: result }];
	headers.forEach(({ id, tagName, textContent }) => {
		let count = 0;
		const max = 100;

		const peek = queue.at(-1);
		if (tagName == peek.tagName) {
			peek.target.push({ id, textContent });
		} else if (tagName > peek.tagName) {
			let nextName = peek.tagName;
			count = 0;
			do {
				count++;
				if (count > max) return;
				nextName = next[nextName];
				const children = [];
				queue.at(-1).target.push(children);
				queue.push({ tagName: nextName, target: children });
			} while (tagName !== nextName);
			queue.at(-1).target.push({ id, textContent });
		} else {
			while (tagName !== queue.at(-1).tagName) {
				queue.pop();
			}
			queue.at(-1).target.push({ id, textContent });
		}
	});
	return result;
}
