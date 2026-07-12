use crate::fs_atomic;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

pub const FLOAT_WINDOW_LOGICAL_SIZE: f64 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogicalPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MonitorWorkArea {
    pub id: String,
    pub bounds: PhysicalRect,
    pub scale_factor: f64,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedFloatPosition {
    pub logical_x: f64,
    pub logical_y: f64,
    pub monitor_id: String,
    pub scale_factor: f64,
    pub saved_work_area: PhysicalRect,
}

pub struct DesktopStateStore {
    path: PathBuf,
}

impl DesktopStateStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Option<SavedFloatPosition>, String> {
        let value = match fs::read_to_string(&self.path) {
            Ok(value) => value,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };

        serde_json::from_str(&value)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    pub fn save(&self, position: &SavedFloatPosition) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "desktop state path has no parent directory".to_string())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;

        let temporary = self.path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(position).map_err(|error| error.to_string())?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&payload)
            .map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);

        fs_atomic::replace_file(&temporary, &self.path)
    }
}

pub fn logical_to_physical(
    point: LogicalPoint,
    monitor: PhysicalRect,
    scale_factor: f64,
) -> PhysicalPoint {
    PhysicalPoint {
        x: monitor.x + (point.x * scale_factor).round() as i32,
        y: monitor.y + (point.y * scale_factor).round() as i32,
    }
}

pub fn physical_to_saved(point: PhysicalPoint, monitor: &MonitorWorkArea) -> SavedFloatPosition {
    SavedFloatPosition {
        logical_x: f64::from(point.x - monitor.bounds.x) / monitor.scale_factor,
        logical_y: f64::from(point.y - monitor.bounds.y) / monitor.scale_factor,
        monitor_id: monitor.id.clone(),
        scale_factor: monitor.scale_factor,
        saved_work_area: monitor.bounds,
    }
}

pub fn select_restore_monitor<'a>(
    saved: &SavedFloatPosition,
    monitors: &'a [MonitorWorkArea],
) -> &'a MonitorWorkArea {
    if let Some(monitor) = monitors
        .iter()
        .find(|monitor| monitor.id == saved.monitor_id)
    {
        return monitor;
    }

    let old_center = PhysicalPoint {
        x: saved.saved_work_area.x
            + ((saved.logical_x + FLOAT_WINDOW_LOGICAL_SIZE / 2.0) * saved.scale_factor).round()
                as i32,
        y: saved.saved_work_area.y
            + ((saved.logical_y + FLOAT_WINDOW_LOGICAL_SIZE / 2.0) * saved.scale_factor).round()
                as i32,
    };

    monitors
        .iter()
        .min_by_key(|monitor| squared_distance_to_rect(old_center, monitor.bounds))
        .or_else(|| monitors.iter().find(|monitor| monitor.primary))
        .expect("at least one monitor is required to restore the float button")
}

pub fn clamp_float_position(
    point: PhysicalPoint,
    bounds: PhysicalRect,
    window_size: i32,
    margin: i32,
) -> PhysicalPoint {
    let min_x = bounds.x + margin;
    let min_y = bounds.y + margin;
    let max_x = bounds.x + bounds.width as i32 - window_size - margin;
    let max_y = bounds.y + bounds.height as i32 - window_size - margin;

    PhysicalPoint {
        x: point.x.clamp(min_x, max_x.max(min_x)),
        y: point.y.clamp(min_y, max_y.max(min_y)),
    }
}

fn squared_distance_to_rect(point: PhysicalPoint, bounds: PhysicalRect) -> i64 {
    let right = bounds.x + bounds.width as i32;
    let bottom = bounds.y + bounds.height as i32;
    let horizontal = if point.x < bounds.x {
        bounds.x - point.x
    } else if point.x > right {
        point.x - right
    } else {
        0
    };
    let vertical = if point.y < bounds.y {
        bounds.y - point.y
    } else if point.y > bottom {
        point.y - bottom
    } else {
        0
    };

    i64::from(horizontal).pow(2) + i64::from(vertical).pow(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn monitor(id: &str, x: i32, y: i32) -> MonitorWorkArea {
        MonitorWorkArea {
            id: id.into(),
            bounds: PhysicalRect {
                x,
                y,
                width: 1920,
                height: 1080,
            },
            scale_factor: 1.0,
            primary: id == "primary",
        }
    }

    fn saved_on_removed_monitor(bounds: PhysicalRect) -> SavedFloatPosition {
        SavedFloatPosition {
            logical_x: 160.0,
            logical_y: 240.0,
            monitor_id: "removed".into(),
            scale_factor: 1.0,
            saved_work_area: bounds,
        }
    }

    #[test]
    fn restores_logical_offset_at_the_monitors_current_scale() {
        let monitor = PhysicalRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };

        assert_eq!(
            logical_to_physical(LogicalPoint { x: 320.0, y: 240.0 }, monitor, 2.0),
            PhysicalPoint { x: 640, y: 480 },
        );
    }

    #[test]
    fn missing_saved_monitor_chooses_the_nearest_work_area() {
        let saved = saved_on_removed_monitor(PhysicalRect {
            x: 1920,
            y: 0,
            width: 1920,
            height: 1080,
        });
        let monitors = vec![monitor("left", -1920, 0), monitor("primary", 0, 0)];

        assert_eq!(select_restore_monitor(&saved, &monitors).id, "primary");
    }

    #[test]
    fn atomically_round_trips_the_saved_position() {
        let dir = tempdir().unwrap();
        let store = DesktopStateStore::new(dir.path().join("desktop-state.json"));
        let expected = SavedFloatPosition {
            logical_x: 320.0,
            logical_y: 240.0,
            monitor_id: "DISPLAY1".into(),
            scale_factor: 1.25,
            saved_work_area: PhysicalRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
        };

        store.save(&expected).unwrap();
        store.save(&expected).unwrap();

        assert_eq!(store.load().unwrap(), Some(expected));
        assert!(!dir.path().join("desktop-state.json.tmp").exists());
    }
}
