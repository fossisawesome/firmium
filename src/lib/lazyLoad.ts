import type { Action } from 'svelte/action'

// Svelte action that uses IntersectionObserver to lazy-load cover art images.
// Usage: <img use:lazyLoad={loadFn} alt="">
// loadFn is called with the img element when it enters the viewport.
export const lazyLoad: Action<HTMLImageElement, (node: HTMLImageElement) => void> = (node, loadFn) => {
  const observer = new IntersectionObserver(entries => {
    entries.forEach(e => {
      if (e.isIntersecting) {
        observer.unobserve(node)
        loadFn(node)
      }
    })
  }, { rootMargin: '100px' })

  observer.observe(node)

  return {
    update(newLoadFn) { loadFn = newLoadFn },
    destroy() { observer.unobserve(node) }
  }
}
