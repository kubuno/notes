import { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { BookOpen, Tag, Pin, Archive } from 'lucide-react'
import { Input } from '@ui'
import { useSearchStore } from '@kubuno/sdk'
import { useNotesStore } from './store'
import { NOTE_COLORS } from './api'

export default function NotesFilterPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('notes')
  const {
    notebooks, labels,
    activeNotebook, activeLabel,
    view,
    setActiveNotebook, setActiveLabel, setView, setSearchQuery,
  } = useNotesStore()

  // ── Two-way sync with the shell search bar (platform rule) ──────────────────
  // Notes' search has no text operators: the bar's query is plain free text
  // (backend `q`). The « Contains the words » field below mirrors it: opening
  // the panel pre-fills it with the current query, and editing it rewrites the
  // bar's text live (running the search like typing does). The view/notebook/
  // label entries are state filters with no query-text representation — they
  // deliberately stay panel-only (no invented operators).
  const query    = useSearchStore(s => s.query)
  const setQuery = useSearchStore(s => s.setQuery)
  const [words, setWords] = useState(query)
  // Remembers the last query WE pushed so its echo doesn't clobber the field.
  const lastBuilt = useRef<string | null>(null)
  useEffect(() => {
    if (query === lastBuilt.current) return
    setWords(query)
  }, [query])

  const setContainsWords = (v: string) => {
    setWords(v)
    lastBuilt.current = v
    setQuery(v)          // rewrite the bar's text live
    setSearchQuery(v)    // run the live search, like typing in the bar does
  }

  const Section = ({ title }: { title: string }) => (
    <p className="text-[10px] font-bold text-text-tertiary uppercase tracking-wider px-4 pt-3 pb-1.5">
      {title}
    </p>
  )

  const FilterBtn = ({
    active, onClick, children,
  }: {
    active: boolean; onClick: () => void; children: React.ReactNode
  }) => (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 w-full text-left text-sm px-4 py-1.5 transition-colors
        ${active
          ? 'bg-primary-light text-primary font-medium'
          : 'text-text-primary hover:bg-surface-1'}`}
    >
      {children}
    </button>
  )

  return (
    <div className="py-2" style={{ minWidth: 240 }}>
      {/* Contains the words — mirrors the shell search bar (two-way sync) */}
      <Section title={t('notes_filter_words')} />
      <div className="px-4 pb-1.5">
        <Input
          type="text"
          placeholder={t('notes_search_ph')}
          value={words}
          onChange={e => setContainsWords(e.target.value)}
        />
      </div>
      <div className="mx-4 my-2 h-px bg-border" />

      {/* Vue rapide */}
      <Section title={t('notes_filter_show')} />
      <FilterBtn active={view === 'all' && !activeNotebook && !activeLabel} onClick={() => { setView('all'); setActiveNotebook(null); setActiveLabel(null); onClose() }}>
        <BookOpen size={14} className="flex-shrink-0" />
        {t('notes_nav_all')}
      </FilterBtn>
      <FilterBtn active={view === 'pinned'} onClick={() => { setView('pinned'); onClose() }}>
        <Pin size={14} className="flex-shrink-0" />
        {t('notes_nav_pinned')}
      </FilterBtn>
      <FilterBtn active={view === 'archived'} onClick={() => { setView('archived'); onClose() }}>
        <Archive size={14} className="flex-shrink-0" />
        {t('notes_nav_archived')}
      </FilterBtn>

      {/* Carnets */}
      {notebooks.length > 0 && (
        <>
          <div className="mx-4 my-2 h-px bg-border" />
          <Section title={t('notes_filter_notebook')} />
          {notebooks.map(nb => (
            <FilterBtn
              key={nb.id}
              active={activeNotebook === nb.id}
              onClick={() => { setActiveNotebook(nb.id); onClose() }}
            >
              <BookOpen size={14} className="flex-shrink-0" />
              {nb.name}
            </FilterBtn>
          ))}
        </>
      )}

      {/* Étiquettes */}
      {labels.length > 0 && (
        <>
          <div className="mx-4 my-2 h-px bg-border" />
          <Section title={t('notes_filter_label')} />
          <div className="flex flex-wrap gap-1.5 px-4 pb-2">
            {labels.map(label => (
              <button
                key={label.id}
                onClick={() => { setActiveLabel(label.id); onClose() }}
                className={`text-xs px-3 py-1 rounded-md border transition-colors
                  ${activeLabel === label.id
                    ? 'bg-primary text-white border-primary'
                    : 'border-border text-text-secondary hover:border-primary hover:text-primary'}`}
              >
                <Tag size={10} className="inline mr-1" />
                {label.name}
              </button>
            ))}
          </div>
        </>
      )}

      {/* Couleurs */}
      <div className="mx-4 my-2 h-px bg-border" />
      <Section title={t('notes_filter_color')} />
      <div className="flex flex-wrap gap-1.5 px-4 pb-3">
        {NOTE_COLORS.filter(c => c.id !== 'default').map(c => (
          <button
            key={c.id}
            title={t(`notes_color_${c.id}`, { defaultValue: c.label })}
            className="w-6 h-6 rounded-full border-2 border-transparent hover:scale-110 transition-transform hover:border-primary"
            style={{ backgroundColor: c.hex, boxShadow: '0 0 0 1px rgba(0,0,0,0.15)' }}
            onClick={() => {
              // Filtre par couleur via la recherche — on laisse le store gérer
              onClose()
            }}
          />
        ))}
      </div>
    </div>
  )
}
