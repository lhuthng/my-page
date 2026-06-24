export const OVERVIEW_QUERY = `
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
  }
`;

export const USERS_QUERY = `
  query Users($limit: Int, $offset: Int, $search: String, $role: String) {
    users(limit: $limit, offset: $offset, search: $search, role: $role) {
      total
      items {
        id username email role displayName bio avatarUrl createdAt
      }
    }
  }
`;

export const FEATURED_POSTS_QUERY = `
  query FeaturedPosts($limit: Int) {
    featuredPosts(limit: $limit) {
      id title slug excerpt coverUrl coverMediaType authorName authorSlug views likes commentsCount
    }
  }
`;

export const FEATURED_PROJECTS_QUERY = `
  query FeaturedProjects($limit: Int) {
    featuredProjects(limit: $limit) {
      id title slug excerpt coverUrl coverMediaType authorName authorSlug demoType views likes commentsCount
    }
  }
`;
