import TimeAgo from "javascript-time-ago";
import en from "javascript-time-ago/locale/en";

export const widthThreshold = {
  lg: 1024,
};

export function preventDefault(e) {
  e.preventDefault();
}

export function stopPropagation(e) {
  e.stopPropagation;
}

export const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

export function textToDate(text) {
  const dt = new Date(text.split(" ")[0]);
  const options = { year: "numeric", month: "short", day: "numeric" };
  return dt.toLocaleDateString("en-US", options);
}

export function nowToDate() {
  const dt = new Date();
  const options = { year: "numeric", month: "short", day: "numeric" };
  return dt.toLocaleDateString("en-US", options);
}

export function dateTillNow(date, format = "mini") {
  const localDate = new Date(date.replace(" ", "T"));
  return time.format(localDate, format);
}

export function arraysEqualIgnoreOrder(a, b) {
  return (
    a.length === b.length &&
    [...a].sort().every((val, i) => val === [...b].sort()[i])
  );
}

TimeAgo.addDefaultLocale(en);

export const time = new TimeAgo("en-US");

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
  if (!str || typeof str !== "string") return NaN;

  const cleaned = str.replace(/[% \s]/g, "").trim();

  if (!cleaned) return NaN;

  const num = parseFloat(cleaned);
  return isNaN(num) ? NaN : num / 100;
}
