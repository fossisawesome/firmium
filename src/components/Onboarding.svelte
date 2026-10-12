<script lang="ts">
  interface Props {
    onFinish: () => void
  }
  let { onFinish }: Props = $props()

  type Panel = { title: string; body: string }

  // Panel 0 is the welcome/logo panel; the rest are feature highlights.
  const panels: Panel[] = [
    { title: 'Welcome to Firmium', body: 'Your music, your server.' },
    { title: 'Your music, your way', body: 'Connect any OpenSubsonic or Navidrome server, or play your local files. No lock-in, nothing uploaded.' },
    { title: 'Built for listening', body: 'Gapless playback and smooth crossfade transitions.' },
    { title: 'Make it yours', body: 'Light, dark, or your own custom theme.' },
    { title: 'Ready to go', body: 'Connect your server to start listening.' },
  ]

  let index = $state(0)
  const isLast = $derived(index === panels.length - 1)

  function next() {
    if (isLast) onFinish()
    else index += 1
  }
  function back() {
    if (index > 0) index -= 1
  }
</script>

<div class="onboarding">
  <div class="card">
    {#if index === 0}
      <svg class="logo" viewBox="0 0 1024 1024" aria-label="Firmium">
        <defs>
          <linearGradient id="onboardHex" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="#e8c97e" />
            <stop offset="100%" stop-color="#863bff" />
          </linearGradient>
        </defs>
        <polygon points="512,128 838,320 838,704 512,896 186,704 186,320"
          fill="none" stroke="url(#onboardHex)" stroke-width="56" stroke-linejoin="round" />
      </svg>
    {/if}

    <h1>{panels[index].title}</h1>
    <p>{panels[index].body}</p>

    <div class="dots">
      {#each panels as _, i}
        <span class="dot" class:active={i === index}></span>
      {/each}
    </div>

    <div class="controls">
      <button class="ghost" onclick={back} disabled={index === 0}>Back</button>
      {#if !isLast}
        <button class="ghost skip" onclick={onFinish}>Skip</button>
      {/if}
      <button class="primary" onclick={next}>{isLast ? 'Connect your server' : 'Next'}</button>
    </div>
  </div>
</div>

<style>
  .onboarding {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
    font-family: var(--font);
    color: var(--text);
  }
  .card {
    width: min(520px, 90vw);
    text-align: center;
    padding: 40px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }
  .logo { width: 120px; height: 120px; }
  h1 { margin: 0; font-size: 1.6rem; }
  p { margin: 0; color: var(--muted); line-height: 1.5; max-width: 40ch; }
  .dots { display: flex; gap: 8px; margin: 8px 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--border); transition: background var(--timing); }
  .dot.active { background: var(--accent); }
  .controls { display: flex; gap: 12px; align-items: center; margin-top: 8px; }
  button { font-family: var(--font); cursor: pointer; border-radius: 8px; padding: 10px 18px; font-size: 0.95rem; transition: all var(--timing); }
  .primary { background: var(--accent); color: var(--bg); border: none; }
  .primary:hover { background: var(--accent-dim); }
  .ghost { background: transparent; color: var(--muted); border: 1px solid var(--border); }
  .ghost:hover:not(:disabled) { color: var(--text); border-color: var(--text); }
  .ghost:disabled { opacity: 0.4; cursor: default; }
  .skip { border: none; }
</style>
