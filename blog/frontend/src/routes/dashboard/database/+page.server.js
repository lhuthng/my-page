import { redirect } from "@sveltejs/kit";
import { env } from "$env/dynamic/private";
import { getGqlClient } from "$lib/graphql.js";

// ---------------------------------------------------------------------------
// GraphQL query strings
// ---------------------------------------------------------------------------

const DB_STATS_QUERY = `
  query DbStats {
    dbStats {
      totalUsers
      totalPosts
      totalComments
      totalMedia
      totalSeries
      totalTags
      totalCategories
    }
  }
`;

const QUERIES = {
  users: `
    query Users($limit: Int, $offset: Int, $search: String, $role: String) {
      users(limit: $limit, offset: $offset, search: $search, role: $role) {
        total
        items {
          id username email role displayName bio avatarUrl createdAt
        }
      }
    }
  `,

  posts: `
    query Posts($limit: Int, $offset: Int, $search: String, $status: String) {
      posts(limit: $limit, offset: $offset, search: $search, status: $status) {
        total
        items {
          id title slug status authorName authorSlug seriesTitle
          viewCount isFeatured publishedAt createdAt updatedAt excerpt
        }
      }
    }
  `,

  comments: `
    query Comments($limit: Int, $offset: Int, $includeDeleted: Boolean) {
      comments(limit: $limit, offset: $offset, includeDeleted: $includeDeleted) {
        total
        items {
          id content postTitle postSlug authorName authorUsername
          parentId isDeleted createdAt
        }
      }
    }
  `,

  media: `
    query Media($limit: Int, $offset: Int, $search: String) {
      media(limit: $limit, offset: $offset, search: $search) {
        total
        items {
          id shortName fileName fileType url size description
          uploaderName useCount createdAt
        }
      }
    }
  `,

  series: `
    query Series($limit: Int, $offset: Int) {
      series(limit: $limit, offset: $offset) {
        total
        items {
          id title slug description postCount createdAt
        }
      }
    }
  `,

  tags: `
    query Tags($limit: Int, $offset: Int) {
      tags(limit: $limit, offset: $offset) {
        total
        items {
          id name slug postCount
        }
      }
    }
  `,

  categories: `
    query Categories($limit: Int, $offset: Int) {
      categories(limit: $limit, offset: $offset) {
        total
        items {
          id name slug description postCount
        }
      }
    }
  `,
};

const VALID_TABLES = [
  "users",
  "posts",
  "comments",
  "media",
  "series",
  "tags",
  "categories",
];

/**
 * Build the GraphQL variables object for a given table and filter state.
 */
function buildVariables(
  table,
  { limit, offset, search, status, role, includeDeleted },
) {
  const base = { limit, offset };
  switch (table) {
    case "users":
      return { ...base, search: search || null, role: role || null };
    case "posts":
      return { ...base, search: search || null, status: status || null };
    case "comments":
      return { ...base, includeDeleted };
    case "media":
      return { ...base, search: search || null };
    default:
      // series, tags, categories — no extra filters
      return base;
  }
}

// ---------------------------------------------------------------------------
// Load function
// ---------------------------------------------------------------------------

export async function load(event) {
  // Get role from the dashboard layout's JWT-decoded data.
  // This is reliable on both SSR (HTML) and client-side navigation (JSON)
  // requests, unlike event.locals.user which is only set on HTML requests.
  const { accessToken, role } = await event.parent();

  if (role !== "admin") {
    redirect(303, "/dashboard");
  }

  // Read URL search params that drive the current view
  const rawTable = event.url.searchParams.get("table") || "users";
  const table = VALID_TABLES.includes(rawTable) ? rawTable : "users";

  const page = Math.max(
    1,
    parseInt(event.url.searchParams.get("page") || "1", 10),
  );
  const search = event.url.searchParams.get("search") || "";
  const status = event.url.searchParams.get("status") || "";
  const role_filter = event.url.searchParams.get("role") || "";
  const includeDeleted =
    event.url.searchParams.get("includeDeleted") === "true";

  const limit = 20;
  const offset = (page - 1) * limit;

  const client = getGqlClient(event.fetch, env.API_URL, accessToken.token);

  const variables = buildVariables(table, {
    limit,
    offset,
    search,
    status,
    role: role_filter,
    includeDeleted,
  });

  try {
    const [statsResult, tableResult] = await Promise.all([
      client.request(DB_STATS_QUERY),
      client.request(QUERIES[table], variables),
    ]);

    return {
      dbStats: statsResult.dbStats,
      table,
      page,
      search,
      status,
      role: role_filter,
      includeDeleted,
      // GraphQL returns { users: { items, total } } etc. — key matches table name
      tableData: tableResult[table] ?? { items: [], total: 0 },
    };
  } catch (e) {
    console.error("[database] GraphQL error:", e);
    return {
      dbStats: null,
      table,
      page,
      search,
      status,
      role: role_filter,
      includeDeleted,
      tableData: { items: [], total: 0 },
    };
  }
}
