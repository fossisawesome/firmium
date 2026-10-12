import { describe, it, expect, vi } from 'vitest'

const onDestroyCallbacks: (() => void)[] = []

vi.mock('svelte', () => ({
  onDestroy: (fn: () => void) => { onDestroyCallbacks.push(fn) },
}))

const { createAbortController } = await import('./utils')

describe('createAbortController', () => {
  it('renew() returns a fresh, non-aborted signal each time', () => {
    const ctrl = createAbortController()
    const first = ctrl.renew()
    expect(first.aborted).toBe(false)
    expect(ctrl.signal).toBe(first)

    const second = ctrl.renew()
    expect(second).not.toBe(first)
    expect(second.aborted).toBe(false)
    expect(first.aborted).toBe(true)
    expect(ctrl.signal).toBe(second)
  })

  it('aborts the current signal when the component is destroyed', () => {
    onDestroyCallbacks.length = 0
    const ctrl = createAbortController()
    const signal = ctrl.renew()
    expect(signal.aborted).toBe(false)

    onDestroyCallbacks.forEach(cb => cb())
    expect(signal.aborted).toBe(true)
  })
})
