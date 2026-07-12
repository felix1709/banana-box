use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex as AsyncMutex, Notify};

#[derive(Default)]
struct PanelState {
    desired_visible: bool,
    actual_visible: bool,
    generation: u64,
    reveal_ack_generation: Option<u64>,
}

#[derive(Default)]
pub struct PanelStateMachine {
    inner: Mutex<PanelState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelTransition {
    pub generation: u64,
    pub target_visible: bool,
}

#[derive(Clone, Copy, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PanelTransitionReason {
    Banana,
    Tray,
    Shortcut,
    FileDrop,
    FocusLoss,
    TitlebarClose,
    ReminderAction,
    SecondInstance,
    Startup,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PanelTargetChanged {
    pub generation: u64,
    pub target_visible: bool,
    pub reason: PanelTransitionReason,
    pub reveal_at_frame: u8,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PanelVisibilityChanged {
    pub generation: u64,
    pub visible: bool,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PanelStateSnapshot {
    pub generation: u64,
    pub desired_visible: bool,
    pub actual_visible: bool,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PanelRevealAck {
    pub generation: u64,
    pub frame: u8,
}

#[derive(Default)]
pub struct WindowStateService {
    panel: PanelStateMachine,
    reveal_notify: Notify,
    native_transition: AsyncMutex<()>,
}

impl PanelStateMachine {
    pub fn request(&self, target_visible: bool) -> PanelTransition {
        let mut state = self.inner.lock().expect("panel state poisoned");
        state.generation += 1;
        state.desired_visible = target_visible;
        state.reveal_ack_generation = None;

        PanelTransition {
            generation: state.generation,
            target_visible,
        }
    }

    pub fn toggle(&self) -> PanelTransition {
        let mut state = self.inner.lock().expect("panel state poisoned");
        state.generation += 1;
        state.desired_visible = !state.desired_visible;
        state.reveal_ack_generation = None;

        PanelTransition {
            generation: state.generation,
            target_visible: state.desired_visible,
        }
    }

    pub fn complete(&self, generation: u64, visible: bool) -> Option<PanelVisibilityChanged> {
        let mut state = self.inner.lock().expect("panel state poisoned");
        if state.generation != generation || state.desired_visible != visible {
            return None;
        }

        state.actual_visible = visible;
        Some(PanelVisibilityChanged {
            generation,
            visible,
        })
    }

    pub fn acknowledge_reveal(&self, generation: u64, frame: u8) -> bool {
        let mut state = self.inner.lock().expect("panel state poisoned");
        if frame < 6 || !state.desired_visible || state.generation != generation {
            return false;
        }

        state.reveal_ack_generation = Some(generation);
        true
    }

    pub fn reveal_acknowledged(&self, generation: u64) -> bool {
        self.inner
            .lock()
            .expect("panel state poisoned")
            .reveal_ack_generation
            == Some(generation)
    }

    pub fn snapshot(&self) -> PanelStateSnapshot {
        let state = self.inner.lock().expect("panel state poisoned");
        PanelStateSnapshot {
            generation: state.generation,
            desired_visible: state.desired_visible,
            actual_visible: state.actual_visible,
        }
    }

    fn restore_actual_visibility(&self) -> PanelTransition {
        let mut state = self.inner.lock().expect("panel state poisoned");
        state.generation += 1;
        state.desired_visible = state.actual_visible;
        state.reveal_ack_generation = None;

        PanelTransition {
            generation: state.generation,
            target_visible: state.desired_visible,
        }
    }
}

impl WindowStateService {
    pub fn request_visibility(
        self: &Arc<Self>,
        app: &tauri::AppHandle,
        target_visible: bool,
        reason: PanelTransitionReason,
    ) -> Result<PanelTargetChanged, String> {
        let transition = self.panel.request(target_visible);
        self.request_transition(app, transition, reason)
    }

    pub fn toggle(
        self: &Arc<Self>,
        app: &tauri::AppHandle,
        reason: PanelTransitionReason,
    ) -> Result<PanelTargetChanged, String> {
        self.request_transition(app, self.panel.toggle(), reason)
    }

    fn request_transition(
        self: &Arc<Self>,
        app: &tauri::AppHandle,
        transition: PanelTransition,
        reason: PanelTransitionReason,
    ) -> Result<PanelTargetChanged, String> {
        let payload = PanelTargetChanged {
            generation: transition.generation,
            target_visible: transition.target_visible,
            reason,
            reveal_at_frame: 6,
        };
        if let Err(error) = app.emit_to("floatbtn", "panel-target-changed", &payload) {
            self.panel.restore_actual_visibility();
            return Err(error.to_string());
        }

        let service = Arc::clone(self);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = service.finish_transition(&app, transition).await;
        });

        Ok(payload)
    }

    pub fn acknowledge_reveal(&self, acknowledgement: &PanelRevealAck) -> bool {
        let accepted = self
            .panel
            .acknowledge_reveal(acknowledgement.generation, acknowledgement.frame);
        if accepted {
            self.reveal_notify.notify_waiters();
        }
        accepted
    }

    pub fn snapshot(&self) -> PanelStateSnapshot {
        self.panel.snapshot()
    }

    async fn finish_transition(
        self: Arc<Self>,
        app: &tauri::AppHandle,
        transition: PanelTransition,
    ) -> Result<(), String> {
        if transition.target_visible && !self.panel.reveal_acknowledged(transition.generation) {
            let notified = self.reveal_notify.notified();
            if !self.panel.reveal_acknowledged(transition.generation) {
                let _ = tokio::time::timeout(Duration::from_millis(400), notified).await;
            }
        }

        self.commit_native_visibility(app, transition).await
    }

    async fn commit_native_visibility(
        &self,
        app: &tauri::AppHandle,
        transition: PanelTransition,
    ) -> Result<(), String> {
        let _native_transition = self.native_transition.lock().await;
        let snapshot = self.panel.snapshot();
        if snapshot.generation != transition.generation
            || snapshot.desired_visible != transition.target_visible
        {
            return Ok(());
        }

        let main = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        if transition.target_visible {
            main.show().map_err(|error| error.to_string())?;
        } else {
            main.hide().map_err(|error| error.to_string())?;
        }

        let Some(committed) = self
            .panel
            .complete(transition.generation, transition.target_visible)
        else {
            return Ok(());
        };
        if transition.target_visible {
            let _ = main.set_focus();
        }
        app.emit_to("floatbtn", "panel-visibility-changed", &committed)
            .map_err(|error| error.to_string())?;
        app.emit_to("main", "panel-visibility-changed", &committed)
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_toggle_fences_the_pending_open() {
        let state = PanelStateMachine::default();
        let open = state.request(true);
        let close = state.request(false);

        assert!(state.complete(open.generation, true).is_none());
        assert_eq!(
            state.complete(close.generation, false).unwrap().visible,
            false
        );
    }

    #[test]
    fn toggle_uses_desired_not_delayed_actual_visibility() {
        let state = PanelStateMachine::default();
        state.request(true);

        assert_eq!(state.toggle().target_visible, false);
    }

    #[test]
    fn serializes_the_frontend_panel_contract_in_camel_case() {
        let payload = PanelTargetChanged {
            generation: 3,
            target_visible: true,
            reason: PanelTransitionReason::TitlebarClose,
            reveal_at_frame: 6,
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "generation": 3,
                "targetVisible": true,
                "reason": "titlebarClose",
                "revealAtFrame": 6,
            }),
        );
    }
}
