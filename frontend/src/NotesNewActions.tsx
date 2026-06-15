import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { useTranslation } from 'react-i18next'
import { FileText, CheckSquare } from 'lucide-react'
import { useNavigate, useLocation } from 'react-router-dom'
import { useNotesStore } from './store'

const ITEM_CLASS =
  'flex items-center gap-3 w-full px-3 py-2 text-sm text-text-primary ' +
  'hover:bg-surface-1 cursor-pointer outline-none'

export default function NotesNewActions() {
  const navigate    = useNavigate()
  const { t }       = useTranslation('notes')
  const { pathname } = useLocation()
  const createNote  = useNotesStore((s) => s.createNote)

  if (!pathname.startsWith('/notes')) return null

  const handleNew = async (type: 'text' | 'checklist') => {
    const note = await createNote({ note_type: type })
    navigate(`/notes/${note.id}`)
  }

  return (
    <>
      <DropdownMenu.Item onSelect={() => handleNew('text')} className={ITEM_CLASS}>
        <FileText size={16} className="text-text-secondary" />
        {t('notes_new_note')}
      </DropdownMenu.Item>
      <DropdownMenu.Item onSelect={() => handleNew('checklist')} className={ITEM_CLASS}>
        <CheckSquare size={16} className="text-text-secondary" />
        {t('notes_new_list')}
      </DropdownMenu.Item>
    </>
  )
}
