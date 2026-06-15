import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { StickyNote } from 'lucide-react'
import { formatDistanceToNow, parseISO } from 'date-fns'
import { getDateLocale } from '@kubuno/sdk'
import { notesApi } from './api'
import { DashboardWidget } from '@kubuno/sdk'

export default function NotesRecentWidget() {
  const { t, i18n } = useTranslation('notes')
  const { data, isLoading } = useQuery({
    queryKey: ['widget-notes-recent'],
    queryFn:  () => notesApi.listNotes(),
    staleTime: 60_000,
  })

  const notes = data?.notes ?? []

  return (
    <DashboardWidget
      title={t('notes_widget_recent')}
      icon={<StickyNote size={15} className="text-yellow-500" />}
      link="/notes"
    >
      {isLoading ? (
        <div className="px-4 py-6 text-center text-sm text-text-tertiary">{t('common_loading')}</div>
      ) : notes.length === 0 ? (
        <div className="px-4 py-6 text-center text-sm text-text-tertiary italic">
          {t('notes_widget_empty')}
        </div>
      ) : (
        <ul className="divide-y divide-border">
          {notes.map(note => (
            <li key={note.id} className="px-4 py-3 hover:bg-surface-1 transition-colors">
              <p className="text-sm font-medium text-text-primary truncate">
                {note.title || <span className="italic text-text-tertiary">{t('common_untitled')}</span>}
              </p>
              {note.content && (
                <p className="text-xs text-text-secondary truncate mt-0.5">
                  {note.content.replace(/<[^>]+>/g, '').slice(0, 80)}
                </p>
              )}
              <p className="text-xs text-text-tertiary mt-1">
                {formatDistanceToNow(parseISO(note.updated_at), { locale: getDateLocale(i18n.language), addSuffix: true })}
              </p>
            </li>
          ))}
        </ul>
      )}
    </DashboardWidget>
  )
}
