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
