// True when running inside the Android WebView (Tauri mobile build).
// Checked once at startup; never changes during a session.
export const isMobile = typeof navigator !== 'undefined' && /android/i.test(navigator.userAgent)
