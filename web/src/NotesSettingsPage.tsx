import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { StickyNote, ArrowLeft, ExternalLink, Check } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Toggle, Button, Radio, useSaveShortcut } from '@ui'
import { useModulePrefs } from './userPrefs'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

// `type`, not `interface`: only a type alias gets the implicit index signature
// that `useModulePrefs<T extends Record<string, unknown>>` requires.
type NotesPrefs = {
  font:        string   // 'sans' | 'serif' | 'mono'
  fontSize:    string   // 'sm' | 'md' | 'lg'
  sort:        string   // 'updated' | 'created' | 'title'
  view:        string   // 'list' | 'grid'
  editorTheme: string   // 'light' | 'dark'
  autosave:    boolean
}

const DEFAULT_PREFS: NotesPrefs = {
  font: 'sans', fontSize: 'md', sort: 'updated',
  view: 'list', editorTheme: 'light', autosave: true,
}

// ── Mail-style layout helpers ───────────────────────────────────────────────────

function SettingsRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-start gap-8 py-4 border-b border-[#e8eaed] last:border-0">
      <div className="w-60 flex-shrink-0">
        <p className="text-sm text-[#202124] font-normal">{label}</p>
        {description && <p className="text-xs text-text-tertiary mt-0.5 leading-relaxed">{description}</p>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}

function RadioGroup({ options, value, onChange }: {
  options: { value: string; label: string }[]; value: string; onChange: (v: string) => void
}) {
  return (
    <div className="flex flex-col items-start gap-2">
      {options.map(opt => (
        <Radio key={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} label={opt.label} />
      ))}
    </div>
  )
}

// ── Préférences tab (per-user) ──────────────────────────────────────────────────

function PreferencesTab() {
  const { t } = useTranslation('notes')
  const { prefs: saved, update } = useModulePrefs<NotesPrefs>('notes', DEFAULT_PREFS)
  const [prefs, setPrefs] = useState<NotesPrefs>(saved)
  const [savedFlag, setSavedFlag] = useState(false)
  const [busy, setBusy] = useState(false)

  const set = <K extends keyof NotesPrefs>(key: K, value: NotesPrefs[K]) =>
    setPrefs(p => ({ ...p, [key]: value }))

  // Ctrl+S saves immediately (disabled while a save is in flight).
  useSaveShortcut(() => { void save() }, !busy)

  const save = async () => {
    setBusy(true)
    try {
      await update(prefs)
      setSavedFlag(true)
      setTimeout(() => setSavedFlag(false), 2500)
    } finally { setBusy(false) }
  }

  return (
    <div>
      <SettingsRow
        label={t('notes_pref_font', { defaultValue: 'Police d\'écriture' })}
        description={t('notes_pref_font_desc', { defaultValue: 'Police utilisée pour le corps de vos notes.' })}
      >
        <RadioGroup
          value={prefs.font}
          onChange={v => set('font', v)}
          options={[
            { value: 'sans',  label: t('notes_pref_font_sans',  { defaultValue: 'Sans empattement' }) },
            { value: 'serif', label: t('notes_pref_font_serif', { defaultValue: 'Avec empattement (serif)' }) },
            { value: 'mono',  label: t('notes_pref_font_mono',  { defaultValue: 'Largeur fixe (monospace)' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('notes_pref_font_size', { defaultValue: 'Taille de police' })}>
        <RadioGroup
          value={prefs.fontSize}
          onChange={v => set('fontSize', v)}
          options={[
            { value: 'sm', label: t('notes_pref_font_size_sm', { defaultValue: 'Petite' }) },
            { value: 'md', label: t('notes_pref_font_size_md', { defaultValue: 'Moyenne' }) },
            { value: 'lg', label: t('notes_pref_font_size_lg', { defaultValue: 'Grande' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('notes_pref_sort', { defaultValue: 'Tri par défaut' })}
        description={t('notes_pref_sort_desc', { defaultValue: 'Ordre d\'affichage de la liste des notes.' })}
      >
        <RadioGroup
          value={prefs.sort}
          onChange={v => set('sort', v)}
          options={[
            { value: 'updated', label: t('notes_pref_sort_updated', { defaultValue: 'Dernière modification' }) },
            { value: 'created', label: t('notes_pref_sort_created', { defaultValue: 'Date de création' }) },
            { value: 'title',   label: t('notes_pref_sort_title',   { defaultValue: 'Titre (A → Z)' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('notes_pref_view', { defaultValue: 'Mode d\'affichage' })}>
        <RadioGroup
          value={prefs.view}
          onChange={v => set('view', v)}
          options={[
            { value: 'list', label: t('notes_pref_view_list', { defaultValue: 'Liste' }) },
            { value: 'grid', label: t('notes_pref_view_grid', { defaultValue: 'Grille' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('notes_pref_editor_theme', { defaultValue: 'Thème de l\'éditeur' })}
        description={t('notes_pref_editor_theme_desc', { defaultValue: 'Apparence claire ou sombre de la zone d\'édition.' })}
      >
        <RadioGroup
          value={prefs.editorTheme}
          onChange={v => set('editorTheme', v)}
          options={[
            { value: 'light', label: t('notes_pref_editor_theme_light', { defaultValue: 'Clair' }) },
            { value: 'dark',  label: t('notes_pref_editor_theme_dark',  { defaultValue: 'Sombre' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('notes_pref_autosave', { defaultValue: 'Sauvegarde automatique' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.autosave} onChange={() => set('autosave', !prefs.autosave)} />
          <span className="text-sm text-text-primary">{t('notes_pref_autosave_on', { defaultValue: 'Enregistrer mes notes au fil de la frappe' })}</span>
        </label>
      </SettingsRow>

      <div className="pt-5 flex items-center gap-3">
        <Button onClick={save} loading={busy}>
          {savedFlag
            ? <><Check size={14} className="mr-1.5 inline" />{t('notes_settings_saved', { defaultValue: 'Enregistré' })}</>
            : t('notes_settings_save_changes', { defaultValue: 'Enregistrer les modifications' })}
        </Button>
        <Button variant="ghost" onClick={() => setPrefs(saved)}>
          {t('common_cancel', { defaultValue: 'Annuler' })}
        </Button>
      </div>
    </div>
  )
}

function AboutTab() {
  const { t } = useTranslation('notes')
  return (
    <div className="space-y-4">
      <div className="bg-white rounded-xl border border-border overflow-hidden">
        <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
          <div className="w-10 h-10 rounded-xl bg-yellow-100 flex items-center justify-center shrink-0">
            <StickyNote size={20} className="text-yellow-600" />
          </div>
          <div>
            <p className="text-sm font-semibold text-text-primary">Kubuno Notes</p>
            <p className="text-xs text-text-tertiary">v0.1.0 · {t('notes_official_module')}</p>
          </div>
          <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">
            Rust
          </span>
        </div>

        <div className="divide-y divide-border">
          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('notes_about_description')}</p>
            <p className="text-sm text-text-secondary leading-relaxed">
              {t('notes_about_description_text')}
            </p>
          </div>

          <div className="px-5 py-4 grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('notes_about_author')}</p>
              <p className="text-sm text-text-primary">Kubuno Contributors</p>
            </div>
            <div>
              <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('notes_about_license')}</p>
              <p className="text-sm text-text-primary">AGPL-3.0</p>
            </div>
          </div>

          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-3">{t('notes_about_technologies')}</p>
            <div className="flex flex-wrap gap-2">
              {['Rust', 'Axum 0.7', 'SQLx 0.8', 'PostgreSQL 16', 'pulldown-cmark', 'tokio'].map(tech => (
                <span key={tech} className="text-xs px-2 py-1 rounded-lg bg-surface-2 text-text-secondary font-mono">{tech}</span>
              ))}
            </div>
          </div>

          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('notes_about_links')}</p>
            <a
              href="https://github.com/kubuno/kubuno"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
            >
              <ExternalLink size={13} />
              github.com/kubuno/kubuno
            </a>
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Main page (mail-style breadcrumb + tab bar) ─────────────────────────────────

// Per-user only. Instance-wide settings (editor behaviour) moved to the core
// admin console (Modules ▸ Notes ▸ Éditeur); they are no longer edited here.
type Tab = 'preferences' | 'about'

export default function NotesSettingsPage() {
  const { t } = useTranslation('notes')
  const [tab, setTab] = useState<Tab>('preferences')

  const visibleTabs: { id: Tab; label: string }[] = [
    { id: 'preferences', label: t('notes_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'about',       label: t('notes_tab_about') },
  ]

  return (
    <div className="flex flex-col h-full bg-white overflow-hidden">
      {/* Breadcrumb header */}
      <div className="flex items-center gap-2 px-6 py-2.5 border-b border-[#e8eaed] flex-shrink-0" style={{ background: '#f8f9fa' }}>
        <Link to="/notes" className="flex items-center gap-1.5 text-sm text-[#1a73e8] hover:underline">
          <ArrowLeft size={14} />
          Notes
        </Link>
        <span className="text-text-tertiary text-sm">/</span>
        <div className="flex items-center gap-1.5">
          <StickyNote size={15} className="text-text-secondary" />
          <span className="text-sm text-text-primary">{t('notes_settings_title', { defaultValue: 'Réglages' })}</span>
        </div>
      </div>

      {/* Tab bar (Gmail-style) */}
      <div className="flex items-end border-b border-[#e8eaed] px-4 flex-shrink-0 overflow-x-auto" style={{ background: '#fff' }}>
        {visibleTabs.map(tb => (
          <button key={tb.id} onClick={() => setTab(tb.id)}
            className={`px-4 py-3 text-sm border-b-2 -mb-px transition-colors whitespace-nowrap ${
              tab === tb.id ? 'border-[#1a73e8] text-[#1a73e8] font-medium' : 'border-transparent text-[#5f6368] hover:text-[#202124] hover:bg-[#f1f3f4]'}`}>
            {tb.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-6">
          {tab === 'preferences' && <PreferencesTab />}
          {tab === 'about'       && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
