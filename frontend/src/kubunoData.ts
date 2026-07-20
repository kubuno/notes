/**
 * Cross-module data sharing (JSON envelopes over the clipboard) — thin typed
 * facade over the platform service published by the core at runtime
 * (`ModuleServiceRegistry('core', …)`). No vendored logic: once
 * `@kubuno/sdk >= 0.1.3` is published on npm, these can become direct SDK
 * imports; until then only the TYPES are declared locally.
 */
import { ModuleServiceRegistry } from '@kubuno/sdk'
import type React from 'react'

export interface KubunoDataEnvelope {
  kubuno: 1
  type: string
  module: string
  title?: string
  text?: string
  href?: string
  data: unknown
}

export interface DataCardProps { envelope: KubunoDataEnvelope }

export interface DataCardStaticRender {
  svg?: string
  dataUrl?: string
  width: number
  height: number
}

export interface DataCardRenderer {
  types: string[]
  Component?: React.ComponentType<DataCardProps>
  renderStatic?: (envelope: KubunoDataEnvelope) => Promise<DataCardStaticRender | null>
}

/** Writes an envelope to the system clipboard (dual text/plain + text/html). */
export function copyKubunoData(envelope: KubunoDataEnvelope): Promise<boolean> {
  return ModuleServiceRegistry.call<Promise<boolean>>('core', 'copyKubunoData', envelope) ?? Promise.resolve(false)
}

/** Extracts an envelope from a paste/drop DataTransfer, if any. */
export function readKubunoData(dt: DataTransfer | null): KubunoDataEnvelope | null {
  return ModuleServiceRegistry.call<KubunoDataEnvelope | null>('core', 'readKubunoData', dt) ?? null
}

/** Registers this module's card renderer on the `core.data-card` extension point. */
export function registerDataCardRenderer(moduleId: string, renderer: DataCardRenderer): void {
  ModuleServiceRegistry.call('core', 'registerDataCardRenderer', moduleId, renderer)
}

/** Markdown block for a pasted envelope (notes are plain markdown). */
export function kubunoDataToMarkdown(env: KubunoDataEnvelope): string {
  const lines: string[] = []
  if (env.title) lines.push(`**${env.title}**`)
  if (env.text) {
    for (const l of env.text.split('\n')) {
      // Skip a first text line that just repeats the title.
      if (lines.length === 1 && l.trim() === env.title) continue
      lines.push(l)
    }
  }
  if (env.href) lines.push(`[${env.module}](${env.href})`)
  if (!lines.length) lines.push(env.type)
  return lines.map(l => `> ${l}`).join('\n') + '\n'
}
