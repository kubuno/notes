import { useQuery } from '@tanstack/react-query'
import { api } from '@kubuno/sdk'

// Editor-side instance defaults, as the administrator left them in the console.
// These act inside the running editor (autosave cadence, spell check), so they
// are declared `public` in notes' module.toml — which is what puts them in
// `/api/v1/config` under `notes.<key>`, the only config route a non-admin
// editor may read. A missing key falls back to the compiled default.

export interface NotesInstance {
  autosaveIntervalS: number
  spellCheck:        boolean
}

const DEFAULTS: NotesInstance = { autosaveIntervalS: 30, spellCheck: true }

export function useNotesInstance(): NotesInstance {
  const { data } = useQuery({
    queryKey: ['notes-instance-config'],
    queryFn: async () => {
      const res = await api.get<{ config: Record<string, unknown> }>('/config')
      return res.data.config ?? {}
    },
    staleTime: 5 * 60_000,
  })
  const cfg = data ?? {}
  const iv = cfg['notes.autosave_interval_s']
  const sc = cfg['notes.enable_spell_check']
  return {
    autosaveIntervalS: typeof iv === 'number' && Number.isFinite(iv) ? iv : DEFAULTS.autosaveIntervalS,
    spellCheck:        typeof sc === 'boolean' ? sc : DEFAULTS.spellCheck,
  }
}
