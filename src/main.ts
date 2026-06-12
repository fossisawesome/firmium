import './style.css'
import App from './App.svelte'
import { mount } from 'svelte'
import { AudioBridge } from './lib/audio-bridge'
import { audioBridge } from './lib/stores'
import { wireBridgeEvents } from './lib/playback'

const bridge = new AudioBridge()
wireBridgeEvents(bridge)
audioBridge.set(bridge)

// Svelte 5 uses mount() instead of new Component()
const app = mount(App, { target: document.getElementById('app')! })

export default app
