import { useTranslation } from 'react-i18next'
import { BookOpen, Tag, Pin, Archive } from 'lucide-react'
import { useNotesStore } from './store'
import { NOTE_COLORS } from './api'

export default function NotesFilterPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('notes')
  const {
    notebooks, labels,
    activeNotebook, activeLabel,
    view,
    setActiveNotebook, setActiveLabel, setView,
  } = useNotesStore()

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
                className={`text-xs px-3 py-1 rounded-full border transition-colors
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
