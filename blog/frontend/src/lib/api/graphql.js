import { GraphQLClient } from 'graphql-request';

export function getGqlClient(fetch, apiUrl, token) {
	return new GraphQLClient(`${apiUrl}/graphql`, {
		fetch,
		requestMiddleware: (request) => ({
			...request,
			headers: {
				...request.headers,
				...(token ? { Authorization: `Bearer ${token}` } : {})
			}
		})
	});
}
