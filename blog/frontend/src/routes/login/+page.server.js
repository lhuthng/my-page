export async function load(event) {
  const register = event.url.searchParams.get("register");
  return { register };
}
