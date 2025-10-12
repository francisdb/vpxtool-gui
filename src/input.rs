use crate::guifrontend::VpxTables;
use crate::list::SelectedItem;
use crate::loading::LoadingState;
use bevy::app::{App, Update};
use bevy::input::ButtonInput;
use bevy::log::debug;
use bevy::prelude::{
    IntoScheduleConfigs, KeyCode, Local, Message, MessageWriter, Res, ResMut, Time, in_state,
};
use bevy::time::Stopwatch;

#[derive(Message)]
pub struct TableSelectionChanged {
    pub _selected_index: usize,
}

#[derive(Default)]
struct ShiftIncrement {
    s: f32,
}

pub(crate) fn input_plugin(app: &mut App) {
    app.add_systems(Update, input_handling.run_if(in_state(LoadingState::Ready)));
}

/// Handles input for changing the selected table.
/// Uses Shift+Left/Right to change selection, with acceleration when held down.
fn input_handling(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut shift_stop_watch: Local<Stopwatch>,
    mut shift_applied: Local<ShiftIncrement>,
    mut selected_item_res: ResMut<SelectedItem>,
    mut event_writer: MessageWriter<TableSelectionChanged>,
    tables: Res<VpxTables>,
) {
    let mut selected_item = selected_item_res.index.unwrap_or(0) as i16;

    // Update timers
    shift_stop_watch.tick(time.delta());

    // Adjust increment based on time pressed
    let shift_increment = (shift_stop_watch.elapsed_secs() / 1.5).min(10.0);

    if keys.just_pressed(KeyCode::ShiftRight) {
        selected_item += 1;
        shift_applied.s = 0.0;
        shift_stop_watch.reset();
    } else if keys.just_pressed(KeyCode::ShiftLeft) {
        selected_item -= 1;
        shift_applied.s = 0.0;
        shift_stop_watch.reset();
    } else if keys.pressed(KeyCode::ShiftRight) {
        shift_applied.s += shift_increment;
        if shift_applied.s >= 1.0 {
            selected_item += shift_applied.s.floor() as i16;
            shift_applied.s = shift_applied.s.fract();
        }
    } else if keys.pressed(KeyCode::ShiftLeft) {
        shift_applied.s += shift_increment;
        if shift_applied.s >= 1.0 {
            selected_item -= shift_applied.s.floor() as i16;
            shift_applied.s = shift_applied.s.fract();
        }
    }

    let table_count = tables.indexed_tables.len();

    // Wrap around if one of the bounds are hit.
    let selected_item = wrap_around(selected_item, table_count);
    if selected_item_res.index != Some(selected_item) {
        event_writer.write(TableSelectionChanged {
            _selected_index: selected_item,
        });
        debug!("Selected item: {} ({} total)", selected_item, table_count);
    }
    selected_item_res.index = Some(selected_item);
}

/// Wraps a number around a maximum value.
pub(crate) fn wrap_around(n: i16, max: usize) -> usize {
    if n == 0 || max == 0 {
        0
    } else if n >= max as i16 {
        n as usize % max
    } else if n < 0 {
        ((n % max as i16 + max as i16) % max as i16) as usize
    } else {
        n as usize
    }
}
