export function createRateLimiter({ maxRequests, windowMs }) {
  const store = new Map();

  setInterval(() => {
    const cutoff = Date.now() - windowMs;
    for (const [key, entries] of store) {
      const valid = entries.filter(t => t > cutoff);
      if (valid.length === 0) {
        store.delete(key);
      } else {
        store.set(key, valid);
      }
    }
  }, windowMs).unref();

  return {
    check(key) {
      const cutoff = Date.now() - windowMs;
      const entries = (store.get(key) ?? []).filter(t => t > cutoff);
      if (entries.length >= maxRequests) {
        return { allowed: false, remaining: 0, resetAfter: windowMs };
      }
      entries.push(Date.now());
      store.set(key, entries);
      return { allowed: true, remaining: maxRequests - entries.length, resetAfter: windowMs };
    }
  };
}
