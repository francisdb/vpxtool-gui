use crate::guifrontend::VpxTables;
use crate::loading::LoadingState;
use bevy::asset::AssetLoadFailedEvent;
use bevy::color::palettes::css::GHOST_WHITE;
use bevy::ecs::system::SystemId;
use bevy::image::Image;
use bevy::log::debug;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_asset::AssetServer;
use std::path::PathBuf;
use vpxtool::indexer::IndexedTable;

/// The currently selected table.
#[derive(Resource, Default)]
pub struct SelectedItem {
    pub(crate) index: Option<usize>,
}

/// Animated scroll position, in table-index units. `current` eases towards
/// `target`; both can run outside `[0, table_count)` and are normalised when
/// the animation settles (see [`update_scroller`]).
#[derive(Resource, Default)]
struct Scroll {
    current: f32,
    target: f32,
}

/// One recycled sprite in the carousel. `world_index` is its (unwrapped)
/// position on the infinite strip; the displayed table is `world_index` modulo
/// the table count.
#[derive(Component)]
struct ScrollerSlot {
    world_index: i64,
    /// The table currently shown, to avoid reloading its image every frame.
    table_index: Option<usize>,
}

/// The table name shown below the centered table image.
#[derive(Component)]
struct ScrollerName;

/// System run when entering the image loading state. Images are loaded lazily
/// by the scroller, so this only reports readiness (it must not block the
/// loading screen on images: a single failed/stalled image would hang it).
#[derive(Resource)]
pub(crate) struct LoadImagesSystem(pub(crate) SystemId);

/// Number of images shown on each side of the centered one.
const VISIBLE_SIDE: i32 = 3;
/// Extra off-screen sprites kept on each side so recycling happens out of view.
const BUFFER: i32 = 1;
/// Half the sprite pool: visible side plus buffer.
const HALF_POOL: i32 = VISIBLE_SIDE + BUFFER;
/// Total number of recycled sprites.
const POOL: i32 = HALF_POOL * 2 + 1;
/// How quickly the scroll eases towards the selection (higher is snappier).
const SCROLL_SPEED: f32 = 7.0;
/// Fallback image (in `assets/`) used when a table has no `media/table.jpg`.
const FALLBACK_IMAGE: &str = "generic_table.png";

pub(crate) fn mediascroller_plugin(app: &mut App) {
    app.insert_resource(SelectedItem::default());
    app.insert_resource(Scroll::default());
    let load_system = app.register_system(load_initial_images);
    app.insert_resource(LoadImagesSystem(load_system));
    app.add_systems(Startup, spawn_scroller);
    app.add_systems(Update, log_failed_images);
    app.add_systems(
        Update,
        update_scroller.run_if(in_state(LoadingState::Ready)),
    );
}

/// Log table images that fail to load, so missing/broken media is visible.
fn log_failed_images(mut events: MessageReader<AssetLoadFailedEvent<Image>>) {
    for event in events.read() {
        warn!("Failed to load table image {}: {}", event.path, event.error);
    }
}

/// Spawn the recycled sprite pool and the name label (all hidden until the
/// first layout in [`update_scroller`]).
fn spawn_scroller(mut commands: Commands) {
    for k in 0..POOL {
        commands.spawn((
            Sprite::default(),
            Transform::from_xyz(0., 0., 0.),
            Visibility::Hidden,
            ScrollerSlot {
                world_index: (k - HALF_POOL) as i64,
                table_index: None,
            },
        ));
    }
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(28.0),
            ..default()
        },
        TextColor::from(GHOST_WHITE),
        Transform::from_xyz(0., 0., 20.),
        Visibility::Hidden,
        ScrollerName,
    ));
}

fn media_path(info: &IndexedTable) -> Option<PathBuf> {
    info.path
        .parent()
        .map(|p| p.join("media").join("table.jpg"))
}

/// A human readable description of the image a table will use, without loading.
fn table_image_desc(info: &IndexedTable) -> String {
    match media_path(info) {
        Some(path) if path.exists() => path.display().to_string(),
        _ => format!("{FALLBACK_IMAGE} (fallback)"),
    }
}

/// Resolve the image handle for a table: `media/table.jpg` next to the .vpx
/// file, or the bundled generic desktop-table view.
fn resolve_table_image(asset_server: &AssetServer, info: &IndexedTable) -> Handle<Image> {
    match media_path(info) {
        Some(path) if path.exists() => asset_server.load_builder().override_unapproved().load(path),
        _ => asset_server.load(FALLBACK_IMAGE),
    }
}

/// Run on entering the image loading state. The scroller loads images lazily in
/// [`update_scroller`], so this does not queue any blocking loads; it only logs.
fn load_initial_images(vpx_tables: Res<VpxTables>) {
    info!(
        "{} tables indexed; scroller images load on demand",
        vpx_tables.indexed_tables.len()
    );
}

/// Shortest signed number of steps from `from` to `to` on a ring of `count`.
fn circular_signed(from: usize, to: usize, count: usize) -> i64 {
    let n = count as i64;
    let mut d = (to as i64 - from as i64) % n;
    if d > n / 2 {
        d -= n;
    } else if d < -n / 2 {
        d += n;
    }
    d
}

/// Ease the scroll towards the selected table each frame and lay out the
/// recycled sprite pool, so the images slide and scale during a change.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_scroller(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut scroll: ResMut<Scroll>,
    mut last_selected: Local<Option<usize>>,
    mut initialized: Local<bool>,
    mut slots: Query<
        (
            &mut ScrollerSlot,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        ),
        Without<ScrollerName>,
    >,
    mut name_query: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<ScrollerName>>,
    selected_item_res: Res<SelectedItem>,
    tables: Res<VpxTables>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let count = tables.indexed_tables.len();
    if count == 0 {
        return;
    }
    let Ok(window) = window_query.single() else {
        return;
    };
    let selected = selected_item_res.index.unwrap_or(0);

    // Retarget the animation when the selection changes, moving the shortest way
    // around the ring.
    let changed = *last_selected != Some(selected);
    if changed {
        let from = scroll.target.round().rem_euclid(count as f32) as usize;
        scroll.target += circular_signed(from, selected, count) as f32;
        *last_selected = Some(selected);
        let info = &tables.indexed_tables[selected];
        info!(
            "Selected table '{}' image: {}",
            display_table_line(info),
            table_image_desc(info)
        );
    }

    let animating = (scroll.target - scroll.current).abs() > 0.001;
    // Nothing to do once settled and the selection hasn't changed.
    if !animating && !changed && *initialized {
        return;
    }

    if animating {
        let t = 1.0 - (-SCROLL_SPEED * time.delta_secs()).exp();
        scroll.current += (scroll.target - scroll.current) * t;
        if (scroll.target - scroll.current).abs() < 0.01 {
            scroll.current = scroll.target;
        }
    }

    // Keep the float positions bounded once settled by shifting everything by a
    // whole number of rings (preserves both wrapped index and on-screen offset).
    let mut shift = 0i64;
    if scroll.current == scroll.target {
        let rings = (scroll.current / count as f32).floor();
        if rings != 0.0 {
            scroll.current -= rings * count as f32;
            scroll.target -= rings * count as f32;
            shift = rings as i64 * count as i64;
        }
    }

    let center_w = window.width() * 0.6;
    let center_h = center_w / 1.5;
    let spacing = window.width() * 0.62;
    let recycle_limit = HALF_POOL as f32 + 0.5;

    for (mut slot, mut sprite, mut transform, mut visibility) in slots.iter_mut() {
        if shift != 0 {
            slot.world_index -= shift;
        }
        let mut rel = slot.world_index as f32 - scroll.current;
        // Recycle sprites that drift past the buffer to the opposite side.
        while rel < -recycle_limit {
            slot.world_index += POOL as i64;
            rel += POOL as f32;
        }
        while rel > recycle_limit {
            slot.world_index -= POOL as i64;
            rel -= POOL as f32;
        }

        let table_index = slot.world_index.rem_euclid(count as i64) as usize;
        if slot.table_index != Some(table_index) {
            let info = &tables.indexed_tables[table_index];
            sprite.image = resolve_table_image(&asset_server, info);
            slot.table_index = Some(table_index);
            debug!("Scroller sprite -> '{}'", display_table_line(info));
        }

        let dist = rel.abs();
        if dist > VISIBLE_SIDE as f32 {
            *visibility = Visibility::Hidden;
            continue;
        }
        // The centered image is largest and opaque; neighbours shrink and fade.
        let scale = 1.0 - 0.18 * dist;
        sprite.custom_size = Some(Vec2::new(center_w * scale, center_h * scale));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, (1.0 - 0.22 * dist).max(0.0));
        transform.translation = Vec3::new(rel * spacing, 0.0, 10.0 - dist);
        *visibility = Visibility::Visible;
    }

    if let Ok((mut text, mut transform, mut visibility)) = name_query.single_mut() {
        if changed || !*initialized {
            text.0 = display_table_line(&tables.indexed_tables[selected]);
        }
        transform.translation = Vec3::new(0.0, -(center_h / 2.0) - 40.0, 20.0);
        *visibility = Visibility::Visible;
    }

    *initialized = true;
}

/// The display name for a table: its table name, falling back to the file stem.
pub(crate) fn display_table_line(table: &IndexedTable) -> String {
    let file_name = table
        .path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    match &table.table_info.table_name {
        Some(name) if !name.trim().is_empty() => capitalize_first_letter(name),
        _ => capitalize_first_letter(&file_name),
    }
}

fn capitalize_first_letter(s: &str) -> String {
    s[0..1].to_uppercase() + &s[1..]
}
