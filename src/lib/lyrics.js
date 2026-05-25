// Converts an LRC timestamp "[mm:ss.xx]" or "[mm:ss.xxx]" to milliseconds.
export const parseLrcTimestamp = (mm, ss, frac) => {
  const fracMs = frac.length === 2 ? parseInt(frac, 10) * 10 : parseInt(frac, 10)
  return (parseInt(mm, 10) * 60 + parseInt(ss, 10)) * 1000 + fracMs
}

// Parse an LRC-format string into an array of { start: ms, value: string }.
export const parseLrc = (lrcText) => {
  const lines = []
  for (const raw of lrcText.split('\n')) {
    const m = raw.match(/^\[(\d{1,2}):(\d{2})\.(\d{2,3})\]\s*(.*)/)
    if (m) lines.push({ start: parseLrcTimestamp(m[1], m[2], m[3]), value: m[4] })
  }
  return lines.sort((a, b) => a.start - b.start)
}

// Normalize song title and artist for better lrclib matching.
const normalizeLrclibQuery = (song) => {
  let title = song.title.replace(/\s*[\(\[](?:Remix|Live|Extended|Acoustic|Instrumental|Remaster|Cover|Edit|Version|feat\.?|featuring)[^\)\]]*[\)\]]/gi, '').trim()
  title = title.replace(/\s*-\s*(?:feat\.|featuring).*$/i, '').trim()
  let artist = song.artist.split(/\s*(?:feat\.|feat|featuring|ft\.?|\/)\s*/i)[0].trim()
  return { artist, title }
}

// lrclib.net — free, no API key, returns synced LRC lyrics.
export const LrclibApi = {
  getLyrics: async (song) => {
    const { artist, title } = normalizeLrclibQuery(song)
    const params = new URLSearchParams({
      artist_name: artist,
      track_name: title,
      duration: String(Math.round(song.duration ?? 0))
    })
    const res = await fetch(`https://lrclib.net/api/get?${params}`, {
      headers: { 'Lrclib-Client': 'Firmium (https://github.com/fossisawesome/firmium)' }
    })
    if (res.status === 404) return null
    if (!res.ok) throw new Error(`LRCLIB ${res.status}`)
    const data = await res.json()
    if (data.instrumental) return { lines: [{ start: 0, value: '♪ Instrumental ♪' }], synced: false }
    if (data.syncedLyrics) {
      const lines = parseLrc(data.syncedLyrics)
      if (lines.length) return { lines, synced: true }
    }
    if (data.plainLyrics) {
      return {
        lines: data.plainLyrics.split('\n').map(v => ({ start: 0, value: v })),
        synced: false
      }
    }
    return null
  }
}
