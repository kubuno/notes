import type { MenuItem } from '@ui'
import { i18n, navigate } from '@kubuno/sdk'
import { FileText, CheckSquare } from 'lucide-react'
import { useNotesStore } from './store'

/**
 * Items for the sidebar "New" button (`shell.new-actions` extension point).
 * Built when the menu opens — fresh labels and store state, no hooks.
 */
export function newActionItems(): MenuItem[] {
  if (!window.location.pathname.startsWith('/notes')) return []

  const handleNew = async (type: 'text' | 'checklist') => {
    const note = await useNotesStore.getState().createNote({ note_type: type })
    navigate(`/notes/${note.id}`)
  }

  return [
    {
      type: 'action',
      label: i18n.t('notes:notes_new_note'),
      icon: <FileText size={16} />,
      onClick: () => { void handleNew('text') },
    },
    {
      type: 'action',
      label: i18n.t('notes:notes_new_list'),
      icon: <CheckSquare size={16} />,
      onClick: () => { void handleNew('checklist') },
    },
  ]
}
