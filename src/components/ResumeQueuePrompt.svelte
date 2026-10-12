<script lang="ts">
  import { Api } from '../lib/api'
  import { toastError } from '../lib/stores'

  let { onDismiss }: { onDismiss: () => void } = $props()

  async function resume() {
    try { await Api.resumeRemoteQueue() } catch (e) { toastError(e) }
    onDismiss()
  }
</script>

<div class="resume-queue-prompt">
  <span>Resume queue?</span>
  <button class="resume-queue-btn" onclick={resume}>Resume</button>
  <button class="resume-queue-dismiss" onclick={onDismiss}>Dismiss</button>
</div>
