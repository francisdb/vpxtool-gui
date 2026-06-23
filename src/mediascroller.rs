use crate::guifrontend::VpxTables;
use crate::input::TableSelectionChanged;
use crate::loading::{LoadingData, LoadingState};
use bevy::color::palettes::css::GHOST_WHITE;
use bevy::ecs::system::SystemId;
use bevy::image::Image;
use bevy::log::debug;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_asset::{AssetId, AssetServer};
use std::collections::HashMap;
use vpxtool::indexer::IndexedTable;

/// Maps loaded image asset ids to a human readable table name. Used by the
/// loading screen to report which asset is being loaded.
#[derive(Resource, Default)]
pub struct AssetPaths {
    pub paths: HashMap<AssetId<Image>, String>,
}

/// The currently selected table.
#[derive(Resource, Default)]
pub struct SelectedItem {
    pub(crate) index: Option<usize>,
}

/// One table image in the horizontal scroller.
#[derive(Component)]
struct ScrollerItem {
    item_number: usize,
}

/// The table name shown below the centered table image.
#[derive(Component)]
struct ScrollerName;

/// System that (re)loads the table images and spawns the scroller sprites.
#[derive(Resource)]
pub(crate) struct LoadImagesSystem(pub(crate) SystemId);

/// Number of images shown on each side of the centered one.
const VISIBLE_SIDE: i32 = 3;
/// Fallback image (in `assets/`) used when a table has no `media/table.jpg`.
const FALLBACK_IMAGE: &str = "generic_table.png";

pub(crate) fn mediascroller_plugin(app: &mut App) {
    app.insert_resource(AssetPaths::default());
    app.insert_resource(SelectedItem::default());
    let load_system = app.register_system(load_table_images);
    app.insert_resource(LoadImagesSystem(load_system));
    app.add_systems(Startup, spawn_scroller_name);
    app.add_systems(
        Update,
        update_scroller.run_if(in_state(LoadingState::Ready)),
    );
}

fn spawn_scroller_name(mut commands: Commands) {
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

/// (Re)load the `media/table.jpg` image for every table and spawn a hidden
/// sprite per table. Run when entering the image loading state.
fn load_table_images(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_data: ResMut<LoadingData>,
    vpx_tables: Res<VpxTables>,
    mut asset_paths: ResMut<AssetPaths>,
    existing: Query<Entity, With<ScrollerItem>>,
) {
    // remove any sprites from a previous load
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    for (table_index, info) in vpx_tables.indexed_tables.iter().enumerate() {
        let media_path = info
            .path
            .parent()
            .map(|p| p.join("media").join("table.jpg"));
        let image_handle = match media_path {
            Some(path) if path.exists() => {
                asset_server.load_builder().override_unapproved().load(path)
            }
            // No per-table image: use the bundled generic desktop table view.
            _ => asset_server.load(FALLBACK_IMAGE),
        };
        loading_data
            .loading_assets
            .push(image_handle.clone().into());

        let table_name = info
            .table_info
            .table_name
            .clone()
            .unwrap_or_else(|| "None".to_string());
        asset_paths.paths.insert(image_handle.id(), table_name);

        commands.spawn((
            Sprite {
                image: image_handle,
                ..default()
            },
            Transform::from_xyz(0., 0., 0.),
            Visibility::Hidden,
            ScrollerItem {
                item_number: table_index,
            },
        ));
    }
}

/// Lay out the scroller around the selected table and update the name label.
#[allow(clippy::type_complexity)]
fn update_scroller(
    mut event_reader: MessageReader<TableSelectionChanged>,
    mut scroller_query: Query<
        (&ScrollerItem, &mut Sprite, &mut Transform, &mut Visibility),
        Without<ScrollerName>,
    >,
    mut name_query: Query<(&mut Text2d, &mut Transform, &mut Visibility), With<ScrollerName>>,
    selected_item_res: Res<SelectedItem>,
    tables: Res<VpxTables>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    for _event in event_reader.read() {
        let Ok(window) = window_query.single() else {
            return;
        };
        let table_count = tables.indexed_tables.len();
        if table_count == 0 {
            return;
        }
        let selected = selected_item_res.index.unwrap_or(0);

        let center_w = window.width() * 0.6;
        let center_h = center_w / 1.5;
        let spacing = window.width() * 0.62;

        for (item, mut sprite, mut transform, mut visibility) in scroller_query.iter_mut() {
            let d = circular_offset(item.item_number, selected, table_count);
            if d.abs() > VISIBLE_SIDE {
                *visibility = Visibility::Hidden;
                continue;
            }
            let dist = d.abs() as f32;
            // The centered image is the largest and fully opaque; neighbours
            // shrink and fade with distance.
            let scale = 1.0 - 0.18 * dist;
            sprite.custom_size = Some(Vec2::new(center_w * scale, center_h * scale));
            sprite.color = Color::srgba(1.0, 1.0, 1.0, 1.0 - 0.22 * dist);
            // Keep the centered image in front of its neighbours.
            transform.translation = Vec3::new(d as f32 * spacing, 0.0, 10.0 - dist);
            *visibility = Visibility::Visible;
        }

        if let Ok((mut text, mut transform, mut visibility)) = name_query.single_mut() {
            text.0 = display_table_line(&tables.indexed_tables[selected]);
            transform.translation = Vec3::new(0.0, -(center_h / 2.0) - 40.0, 20.0);
            *visibility = Visibility::Visible;
        }
    }
}

/// Shortest signed distance from `selected` to `item` on a ring of `count`
/// items, so the scroller wraps around at both ends.
fn circular_offset(item: usize, selected: usize, count: usize) -> i32 {
    let n = count as i32;
    let mut d = (item as i32 - selected as i32) % n;
    if d > n / 2 {
        d -= n;
    } else if d < -n / 2 {
        d += n;
    }
    d
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
        _ => {
            debug!("No table name for {}", table.path.display());
            capitalize_first_letter(&file_name)
        }
    }
}

fn capitalize_first_letter(s: &str) -> String {
    s[0..1].to_uppercase() + &s[1..]
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_circular_offset() {
        // within a small ring, distances wrap to the shorter direction
        assert_eq!(circular_offset(0, 0, 10), 0);
        assert_eq!(circular_offset(1, 0, 10), 1);
        assert_eq!(circular_offset(9, 0, 10), -1);
        assert_eq!(circular_offset(0, 9, 10), 1);
        assert_eq!(circular_offset(5, 0, 10), 5);
    }
}
