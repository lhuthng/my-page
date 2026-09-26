/**
 * Formatting helpers for audio time values.
 *
 * Kept isomorphic and dependency-free so the same code can format a duration
 * during SSR and while the player is ticking on the client.
 */

/**
 * Format a number of seconds as a clock string.
 *
 * Under an hour this is `m:ss`; an hour or more becomes `h:mm:ss`. Non-finite,
 * negative, and missing values render as `--:--` so a track whose duration has
 * not been probed yet shows a placeholder instead of `NaN`.
 */
export function formatClock(seconds) {
	if (seconds == null || !Number.isFinite(seconds) || seconds < 0) return '--:--';

	const total = Math.floor(seconds);
	const hours = Math.floor(total / 3600);
	const minutes = Math.floor((total % 3600) / 60);
	const secs = total % 60;

	const pad = (value) => String(value).padStart(2, '0');

	if (hours > 0) return `${hours}:${pad(minutes)}:${pad(secs)}`;
	return `${minutes}:${pad(secs)}`;
}

/**
 * Format a duration for display next to a total, e.g. "3 hr 42 min" or
 * "18 min". Pass `lang = 'vi'` for Vietnamese wording ("3 tiếng 42 phút").
 * Returns an empty string for unknown or zero durations so callers can render
 * nothing rather than a misleading "0 min".
 */
export function formatDurationLabel(seconds, lang = 'en') {
	if (!Number.isFinite(seconds) || seconds <= 0) return '';

	// Check the raw value, not the rounded minutes: rounding 30s would produce
	// "1 min", and "under a minute" should mean exactly that.
	if (seconds < 60) return lang === 'vi' ? 'dưới một phút' : 'under a minute';

	const totalMinutes = Math.round(seconds / 60);
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;

	if (lang === 'vi') {
		if (hours === 0) return `${minutes} phút`;
		if (minutes === 0) return `${hours} tiếng`;
		return `${hours} tiếng ${minutes} phút`;
	}

	if (hours === 0) return `${minutes} min`;
	if (minutes === 0) return `${hours} hr`;
	return `${hours} hr ${minutes} min`;
}

/**
 * Percentage of `total` that `value` represents, clamped to 0..100 and safe
 * against a zero or unknown total (which would otherwise produce Infinity).
 */
export function percentOf(value, total) {
	if (!Number.isFinite(value) || !Number.isFinite(total) || total <= 0) return 0;
	return Math.min(100, Math.max(0, (value / total) * 100));
}
