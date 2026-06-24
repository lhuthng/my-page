import { GraphQLClient } from 'graphql-request';
import { auth } from '$lib/auth/user.svelte.js';

let client;

function getClient() {
	if (client) return client;
	const url = globalThis.location
		? `${globalThis.location.origin}/api/graphql`
		: '/api/graphql';
	client = new GraphQLClient(url, {
		requestMiddleware: (req) => ({
			...req,
			headers: { ...req.headers, Authorization: auth() }
		})
	});
	return client;
}

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

const Q = {
	dashboardPosts: `
	  query DashboardPosts($limit: Int, $offset: Int, $search: String) {
	    dashboardPosts(limit: $limit, offset: $offset, search: $search) {
	      total
	      items { id title slug excerpt status views likes commentsCount authorName authorSlug tagSlugs coverUrl coverMediaType }
	    }
	  }`,
	dashboardProjects: `
	  query DashboardProjects($limit: Int, $offset: Int, $search: String) {
	    dashboardProjects(limit: $limit, offset: $offset, search: $search) {
	      total
	      items { id postId title slug excerpt status demoType views likes commentsCount authorName authorSlug tagSlugs coverUrl coverMediaType }
	    }
	  }`,
	users: `
	  query Users($limit: Int, $offset: Int, $search: String, $role: String) {
	    users(limit: $limit, offset: $offset, search: $search, role: $role) {
	      total
	      items { id username email role displayName avatarUrl createdAt }
	    }
	  }`,
	roleCounts: `
	  query { overview { roleCounts { admin moderator user } } }`,
	series: `
	  query Series($limit: Int, $offset: Int) {
	    series(limit: $limit, offset: $offset) {
	      items { id title slug description coverUrl ownerUsername postCount createdAt }
	    }
	  }`,
	seriesPosts: `
	  query SeriesPosts($seriesId: Int!) {
	    seriesPosts(seriesId: $seriesId) {
	      postId title slug status number coverUrl
	    }
	  }`,
	overview: `
	  query Overview {
	    overview {
	      totalPublished totalDrafts totalUsers totalComments
	      topPostsByViews { id title slug excerpt status views likes commentsCount authorName authorSlug tagNames tagSlugs coverUrl coverMediaType }
	      topPostsByLikes { id title slug excerpt status views likes commentsCount authorName authorSlug tagNames tagSlugs coverUrl coverMediaType }
	      topPostsByComments { id title slug excerpt status views likes commentsCount authorName authorSlug tagNames tagSlugs coverUrl coverMediaType }
	      recentPosts { id title slug excerpt status views likes commentsCount authorName authorSlug tagNames tagSlugs coverUrl coverMediaType }
	      recentUsers { username displayName role avatarUrl createdAt }
	      roleCounts { admin moderator user }
	      growth { date newPosts newUsers }
	    }
	  }`
};

export function fixUrl(path) {
	if (!path || path.includes('://') || path.startsWith('/')) return path;
	return `/api/${path}`;
}

function fixUrls(data) {
	if (Array.isArray(data)) return data.map(fixUrls);
	if (data && typeof data === 'object') {
		const result = {};
		for (const [key, value] of Object.entries(data)) {
			result[key] = fixUrls(value);
		}
		return result;
	}
	if (typeof data === 'string' && /^media\//.test(data)) return fixUrl(data);
	return data;
}

export const gql = {
	request(query, variables) {
		return getClient().request(query, variables);
	},
	dashboardPosts(vars) { return this.request(Q.dashboardPosts, vars).then(fixUrls); },
	dashboardProjects(vars) { return this.request(Q.dashboardProjects, vars).then(fixUrls); },
	users(vars) { return this.request(Q.users, vars).then(fixUrls); },
	roleCounts() { return this.request(Q.roleCounts); },
	series(vars) { return this.request(Q.series, vars).then(fixUrls); },
	seriesPosts(vars) { return this.request(Q.seriesPosts, vars).then(fixUrls); },
	overview() { return this.request(Q.overview).then(fixUrls); }
};
