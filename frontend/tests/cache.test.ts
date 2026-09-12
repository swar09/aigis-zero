import { describe, it, expect, beforeEach, vi } from 'vitest';
import { QueryCache } from '../lib/cache';

describe('QueryCache', () => {
  let cache: QueryCache;

  beforeEach(() => {
    cache = new QueryCache(5000);
  });

  it('stores and retrieves cached values', () => {
    cache.set('test-key', { count: 42 });
    expect(cache.get('test-key')).toEqual({ count: 42 });
  });

  it('returns null for missing keys', () => {
    expect(cache.get('non-existent')).toBeNull();
  });

  it('correctly reports freshness based on TTL', () => {
    cache.set('fresh-key', 'data');
    expect(cache.isFresh('fresh-key', 1000)).toBe(true);
    expect(cache.isFresh('fresh-key', 0)).toBe(false);
  });

  it('invalidates specific keys and patterns', () => {
    cache.set('user:1', 'Alice');
    cache.set('user:2', 'Bob');
    cache.set('node:1', 'ServerA');

    cache.invalidate('user:1');
    expect(cache.get('user:1')).toBeNull();
    expect(cache.get('user:2')).toBe('Bob');

    cache.invalidatePattern(/^user:/);
    expect(cache.get('user:2')).toBeNull();
    expect(cache.get('node:1')).toBe('ServerA');
  });

  it('deduplicates in-flight requests for the same key', async () => {
    const fetcher = vi.fn().mockImplementation(async () => {
      await new Promise((r) => setTimeout(r, 50));
      return { result: 'success' };
    });

    const promise1 = cache.fetchWithDedupe('shared-key', fetcher);
    const promise2 = cache.fetchWithDedupe('shared-key', fetcher);

    const [res1, res2] = await Promise.all([promise1, promise2]);

    expect(res1).toEqual({ result: 'success' });
    expect(res2).toEqual({ result: 'success' });
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
});
