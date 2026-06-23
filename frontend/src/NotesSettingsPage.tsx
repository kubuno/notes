import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api, useAuthStore } from '@kubuno/sdk'
import { StickyNote, Save, ArrowLeft, ExternalLink, Check } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Toggle, Button, Radio } from '@ui'
import { useModulePrefs } from './userPrefs'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

interface NotesPrefs {
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

// ── Admin-only global settings (instance, via /admin/settings) ──────────────────

interface NotesSettings {
  'notes.default_editor': string
  'notes.autosave_interval_s': number
  'notes.enable_spell_check': boolean
  'notes.enable_bidirectional_links': boolean
  'notes.default_reminder_before_min': number
}

function useAdminSettings() {
  return useQuery({
    queryKey: ['admin-settings'],
    queryFn: () =>
      api.get<{ settings: { key: string; value: unknown }[] }>('/admin/settings').then((r) => {
        const map: Record<string, unknown> = {}
        r.data.settings.forEach((s) => { map[s.key] = s.value })
        return map as unknown as NotesSettings
      }),
  })
}

function EditorTab() {
  const { t } = useTranslation('notes')
  const queryClient = useQueryClient()
  const { data: settings } = useAdminSettings()

  const EDITOR_OPTIONS = [
    { value: 'wysiwyg',  label: t('notes_editor_wysiwyg'),  desc: t('notes_editor_wysiwyg_desc') },
    { value: 'markdown', label: t('notes_editor_markdown'), desc: t('notes_editor_markdown_desc') },
  ]

  const AUTOSAVE_OPTIONS = [
    { value: 5,   label: t('notes_autosave_5s') },
    { value: 30,  label: t('notes_autosave_30s') },
    { value: 60,  label: t('notes_autosave_1m') },
    { value: 0,   label: t('notes_autosave_off') },
  ]

  const [editorMode, setEditorMode]   = useState<string | null>(null)
  const [autosave, setAutosave]       = useState<number | null>(null)
  const [spellCheck, setSpellCheck]   = useState<boolean | null>(null)
  const [biLinks, setBiLinks]         = useState<boolean | null>(null)

  const currentEditor     = editorMode  ?? (settings?.['notes.default_editor']             ?? 'wysiwyg')
  const currentAutosave   = autosave    ?? (settings?.['notes.autosave_interval_s']         ?? 30)
  const currentSpellCheck = spellCheck  ?? (settings?.['notes.enable_spell_check']          ?? true)
  const currentBiLinks    = biLinks     ?? (settings?.['notes.enable_bidirectional_links']  ?? true)

  const isDirty = editorMode !== null || autosave !== null || spellCheck !== null || biLinks !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-settings'] })
      setEditorMode(null)
      setAutosave(null)
      setSpellCheck(null)
      setBiLinks(null)
    },
  })

  function handleSave() {
    const updates: Record<string, unknown> = {}
    if (editorMode  !== null) updates['notes.default_editor']            = editorMode
    if (autosave    !== null) updates['notes.autosave_interval_s']       = autosave
    if (spellCheck  !== null) updates['notes.enable_spell_check']        = spellCheck
    if (biLinks     !== null) updates['notes.enable_bidirectional_links'] = biLinks
    if (Object.keys(updates).length > 0) save.mutate(updates)
  }

  return (
    <div>
      <div className="bg-white rounded-xl border border-border divide-y divide-border">
        {/* Editor mode */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">{t('notes_default_editor_mode')}</p>
          <p className="text-xs text-text-secondary mb-3">
            {t('notes_default_editor_mode_desc')}
          </p>
          <div className="flex gap-3">
            {EDITOR_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setEditorMode(opt.value)}
                className={`flex-1 max-w-[180px] py-3 rounded-xl border text-center transition-colors ${
                  currentEditor === opt.value
                    ? 'border-primary bg-primary-light text-primary'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                }`}
              >
                <p className="text-sm font-semibold">{opt.label}</p>
                <p className="text-xs mt-0.5 opacity-70">{opt.desc}</p>
              </button>
            ))}
          </div>
        </div>

        {/* Autosave */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">{t('notes_autosave_interval')}</p>
          <p className="text-xs text-text-secondary mb-3">
            {t('notes_autosave_interval_desc')}
          </p>
          <div className="flex flex-wrap gap-2">
            {AUTOSAVE_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setAutosave(opt.value)}
                className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                  currentAutosave === opt.value
                    ? 'border-primary bg-primary-light text-primary font-medium'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>

        {/* Spell check toggle */}
        <div className="p-5 flex items-start justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-text-primary">{t('notes_spell_check')}</p>
            <p className="text-xs text-text-secondary mt-0.5">
              {t('notes_spell_check_desc')}
            </p>
          </div>
          <Toggle checked={currentSpellCheck} onChange={() => setSpellCheck(!currentSpellCheck)} />
        </div>

        {/* Bidirectional links toggle */}
        <div className="p-5 flex items-start justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-text-primary">{t('notes_bidi_links')}</p>
            <p className="text-xs text-text-secondary mt-0.5">
              {t('notes_bidi_links_desc')} <code className="font-mono bg-surface-2 px-1 rounded text-xs">[[{t('notes_bidi_links_syntax')}]]</code>.
              {' '}{t('notes_bidi_links_desc2')}
            </p>
          </div>
          <Toggle checked={currentBiLinks} onChange={() => setBiLinks(!currentBiLinks)} />
        </div>
      </div>

      <div className="mt-4 flex justify-end">
        <Button onClick={handleSave} disabled={!isDirty || save.isPending}>
          <Save size={15} />
          {save.isPending ? t('notes_saving') : t('common_save')}
        </Button>
      </div>
    </div>
  )
}

function RemindersTab() {
  const { t } = useTranslation('notes')
  const queryClient = useQueryClient()
  const { data: settings } = useAdminSettings()

  const REMINDER_OPTIONS = [
    { value: 0,    label: t('notes_reminder_ontime') },
    { value: 15,   label: t('notes_reminder_15m') },
    { value: 60,   label: t('notes_reminder_1h') },
    { value: 1440, label: t('notes_reminder_1d') },
  ]

  const [reminderMin, setReminderMin] = useState<number | null>(null)

  const currentReminder = reminderMin ?? (settings?.['notes.default_reminder_before_min'] ?? 60)
  const isDirty = reminderMin !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-settings'] })
      setReminderMin(null)
    },
  })

  return (
    <div>
      <div className="bg-white rounded-xl border border-border divide-y divide-border">
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">
            {t('notes_default_reminder')}
          </p>
          <p className="text-xs text-text-secondary mb-3">
            {t('notes_default_reminder_desc')}
          </p>
          <div className="flex flex-wrap gap-2">
            {REMINDER_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setReminderMin(opt.value)}
                className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                  currentReminder === opt.value
                    ? 'border-primary bg-primary-light text-primary font-medium'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-4 flex justify-end">
        <Button
          icon={<Save size={15} />}
          onClick={() => { if (reminderMin !== null) save.mutate({ 'notes.default_reminder_before_min': reminderMin }) }}
          disabled={!isDirty}
          loading={save.isPending}
        >
          {save.isPending ? t('notes_saving') : t('common_save')}
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

type Tab = 'preferences' | 'editor' | 'reminders' | 'about'

export default function NotesSettingsPage() {
  const { t } = useTranslation('notes')
  const isAdmin = useAuthStore(s => s.user?.role === 'admin')
  const [tab, setTab] = useState<Tab>('preferences')

  // Admin-only tabs (instance-wide settings) are hidden for non-admins.
  const tabs: { id: Tab; label: string; adminOnly?: boolean }[] = [
    { id: 'preferences', label: t('notes_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'editor',      label: t('notes_tab_editor'),    adminOnly: true },
    { id: 'reminders',   label: t('notes_tab_reminders'), adminOnly: true },
    { id: 'about',       label: t('notes_tab_about') },
  ]
  const visibleTabs = tabs.filter(tb => !tb.adminOnly || isAdmin)

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
          {tab === 'preferences'           && <PreferencesTab />}
          {tab === 'editor'    && isAdmin   && <EditorTab />}
          {tab === 'reminders' && isAdmin   && <RemindersTab />}
          {tab === 'about'                  && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
