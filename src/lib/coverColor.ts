export interface OrbPalette {
  primary: { r: number; g: number; b: number }
  secondary: { r: number; g: number; b: number }
  tertiary: { r: number; g: number; b: number }
}

const DEFAULT_ORB_PALETTE: OrbPalette = {
  primary: { r: 124, g: 92, b: 255 },
  secondary: { r: 170, g: 136, b: 255 },
  tertiary: { r: 85, g: 51, b: 204 },
}

export function extractOrbPalette(imageUrl: string): Promise<OrbPalette> {
  return new Promise((resolve) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      try {
        const size = 64
        const canvas = document.createElement('canvas')
        canvas.width = size; canvas.height = size
        const ctx = canvas.getContext('2d')
        if (!ctx) { resolve(DEFAULT_ORB_PALETTE); return }
        ctx.drawImage(img, 0, 0, size, size)
        const data = ctx.getImageData(0, 0, size, size).data

        interface Bucket { count: number; sr: number; sg: number; sb: number; sat: number }
        const buckets = new Map<string, Bucket>()
        for (let i = 0; i < data.length; i += 4) {
          if (data[i + 3] < 128) continue
          const r = data[i], g = data[i + 1], b = data[i + 2]
          const key = `${r >> 4},${g >> 4},${b >> 4}`
          const max = Math.max(r, g, b), min = Math.min(r, g, b)
          const sat = max > 0 ? (max - min) / max : 0
          let bkt = buckets.get(key)
          if (!bkt) { bkt = { count: 0, sr: 0, sg: 0, sb: 0, sat }; buckets.set(key, bkt) }
          bkt.count++; bkt.sr += r; bkt.sg += g; bkt.sb += b
        }

        const sorted = Array.from(buckets.values())
          .filter(b => b.count > 2)
          .map(b => ({ r: Math.round(b.sr / b.count), g: Math.round(b.sg / b.count), b: Math.round(b.sb / b.count), score: b.sat * b.count }))
          .sort((a, b) => b.score - a.score)

        if (sorted.length === 0) { resolve(DEFAULT_ORB_PALETTE); return }

        const dist = (a: { r: number; g: number; b: number }, c: { r: number; g: number; b: number }) =>
          Math.abs(a.r - c.r) + Math.abs(a.g - c.g) + Math.abs(a.b - c.b)

        const primary = sorted[0]
        const secondary = sorted.find(c => dist(c, primary) > 80) ?? sorted[Math.min(1, sorted.length - 1)]
        const tertiary = sorted.find(c => dist(c, primary) > 60 && dist(c, secondary) > 60) ?? sorted[Math.min(2, sorted.length - 1)]

        resolve({ primary, secondary, tertiary })
      } catch (_) {
        resolve(DEFAULT_ORB_PALETTE)
      }
    }
    img.onerror = () => resolve(DEFAULT_ORB_PALETTE)
    img.src = imageUrl
  })
}

// Extracts a representative dominant color from an image, used to tint the
// lyrics panel's glow background based on the current track's cover art.
export function extractDominantColor(imageUrl: string): Promise<{ r: number; g: number; b: number } | null> {
  return new Promise((resolve) => {
    const img = new Image()
    img.crossOrigin = 'anonymous'
    img.onload = () => {
      try {
        const size = 32
        const canvas = document.createElement('canvas')
        canvas.width = size
        canvas.height = size
        const ctx = canvas.getContext('2d')
        if (!ctx) return resolve(null)
        ctx.drawImage(img, 0, 0, size, size)
        const data = ctx.getImageData(0, 0, size, size).data

        // Bucket pixels into a coarse 8x8x8 histogram and average the most
        // populous bucket — avoids landing on a single noisy/outlier pixel.
        const buckets = new Map<string, { count: number; r: number; g: number; b: number }>()
        for (let i = 0; i < data.length; i += 4) {
          const a = data[i + 3]
          if (a < 128) continue
          const r = data[i], g = data[i + 1], b = data[i + 2]
          const key = `${r >> 5},${g >> 5},${b >> 5}`
          const bucket = buckets.get(key) ?? { count: 0, r: 0, g: 0, b: 0 }
          bucket.count++
          bucket.r += r
          bucket.g += g
          bucket.b += b
          buckets.set(key, bucket)
        }

        let best: { count: number; r: number; g: number; b: number } | null = null
        for (const bucket of buckets.values()) {
          if (!best || bucket.count > best.count) best = bucket
        }
        if (!best) return resolve(null)
        resolve({
          r: Math.round(best.r / best.count),
          g: Math.round(best.g / best.count),
          b: Math.round(best.b / best.count),
        })
      } catch (e) {
        console.warn('extractDominantColor failed:', e)
        resolve(null)
      }
    }
    img.onerror = () => resolve(null)
    img.src = imageUrl
  })
}
