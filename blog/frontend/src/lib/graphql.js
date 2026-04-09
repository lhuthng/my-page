import { GraphQLClient } from 'graphql-request';

/**
 * Creates a GraphQLClient pointed at the backend's /graphql endpoint.
 *
 * @param {typeof fetch} fetch   - The SvelteKit `fetch` (from load event) so cookies/headers are forwarded correctly.
 * @param {string}       apiUrl  - Base API URL (e.g. "http://localhost:3000"), taken from env.API_URL.
 * @param {string}       token   - Raw bearer token string (accessToken.token from locals/parent).
 */
export function getGqlClient(fetch, apiUrl, token) {
  return new GraphQLClient(`${apiUrl}/graphql`, {
    fetch,
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}
