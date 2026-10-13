// Svelte attachments that loads a cover image via the backend (disk-cached in Rust).
import type { Attachment } from 'svelte/attachments'
import { Api } from './api'
import { fileUrl } from './ipc'

const resolved = new Map<string, Promise<string>>()

function resolve(coverId: string): Promise<string> {
  let p = resolved.get(coverId)
  if (!p) {
    p = Api.cover(coverId).then(fileUrl)
    p.catch(() => resolved.delete(coverId))
    resolved.set(coverId, p)
  }
  return p
}

/** `<img {@attach cover(song.coverArtId)}>` */
export function cover(id: string | null | undefined): Attachment<HTMLImageElement> {
  return img => {
    let live = true
    img.removeAttribute('src')
    if (id) resolve(id).then(url => { if (live) img.src = url }).catch(() => {})
    return () => { live = false }
  }
}

const DEFAULT_AVATAR = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='-4 -4 32 32'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>`
export { DEFAULT_AVATAR }

const photos = new Map<string, Promise<string | null>>()

/** `<img {@attach artistPhoto(artist.id)}>`: swaps in the server photo when one exists. */
export function artistPhoto(id: string): Attachment<HTMLImageElement> {
  return img => {
    let live = true
    img.src = DEFAULT_AVATAR
    let p = photos.get(id)
    if (!p) { p = Api.artistImage(id).catch(() => null); photos.set(id, p) }
    p.then(url => { if (url && live) img.src = url })
    return () => { live = false }
  }
}
