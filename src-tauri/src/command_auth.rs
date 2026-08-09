use serde::de::DeserializeOwned;
use tauri::{ipc::CommandArg, Runtime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IpcCaller {
    Main,
    FloatButton,
    Reminder,
    PiWebRepair,
}

pub(crate) fn require_caller_label(label: &str, allowed: &[IpcCaller]) -> Result<(), String> {
    let caller = match label {
        "main" => IpcCaller::Main,
        "floatbtn" => IpcCaller::FloatButton,
        "reminder" => IpcCaller::Reminder,
        "pi-web-repair" => IpcCaller::PiWebRepair,
        _ => return Err("FORBIDDEN_WINDOW".into()),
    };

    allowed
        .contains(&caller)
        .then_some(())
        .ok_or_else(|| "FORBIDDEN_WINDOW".into())
}

pub(crate) struct AuthorizedArgs<T, const MASK: u8>(pub T);

pub(crate) type MainArgs<T> = AuthorizedArgs<T, 0b001>;
pub(crate) type FloatArgs<T> = AuthorizedArgs<T, 0b010>;
pub(crate) type ReminderArgs<T> = AuthorizedArgs<T, 0b100>;
pub(crate) type MainOrFloatArgs<T> = AuthorizedArgs<T, 0b011>;
pub(crate) type MainOrPiWebRepairArgs<T> = AuthorizedArgs<T, 0b1001>;

fn allowed_callers<const MASK: u8>() -> Vec<IpcCaller> {
    let mut allowed = Vec::with_capacity(3);
    if MASK & 0b001 != 0 {
        allowed.push(IpcCaller::Main);
    }
    if MASK & 0b010 != 0 {
        allowed.push(IpcCaller::FloatButton);
    }
    if MASK & 0b100 != 0 {
        allowed.push(IpcCaller::Reminder);
    }
    if MASK & 0b1000 != 0 {
        allowed.push(IpcCaller::PiWebRepair);
    }
    allowed
}

fn deserialize_authorized_payload<T, const MASK: u8>(
    label: &str,
    payload: &tauri::ipc::InvokeBody,
) -> Result<T, String>
where
    T: DeserializeOwned,
{
    require_caller_label(label, &allowed_callers::<MASK>())?;
    let value = match payload {
        tauri::ipc::InvokeBody::Json(value) => value.clone(),
        tauri::ipc::InvokeBody::Raw(_) => return Err("INVALID_INPUT".into()),
    };

    serde_json::from_value(value).map_err(|_| "INVALID_INPUT".into())
}

impl<'de, T, const MASK: u8, R> CommandArg<'de, R> for AuthorizedArgs<T, MASK>
where
    T: DeserializeOwned,
    R: Runtime,
{
    fn from_command(
        command: tauri::ipc::CommandItem<'de, R>,
    ) -> Result<Self, tauri::ipc::InvokeError> {
        deserialize_authorized_payload::<T, MASK>(
            command.message.webview_ref().label(),
            command.message.payload(),
        )
        .map(Self)
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct ExampleCommandArgs {
        project_name: String,
    }

    #[test]
    fn main_args_reject_a_forbidden_window_before_deserializing_raw_input() {
        let error = deserialize_authorized_payload::<ExampleCommandArgs, 0b001>(
            "floatbtn",
            &tauri::ipc::InvokeBody::Raw(vec![0]),
        )
        .unwrap_err();

        assert_eq!(error, "FORBIDDEN_WINDOW");
    }

    #[test]
    fn main_args_return_invalid_input_for_raw_input_from_main() {
        let error = deserialize_authorized_payload::<ExampleCommandArgs, 0b001>(
            "main",
            &tauri::ipc::InvokeBody::Raw(vec![0]),
        )
        .unwrap_err();

        assert_eq!(error, "INVALID_INPUT");
    }

    #[test]
    fn main_args_use_camel_case_and_reject_unknown_fields() {
        let parsed = deserialize_authorized_payload::<ExampleCommandArgs, 0b001>(
            "main",
            &tauri::ipc::InvokeBody::Json(serde_json::json!({ "projectName": "L36" })),
        )
        .unwrap();
        assert_eq!(parsed.project_name, "L36");

        let error = deserialize_authorized_payload::<ExampleCommandArgs, 0b001>(
            "main",
            &tauri::ipc::InvokeBody::Json(serde_json::json!({ "project_name": "L36" })),
        )
        .unwrap_err();
        assert_eq!(error, "INVALID_INPUT");
    }

    #[test]
    fn main_or_pi_web_repair_args_allow_the_repair_window_label() {
        let parsed = deserialize_authorized_payload::<ExampleCommandArgs, 0b1001>(
            "pi-web-repair",
            &tauri::ipc::InvokeBody::Json(serde_json::json!({ "projectName": "PI-Web" })),
        )
        .unwrap();

        assert_eq!(parsed.project_name, "PI-Web");
    }
}
