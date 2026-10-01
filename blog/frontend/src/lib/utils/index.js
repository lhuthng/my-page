import TimeAgo from 'javascript-time-ago';
import en from 'javascript-time-ago/locale/en';
import { formatDateOnly, parseDbDateTime } from './datetime.js';

export const widthThreshold = {
	lg: 1024
};

export function preventDefault(e) {
	e.preventDefault();
}

export function stopPropagation(e) {
	e.stopPropagation();
}

export const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

/**
 * A record's stored calendar day, e.g. `Sep 29, 2026`.
 *
 * Delegates to the shared formatter, which parses the database's UTC timestamp
 * and renders the day in UTC. The old `text.split(' ')[0]` + bare `new Date()`
 * looked right by accident: a date-only ISO string *is* parsed as UTC, but it
 * was then rendered in the reader's local zone, so anyone west of Greenwich saw
 * the previous day.
 */
export function textToDate(text) {
	return formatDateOnly(text);
}

/**
 * Today's date, in the reader's own timezone.
 *
 * Deliberately *not* UTC, unlike `textToDate`: this one seeds a new post's date
 * field, so the question is "what day is it for the author sitting here", and
 * answering in UTC would hand an author in Hanoi a yesterday-dated draft.
 */
export function nowToDate() {
	return formatDateOnly(new Date(), { timeZone: undefined });
}

/**
 * How long ago something happened, e.g. `3 hours ago`.
 *
 * The instant is parsed as UTC (it used to be read as local time, which threw
 * the computed age off by the reader's offset), and the result is a difference
 * between two instants — so it is the same in every timezone.
 */
export function dateTillNow(date, format = 'mini') {
	const parsed = parseDbDateTime(date);
	if (!parsed) return '';
	return time.format(parsed, format);
}

export function arraysEqualIgnoreOrder(a, b) {
	return a.length === b.length && [...a].sort().every((val, i) => val === [...b].sort()[i]);
}

TimeAgo.addDefaultLocale(en);

export const time = new TimeAgo('en-US');

function sign(px, py, x1, y1, x2, y2) {
	return (px - x2) * (y1 - y2) - (x1 - x2) * (py - y2);
}

export function isPointInTriangle(px, py, x1, y1, x2, y2, x3, y3) {
	const d1 = sign(px, py, x1, y1, x2, y2);
	const d2 = sign(px, py, x2, y2, x3, y3);
	const d3 = sign(px, py, x3, y3, x1, y1);
	const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
	const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
	return !(hasNeg && hasPos);
}

export function percentToDecimal(str) {
	if (!str || typeof str !== 'string') return NaN;

	const cleaned = str.replace(/[% \s]/g, '').trim();

	if (!cleaned) return NaN;

	const num = parseFloat(cleaned);
	return isNaN(num) ? NaN : num / 100;
}
