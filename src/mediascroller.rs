use crate::guifrontend::VpxTables;
use crate::input::wrap_around;
use crate::loading::LoadingState;
use bevy::asset::AssetLoadFailedEvent;
use bevy::color::palettes::css::GHOST_WHITE;
use bevy::ecs::system::SystemId;
use bevy::image::Image;
use bevy::log::debug;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_asset::AssetServer;
use vpxtool::indexer::IndexedTable;

/// The currently selected table.
#[derive(Resource, Default)]
pub struct SelectedItem {
    pub(crate) index: Option<usize>,
}

/// One slot in the horizontal scroller. There is a fixed number of slots; each
/// shows the table at `selected + offset` and (re)loads its image lazily.
#[derive(Component)]
struct ScrollerSlot {
    offset: i32,
    /// The table currently shown in this slot, to avoid reloading every frame.
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
/// Fallback image (in `assets/`) used when a table has no `media/table.jpg`.
const FALLBACK_IMAGE: &str = "generic_table.png";

pub(crate) fn mediascroller_plugin(app: &mut App) {
    app.insert_resource(SelectedItem::default());
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

/// Spawn the fixed set of scroller slots and the name label (all hidden until
/// the first layout in [`update_scroller`]).
fn spawn_scroller(mut commands: Commands) {
    for offset in -VISIBLE_SIDE..=VISIBLE_SIDE {
        commands.spawn((
            Sprite::default(),
            Transform::from_xyz(0., 0., 0.),
            Visibility::Hidden,
            ScrollerSlot {
                offset,
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

/// Resolve the image for a table: `media/table.jpg` next to the .vpx file, or
/// the bundled generic desktop-table view. Returns the handle and a human
/// readable description for logging.
fn resolve_table_image(asset_server: &AssetServer, info: &IndexedTable) -> (Handle<Image>, String) {
    let media = info
        .path
        .parent()
        .map(|p| p.join("media").join("table.jpg"));
    match media {
        Some(path) if path.exists() => {
            let handle = asset_server
                .load_builder()
                .override_unapproved()
                .load(path.clone());
            (handle, path.display().to_string())
        }
        _ => (
            asset_server.load(FALLBACK_IMAGE),
            format!("{FALLBACK_IMAGE} (fallback)"),
        ),
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

/// Lay out the scroller around the selected table and (re)load slot images on
/// demand. Only the visible window of images is kept loaded at any time.
#[allow(clippy::type_complexity)]
fn update_scroller(
    asset_server: Res<AssetServer>,
    mut last_selected: Local<Option<usize>>,
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
    let table_count = tables.indexed_tables.len();
    if table_count == 0 {
        return;
    }
    let selected = selected_item_res.index.unwrap_or(0);
    // Only relayout when the selection actually changed. This also drives the
    // initial layout (last_selected starts as None), so the scroller appears
    // without depending on an input event.
    if *last_selected == Some(selected) {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    *last_selected = Some(selected);

    let center_w = window.width() * 0.6;
    let center_h = center_w / 1.5;
    let spacing = window.width() * 0.62;

    for (mut slot, mut sprite, mut transform, mut visibility) in slots.iter_mut() {
        let table_index = wrap_around(selected as i16 + slot.offset as i16, table_count);
        // Reload only when this slot now shows a different table. Replacing the
        // sprite's handle drops the previous one, so images scroll out of memory.
        if slot.table_index != Some(table_index) {
            let info = &tables.indexed_tables[table_index];
            let (handle, desc) = resolve_table_image(&asset_server, info);
            sprite.image = handle;
            slot.table_index = Some(table_index);
            if slot.offset == 0 {
                info!(
                    "Selected table '{}' image: {}",
                    display_table_line(info),
                    desc
                );
            } else {
                debug!(
                    "Scroller slot {} -> '{}' image: {}",
                    slot.offset,
                    display_table_line(info),
                    desc
                );
            }
        }

        let dist = slot.offset.abs() as f32;
        // The centered image is largest and opaque; neighbours shrink and fade.
        let scale = 1.0 - 0.18 * dist;
        sprite.custom_size = Some(Vec2::new(center_w * scale, center_h * scale));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, 1.0 - 0.22 * dist);
        // Keep the centered image in front of its neighbours.
        transform.translation = Vec3::new(slot.offset as f32 * spacing, 0.0, 10.0 - dist);
        *visibility = Visibility::Visible;
    }

    if let Ok((mut text, mut transform, mut visibility)) = name_query.single_mut() {
        text.0 = display_table_line(&tables.indexed_tables[selected]);
        transform.translation = Vec3::new(0.0, -(center_h / 2.0) - 40.0, 20.0);
        *visibility = Visibility::Visible;
    }
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
