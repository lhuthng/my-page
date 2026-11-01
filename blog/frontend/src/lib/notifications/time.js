export function relativeTime(value) {
	if (!value) return '';
	const then = new Date(value);
	const seconds = Math.round((then.getTime() - Date.now()) / 1000);
	const formatter = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' });

	const thresholds = [
		[60, 'second'],
		[3600, 'minute'],
		[86400, 'hour'],
		[604800, 'day'],
		[2629800, 'week'],
		[31557600, 'month']
	];

	let previous = 1;
	for (const [limit, unit] of thresholds) {
		if (Math.abs(seconds) < limit) {
			return formatter.format(Math.round(seconds / previous), unit);
		}
		previous = limit;
	}
	return formatter.format(Math.round(seconds / 31557600), 'year');
}
