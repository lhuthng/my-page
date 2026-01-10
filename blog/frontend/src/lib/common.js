import TimeAgo from "javascript-time-ago";
import en from "javascript-time-ago/locale/en";

export function preventDefault(e) {
  e.preventDefault();
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
