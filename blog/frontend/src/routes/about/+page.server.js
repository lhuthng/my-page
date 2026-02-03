export async function load({ setHeaders }) {
  setHeaders({
    "cache-control": "public, max-age=60, s-maxage=60",
  });
  return {};
}
