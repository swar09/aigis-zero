export interface PaginationQuery {
  page?: number;
  limit?: number;
}

export interface PaginationResult {
  page: number;
  limit: number;
  offset: number;
}

export function getPagination(
  query: PaginationQuery,
): PaginationResult {
  const page = Math.max(1, query.page ?? 1);
  const limit = Math.min(100, Math.max(1, query.limit ?? 20));

  return {
    page,
    limit,
    offset: (page - 1) * limit,
  };
}