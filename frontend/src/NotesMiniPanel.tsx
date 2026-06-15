import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Plus, FileText, CheckSquare, Pin } from 'lucide-react'
import { Button } from '@ui'
import { formatDistanceToNow, parseISO } from 'date-fns'
import { getDateLocale } from '@kubuno/sdk'
import { useNotesStore } from './store'
import type { Note } from './api'

// ── Carte note compacte ───────────────────────────────────────────────────────

function NoteCard({ note, onClick }: { note: Note; onClick: () => void }) {
  const { i18n } = useTranslation('notes')
  const preview = note.title
    ? note.content?.replace(/<[^>]+>/g, '').slice(0, 60)
    : note.content?.replace(/<[^>]+>/g, '').slice(0, 80)

  const checklistDone  = note.checklist?.filter((i) => i.checked).length ?? 0
  const checklistTotal = note.checklist?.length ?? 0

  return (
    <button
      onClick={onClick}
      className="w-full text-left rounded-lg border border-border bg-white px-3 py-2.5
                 hover:shadow-md transition-shadow group"
    >
      <div className="flex items-start gap-2">
        <span className="mt-0.5 flex-shrink-0 text-text-tertiary">
          {note.note_type === 'checklist'
            ? <CheckSquare size={13} />
            : <FileText size={13} />}
        </span>

        <div className="flex-1 min-w-0">
          {note.is_pinned && (
            <Pin size={10} className="inline-block text-primary mr-1 -mt-0.5" />
          )}
          {note.title && (
            <span className="text-xs font-medium text-text-primary truncate block">
              {note.title}
            </span>
          )}

          {note.note_type === 'checklist' && checklistTotal > 0 ? (
            <div className="flex items-center gap-1.5 mt-0.5">
              <div className="flex-1 h-1 bg-surface-3 rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary rounded-full transition-all"
                  style={{ width: `${Math.round((checklistDone / checklistTotal) * 100)}%` }}
                />
              </div>
              <span className="text-[10px] text-text-tertiary flex-shrink-0">
                {checklistDone}/{checklistTotal}
              </span>
            </div>
          ) : preview ? (
            <p className="text-[11px] text-text-secondary line-clamp-2 leading-relaxed mt-0.5">
              {preview}
            </p>
          ) : null}

          <span className="text-[10px] text-text-tertiary mt-1 block">
            {formatDistanceToNow(parseISO(note.updated_at), { addSuffix: true, locale: getDateLocale(i18n.language) })}
          </span>
        </div>
      </div>
    </button>
  )
}

// ── Composant principal ───────────────────────────────────────────────────────

export default function NotesMiniPanel() {
  const navigate = useNavigate()
  const { t } = useTranslation('notes')
  const { notes, isLoading, fetchNotes, createNote } = useNotesStore()

  useEffect(() => {
    fetchNotes()
  }, [fetchNotes])

  const handleNewNote = async () => {
    const note = await createNote({ note_type: 'text' })
    navigate(`/notes/${note.id}`)
  }

  const handleNewChecklist = async () => {
    const note = await createNote({ note_type: 'checklist' })
    navigate(`/notes/${note.id}`)
  }

  const recentNotes = notes.slice(0, 20)

  return (
    <div className="flex flex-col h-full">
      {/* Actions rapides */}
      <div
        className="flex gap-1.5 px-3 py-2.5 flex-shrink-0"
        style={{ borderBottom: '1px solid #dadce0' }}
      >
        <Button size="sm" icon={<Plus size={13} />} onClick={handleNewNote} className="flex-1 text-xs">
          {t('notes_mini_note')}
        </Button>
        <Button size="sm" variant="secondary" icon={<CheckSquare size={13} />} onClick={handleNewChecklist} className="flex-1 text-xs">
          {t('notes_mini_list')}
        </Button>
      </div>

      {/* En-tête liste */}
      <div className="px-3 pt-2 pb-1 flex-shrink-0">
        <button
          onClick={() => navigate('/notes')}
          className="text-[10px] font-bold uppercase tracking-widest text-text-tertiary
                     hover:text-primary transition-colors"
        >
          {t('notes_mini_recent')}
        </button>
      </div>

      {/* Liste des notes */}
      <div className="flex-1 overflow-y-auto px-2 pb-3 space-y-2 pt-2 bg-surface-1">
        {isLoading && (
          <div className="space-y-2 px-2 pt-1">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="h-14 bg-surface-2 rounded-lg animate-pulse" />
            ))}
          </div>
        )}

        {!isLoading && recentNotes.length === 0 && (
          <div className="flex flex-col items-center justify-center py-8 px-4 text-center">
            <FileText size={28} className="text-text-tertiary/40 mb-2" />
            <p className="text-xs text-text-tertiary">{t('notes_mini_empty')}</p>
            <button
              onClick={handleNewNote}
              className="mt-3 text-xs text-primary hover:underline"
            >
              {t('notes_mini_create_first')}
            </button>
          </div>
        )}

        {!isLoading && recentNotes.map((note) => (
          <NoteCard
            key={note.id}
            note={note}
            onClick={() => navigate(`/notes/${note.id}`)}
          />
        ))}
      </div>
    </div>
  )
}
