import './style.css'
import App from './App.svelte'
import { mount } from 'svelte'
import { AudioBridge } from './lib/audio-bridge.js'
import { audioBridge } from './lib/stores.js'
import { wireBridgeEvents } from './lib/playback.js'

const bridge = new AudioBridge()
wireBridgeEvents(bridge)
audioBridge.set(bridge)

// Svelte 5 uses mount() instead of new Component()
const app = mount(App, { target: document.getElementById('app') })

export default app
