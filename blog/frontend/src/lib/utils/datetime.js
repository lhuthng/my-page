/**
 * Formatting helpers for database timestamps.
 *
 * Kept isomorphic and dependency-free so a timestamp can be formatted during
 * SSR and again on the client without either path disagreeing.
 *
 * SQLite hands these out as `YYYY-MM-DD HH:MM:SS` in **UTC** — `CURRENT_TIMESTAMP`
 * is UTC by definition, and every migration default in the schema is too. That
 * string carries no timezone marker, so `new Date()` would read it as local
 * time and shift every timestamp by the reader's offset. The marker has to be
 * added explicitly here rather than at each call site.
 *
 * Two different questions are asked of a timestamp, and they do not have the
 * same right answer:
 *
 * - "Which *day* was this record created on?" → the calendar day as stored, so
 *   it reads the same for every reader. Use `formatDateOnly`.
 * - "How long ago was this, and when exactly?" → a moment in time, relative to
 *   the reader's own now. Use `formatRelative` / `formatLastUpdated`.
 *
 * Getting this backwards is not a cosmetic slip: rendering a stored date in the
 * reader's zone shows a post published at 23:00 UTC as the *next* day to anyone
 * east of Greenwich, and a sign-up at 01:00 UTC as the day *before*.
 */

/** Matches SQLite's `YYYY-MM-DD HH:MM:SS`, with optional fractional seconds. */
const SQLITE_DATETIME = /^(\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}(?::\d{2})?)(?:\.\d+)?$/;

/**
 * Parse a database timestamp into a `Date`, or `null` when it is absent or
 * unreadable.
 *
 * Strings that already carry a timezone (ISO-8601 with `Z` or an offset, which
 * is what an API serialising a proper timestamp would send) are handed to the
 * parser as they are; only the unmarked SQLite form gets `Z` appended.
 */
export function parseDbDateTime(value) {
	if (value == null) return null;
	// A caller measuring something that happened in this session (the moment a
	// fetch landed, say) already holds a real Date.
	if (value instanceof Date) {
		return Number.isNaN(value.getTime()) ? null : value;
	}
	// SQLite also hands out datetimes as integers (unix seconds); accept those
	// too rather than rendering a blank field for them.
	if (typeof value === 'number') {
		return Number.isFinite(value) ? new Date(value * 1000) : null;
	}
	const text = String(value).trim();
	if (!text) return null;

	const iso = text.includes('T') ? text : text.replace(' ', 'T');
	const parsed = new Date(SQLITE_DATETIME.test(iso) ? `${iso}Z` : iso);
	return Number.isNaN(parsed.getTime()) ? null : parsed;
}

/**
 * A record's calendar day — the one stored in the database — e.g.
 * `Sep 29, 2026`.
 *
 * Rendered in UTC on purpose, so the same record shows the same date to every
 * reader. A day is a property of the record, not of whoever is looking at it:
 * rendering it locally would let a reader east of Greenwich see a post
 * published late on the 29th as "Sep 30".
 *
 * `locale` defaults to `en-US` to match the formatting these strings had before
 * they came here; pass a BCP-47 tag for anything else. `timeZone` defaults to
 * UTC for the reason above; pass the reader's own zone only when the question
 * really is "what day is it for them" rather than "what day is this record on".
 */
export function formatDateOnly(value, { locale = 'en-US', timeZone = 'UTC' } = {}) {
	const date = parseDbDateTime(value);
	if (!date) return '';

	return date.toLocaleDateString(locale, {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		timeZone
	});
}

/**
 * A coarse "how long ago" phrase, e.g. `just now`, `12 min ago`, `3 days ago`.
 *
 * Deliberately hand-rolled rather than pulled from `Intl.RelativeTimeFormat`:
 * the thresholds are tuned for an editor scanning a list of recently touched
 * items, and a floor at `just now` reads better than rounding a 30-second gap
 * down to "0 seconds ago".
 */
export function formatRelative(value, now = Date.now()) {
	const date = parseDbDateTime(value);
	if (!date) return '';

	const seconds = Math.round((now - date.getTime()) / 1000);
	// A timestamp in the future means clock skew or a wrong timezone read, not
	// something the reader needs explained; treat it as brand new.
	if (seconds < 45) return 'just now';

	const minutes = Math.round(seconds / 60);
	if (minutes < 60) return `${minutes} min ago`;

	const hours = Math.round(minutes / 60);
	if (hours < 24) return hours === 1 ? '1 hour ago' : `${hours} hours ago`;

	const days = Math.round(hours / 24);
	if (days < 30) return days === 1 ? '1 day ago' : `${days} days ago`;

	const months = Math.round(days / 30);
	if (months < 12) return months === 1 ? '1 month ago' : `${months} months ago`;

	const years = Math.round(months / 12);
	return years === 1 ? '1 year ago' : `${years} years ago`;
}

/**
 * A timestamp as an absolute date **and** time, e.g. `29 Sep 2026, 21:04`.
 *
 * `style: 'long'` spells the month out and keeps the year, for places that need
 * to read as a complete moment rather than a compact field value.
 */
export function formatDateTime(value, { locale = 'en', style = 'short' } = {}) {
	const date = parseDbDateTime(value);
	if (!date) return '';

	const options =
		style === 'long'
			? { dateStyle: 'long', timeStyle: 'short' }
			: { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' };

	// `en-GB` orders the day before the month and uses a 24-hour clock, which
	// is how the dashboard's short dates already read; the default `en-US`
	// ordering does neither.
	const tag = locale === 'vi' ? 'vi-VN' : 'en-GB';
	return date.toLocaleString(tag, { ...options, hour12: locale !== 'vi' });
}

/**
 * What an editor actually wants next to a "last updated" label: how long ago,
 * with the exact date and time available alongside it. Past a week the relative
 * half stops earning its space and only the absolute moment is shown.
 */
export function formatLastUpdated(value, { locale = 'en', now = Date.now() } = {}) {
	const date = parseDbDateTime(value);
	if (!date) return '';

	const age = now - date.getTime();
	const absolute = formatDateTime(value, { locale });
	if (age > 7 * 24 * 60 * 60 * 1000) return absolute;

	return `${formatRelative(date, now)} · ${absolute}`;
}

/**
 * The machine-readable value for a `<time datetime>` attribute, or an empty
 * string when there is nothing to describe.
 */
export function toDateTimeAttr(value) {
	return parseDbDateTime(value)?.toISOString() ?? '';
}
