export type PanelTransitionReason =
  | 'banana'
  | 'tray'
  | 'shortcut'
  | 'fileDrop'
  | 'focusLoss'
  | 'titlebarClose'
  | 'reminderAction'
  | 'secondInstance'
  | 'startup'

export interface PanelTargetChanged {
  generation: number
  targetVisible: boolean
  reason: PanelTransitionReason
  revealAtFrame: 6
}

export interface PanelVisibilityChanged {
  generation: number
  visible: boolean
}

export interface PanelRevealAck {
  generation: number
  frame: number
}

export interface PanelStateSnapshot {
  generation: number
  desiredVisible: boolean
  actualVisible: boolean
}

export type ReminderKind = 'dailyTasks'
export type ReminderPhase = 'initial' | 'snooze'
export type ReminderAction = 'settle' | 'snooze' | 'dismiss'
export type ReminderSide = 'left' | 'right'

export interface ReminderClaimRef {
  kind: ReminderKind
  localDate: string
  phase: ReminderPhase
  deliveryId: string
  attemptToken: string
  ownerId: string
  fence: number
}

export interface ReminderPlacement {
  side: ReminderSide
  tailOffsetPx: number
}

export interface ReminderPreparePayload {
  claim: ReminderClaimRef
  title: string
  body: string
  timestamp: string
  actions: ReminderAction[]
  severity: 'info' | 'warning'
}

export interface ReminderShownPayload {
  claim: ReminderClaimRef
}

export interface ReminderAttentionPayload {
  claim: ReminderClaimRef
  durationMs: 220
}

export interface ReminderUnreadChanged {
  unread: boolean
  revision: number
}

export type ReminderUnreadState = ReminderUnreadChanged

export interface ActivateFloatButtonResult {
  action: 'panelToggleRequested' | 'unreadReminderReopened' | 'reminderPriorityInFlight'
}

export interface ReminderMutationResult {
  accepted: true
  replayed: boolean
  uiSyncWarning: boolean
}
