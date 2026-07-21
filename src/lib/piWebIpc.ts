import { invoke } from '@tauri-apps/api/core'

export type PiWebServiceState =
  | 'missingRuntime'
  | 'stopped'
  | 'checking'
  | 'starting'
  | 'running'
  | 'error'

export interface PiWebDiagnosticLink {
  label: string
  url: string
}

export interface PiWebStatus {
  state: PiWebServiceState
  url: string
  port: number
  message: string
  detail: string
  missingDependency: string
  installLinks: PiWebDiagnosticLink[]
  canStart: boolean
  canOpen: boolean
  canStop: boolean
}

export type PiWebChatHealthState = 'idle' | 'ok' | 'warning' | 'error'

export interface PiWebChatHealth {
  state: PiWebChatHealthState
  message: string
  detail: string
  provider: string
  modelId: string
}

export interface PiWebRepairResult {
  changed: boolean
  message: string
  detail: string
}

export async function getPiWebStatus(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('get_pi_web_status', {})
}

export async function startPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('start_pi_web', {})
}

export async function stopPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('stop_pi_web', {})
}

export async function openPiWeb(): Promise<PiWebStatus> {
  return await invoke<PiWebStatus>('open_pi_web', {})
}

export async function getPiWebChatHealth(): Promise<PiWebChatHealth> {
  return await invoke<PiWebChatHealth>('get_pi_web_chat_health', {})
}

export async function repairPiWebModelCompatibility(): Promise<PiWebRepairResult> {
  return await invoke<PiWebRepairResult>('repair_pi_web_model_compatibility', {})
}
