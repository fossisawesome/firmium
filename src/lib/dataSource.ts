// Picks between the server API and the local-library API depending on whether
// the user is connected. Kept in its own module (rather than stores.ts) so
// api.ts/localApi.ts can import store state without creating an import cycle.
import { derived } from 'svelte/store'
import { isAuthed } from './stores'
import { Api } from './api'
import { LocalApi } from './localApi'

export const dataSource = derived(isAuthed, $isAuthed => $isAuthed ? Api : LocalApi)
