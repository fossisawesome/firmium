// Svelte action that loads a cover image via the backend (disk-cached in Rust).
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

/** `<img use:cover={song.coverArtId}>` */
export function cover(img: HTMLImageElement, coverId: string | null | undefined) {
  let current = coverId
  const load = (id: string | null | undefined) => {
    current = id
    img.removeAttribute('src')
    if (!id) return
    resolve(id).then(url => { if (current === id) img.src = url }).catch(() => {})
  }
  load(coverId)
  return { update: load }
}

const DEFAULT_AVATAR = `data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' fill='%23888' viewBox='0 0 24 24'><path d='M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z'/></svg>`
export { DEFAULT_AVATAR }

const photos = new Map<string, Promise<string | null>>()

/** `<img use:artistPhoto={artist.id}>`: swaps in the server photo when one exists. */
export function artistPhoto(img: HTMLImageElement, artistId: string) {
  let current = artistId
  const load = (id: string) => {
    current = id
    img.src = DEFAULT_AVATAR
    let p = photos.get(id)
    if (!p) { p = Api.artistImage(id).catch(() => null); photos.set(id, p) }
    p.then(url => { if (url && current === id) img.src = url })
  }
  load(artistId)
  return { update: load }
}
