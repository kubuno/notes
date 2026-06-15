import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@kubuno/sdk'
import { StickyNote, Save, ChevronLeft, ExternalLink } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Toggle, Button, Tabs } from '@ui'

type Tab = 'editor' | 'reminders' | 'about'

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
              {['Rust', 'Axum 0.7', 'SQLx 0.8', 'PostgreSQL 16', 'pulldown-cmark', 'tokio'].map(t => (
                <span key={t} className="text-xs px-2 py-1 rounded-lg bg-surface-2 text-text-secondary font-mono">{t}</span>
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

export default function NotesSettingsPage() {
  const { t } = useTranslation('notes')
  const [tab, setTab] = useState<Tab>('editor')

  const TABS: { id: Tab; label: string }[] = [
    { id: 'editor',    label: t('notes_tab_editor') },
    { id: 'reminders', label: t('notes_tab_reminders') },
    { id: 'about',     label: t('notes_tab_about') },
  ]

  return (
    <div className="max-w-2xl">
      <div className="flex items-center gap-3 mb-6">
        <Link to="/admin?tab=modules" className="p-1.5 rounded-lg hover:bg-surface-2 text-text-secondary hover:text-text-primary transition-colors">
          <ChevronLeft size={18} />
        </Link>
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-yellow-100 flex items-center justify-center">
            <StickyNote size={16} className="text-yellow-600" />
          </div>
          <div>
            <h1 className="text-lg font-medium text-text-primary">{t('notes_settings_title')}</h1>
            <p className="text-xs text-text-tertiary">{t('notes_settings_subtitle')}</p>
          </div>
        </div>
      </div>

      <Tabs tabs={TABS} value={tab} onChange={setTab} className="mb-6" />

      {tab === 'editor'    && <EditorTab />}
      {tab === 'reminders' && <RemindersTab />}
      {tab === 'about'     && <AboutTab />}
    </div>
  )
}
