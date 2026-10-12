// Some libs (e.g. SafeStorage in src/lib/utils.ts) reference `localStorage`/`document`
// as bare globals; jsdom exposes them on `window` but not always on `globalThis`.
for (const key of ['localStorage', 'document'] as const) {
  if (typeof (globalThis as Record<string, unknown>)[key] === 'undefined') {
    Object.defineProperty(globalThis, key, {
      value: window[key],
      configurable: true,
      writable: true,
    })
  }
}
