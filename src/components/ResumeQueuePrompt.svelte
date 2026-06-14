<script lang="ts">
  import { setQueueSeamless } from '../lib/playback'
  import { audioBridge } from '../lib/stores'
  import { get } from 'svelte/store'
  import type { RemotePlayQueue } from '../lib/types/tauri-commands'

  let { remoteQueue, onDismiss }: { remoteQueue: RemotePlayQueue, onDismiss: () => void } = $props()

  function resume() {
    const idx = remoteQueue.current ? remoteQueue.entries.findIndex(t => t.id === remoteQueue.current) : 0
    setQueueSeamless(remoteQueue.entries, idx >= 0 ? idx : 0)
    const positionSec = (remoteQueue.positionMs ?? 0) / 1000
    if (positionSec > 0) {
      setTimeout(() => { get(audioBridge)?.seek(positionSec) }, 300)
    }
    onDismiss()
  }
</script>

<div class="resume-queue-prompt">
  <span>Resume queue from another device?</span>
  <button class="resume-queue-btn" onclick={resume}>Resume</button>
  <button class="resume-queue-dismiss" onclick={onDismiss}>Dismiss</button>
</div>
