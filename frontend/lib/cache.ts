/**
 * Aigis-Zero Client-Side Query Cache
 * Lightweight in-memory cache with in-flight request deduplication
 * and TTL-based stale-while-revalidate data delivery.
 */

interface CacheEntry<T> {
  data: T;
  timestamp: number;
}

export class QueryCache {
  private cache = new Map<string, CacheEntry<unknown>>();
  private inFlight = new Map<string, Promise<unknown>>();
  private defaultTtlMs: number;

  constructor(defaultTtlMs = 15000) {
    this.defaultTtlMs = defaultTtlMs;
  }

  get<T>(key: string): T | null {
    const entry = this.cache.get(key);
    if (!entry) return null;
    return entry.data as T;
  }

  isFresh(key: string, ttlMs?: number): boolean {
    const entry = this.cache.get(key);
    if (!entry) return false;
    const ttl = ttlMs ?? this.defaultTtlMs;
    return Date.now() - entry.timestamp < ttl;
  }

  set<T>(key: string, data: T): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
    });
  }

  invalidate(key: string): void {
    this.cache.delete(key);
  }

  invalidatePattern(pattern: RegExp): void {
    for (const key of this.cache.keys()) {
      if (pattern.test(key)) {
        this.cache.delete(key);
      }
    }
  }

  clear(): void {
    this.cache.clear();
    this.inFlight.clear();
  }

  async fetchWithDedupe<T>(
    key: string,
    fetcher: () => Promise<T>,
    ttlMs?: number
  ): Promise<T> {
    if (this.isFresh(key, ttlMs)) {
      const cached = this.get<T>(key);
      if (cached !== null) return cached;
    }

    const existingPromise = this.inFlight.get(key);
    if (existingPromise) {
      return existingPromise as Promise<T>;
    }

    const promise = fetcher()
      .then((data) => {
        this.set(key, data);
        this.inFlight.delete(key);
        return data;
      })
      .catch((err) => {
        this.inFlight.delete(key);
        throw err;
      });

    this.inFlight.set(key, promise);
    return promise;
  }
}

export const globalQueryCache = new QueryCache();
