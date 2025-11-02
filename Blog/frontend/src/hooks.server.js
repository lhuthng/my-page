export async function handle({ event, resolve }) {
  const accept = event.request.headers.get("accept") ?? "";
  if (accept.includes("text/html")) {
    const refreshToken = event.cookies.get("refresh-token");
  }
  return resolve(event);
}
