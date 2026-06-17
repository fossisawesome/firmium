<script lang="ts">
  import { tauriInvoke } from '../lib/tauri'
  import type { RemotePlayQueue } from '../lib/types/tauri-commands'

  let { remoteQueue, onDismiss }: { remoteQueue: RemotePlayQueue, onDismiss: () => void } = $props()

  async function resume() {
    const idx = remoteQueue.current ? remoteQueue.entries.findIndex(t => t.id === remoteQueue.current) : 0
    await tauriInvoke('set_queue_seamless', { songs: remoteQueue.entries, startIdx: idx >= 0 ? idx : 0 }).catch(console.error)
    const positionSec = (remoteQueue.positionMs ?? 0) / 1000
    if (positionSec > 0) {
      setTimeout(() => tauriInvoke('seek_queue', { position: positionSec }).catch(console.error), 300)
    }
    onDismiss()
  }
</script>

<div class="resume-queue-prompt">
  <span>Resume queue?</span>
  <button class="resume-queue-btn" onclick={resume}>Resume</button>
  <button class="resume-queue-dismiss" onclick={onDismiss}>Dismiss</button>
</div>
