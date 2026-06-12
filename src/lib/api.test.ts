import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setAuth } from './stores'

vi.mock('./tauri', () => ({
  tauriInvoke: vi.fn().mockResolvedValue({}),
  tauriFetch: vi.fn(),
}))

const { Api } = await import('./api')
const { tauriFetch } = await import('./tauri')

const jsonResponse = (status: number, body: unknown): Response =>
  ({ status, ok: status >= 200 && status < 300, json: async () => body } as unknown as Response)

beforeEach(() => {
  vi.clearAllMocks()
  setAuth('http://example.com', 'user', 'pass')
})

describe('Api.fetch session expiry', () => {
  it('throws SESSION_EXPIRED and dispatches firmium:session-expired on HTTP 401', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(401, {}))
    const handler = vi.fn()
    window.addEventListener('firmium:session-expired', handler)

    await expect(Api.fetch('ping')).rejects.toMatchObject({ code: 'SESSION_EXPIRED' })
    expect(handler).toHaveBeenCalledTimes(1)

    window.removeEventListener('firmium:session-expired', handler)
  })

  it('treats status:"failed" with error code 40 as session expiry', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(200, {
      'subsonic-response': { status: 'failed', error: { code: 40, message: 'Wrong username or password' } },
    }))
    const handler = vi.fn()
    window.addEventListener('firmium:session-expired', handler)

    await expect(Api.fetch('ping')).rejects.toMatchObject({ code: 'SESSION_EXPIRED' })
    expect(handler).toHaveBeenCalledTimes(1)

    window.removeEventListener('firmium:session-expired', handler)
  })

  it('treats status:"failed" with error code 41 as session expiry', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(200, {
      'subsonic-response': { status: 'failed', error: { code: 41, message: 'Token expired' } },
    }))

    await expect(Api.fetch('ping')).rejects.toMatchObject({ code: 'SESSION_EXPIRED' })
  })

  it('throws a generic error for other error codes without session-expiry', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(200, {
      'subsonic-response': { status: 'failed', error: { code: 70, message: 'Not found' } },
    }))
    const handler = vi.fn()
    window.addEventListener('firmium:session-expired', handler)

    await expect(Api.fetch('ping')).rejects.toThrow('Not found')
    expect(handler).not.toHaveBeenCalled()

    window.removeEventListener('firmium:session-expired', handler)
  })

  it('does not dispatch firmium:session-expired when silentSessionExpiry is true', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(401, {}))
    const handler = vi.fn()
    window.addEventListener('firmium:session-expired', handler)

    await expect(Api.fetch('ping', {}, null, { silentSessionExpiry: true }))
      .rejects.toMatchObject({ code: 'SESSION_EXPIRED' })
    expect(handler).not.toHaveBeenCalled()

    window.removeEventListener('firmium:session-expired', handler)
  })

  it('returns the subsonic-response body on success', async () => {
    vi.mocked(tauriFetch).mockResolvedValue(jsonResponse(200, {
      'subsonic-response': { status: 'ok', albumList2: { album: [] } },
    }))

    const result = await Api.fetch('getAlbumList2')
    expect(result.status).toBe('ok')
  })
})
