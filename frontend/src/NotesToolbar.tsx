import { useTranslation } from 'react-i18next'
import { BookOpen, Pin, Archive, Trash2 } from 'lucide-react'
import { useNotesStore } from './store'

const VIEW_LABEL_KEYS = {
  all:      'notes_toolbar_all',
  pinned:   'notes_nav_pinned',
  archived: 'notes_nav_archived',
  trashed:  'notes_nav_trash',
}

const VIEW_ICONS = {
  all:      BookOpen,
  pinned:   Pin,
  archived: Archive,
  trashed:  Trash2,
}

export default function NotesToolbar() {
  const { t } = useTranslation('notes')
  const { view, activeNotebook, activeLabel, notebooks, labels } = useNotesStore()

  const Icon  = VIEW_ICONS[view]
  const label = activeNotebook
    ? (notebooks.find(n => n.id === activeNotebook)?.name ?? t('notes_filter_notebook'))
    : activeLabel
    ? (labels.find(l => l.id === activeLabel)?.name ?? t('notes_filter_label'))
    : t(VIEW_LABEL_KEYS[view])

  return (
    <div className="flex items-center h-14 px-4 gap-2">
      <Icon size={18} className="text-text-tertiary flex-shrink-0" />
      <h1 className="text-[22px] font-normal text-text-primary tracking-tight">{label}</h1>
    </div>
  )
}
