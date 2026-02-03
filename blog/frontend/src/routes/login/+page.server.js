export async function load({ url, setHeaders }) {
  const register = url.searchParams.get("register");
  setHeaders({
    "cache-control": "public, max-age=60, s-maxage=60",
  });
  return { register };
}
