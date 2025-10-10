use crate::guifrontend::VpxTables;
use crate::input::{TableSelectionChanged, wrap_around};
use bevy::color::palettes::css::{GHOST_WHITE, GOLD};
use bevy::prelude::*;
use std::cmp::Ordering;
use vpxtool::indexer::IndexedTable;

#[derive(Component, Debug)]
pub(crate) struct TableText {
    pub(crate) list_index: usize,
    pub(crate) table_text: String,
}

#[derive(Resource, Default)]
pub struct SelectedItem {
    pub(crate) index: Option<usize>,
}

#[derive(Component)]
pub struct TextItem;

#[derive(Bundle)]
struct MenuTextBundle {
    text: Text,
    text_font: TextFont,
    text_color: TextColor,
    text_node: Node,
    table_text: TableText,
    text_item: TextItem,
}

const ITEMS_AROUND_SELECTED: usize = 10;
const ITEMS_SHOWN: usize = ITEMS_AROUND_SELECTED * 2 + 1;

pub(crate) fn list_plugin(app: &mut App) {
    app.insert_resource(SelectedItem::default());
    app.add_systems(Startup, create_list);
    app.add_systems(Update, handle_table_selection_changed);
}

fn create_list(mut commands: Commands) {
    for list_index in 0..ITEMS_SHOWN {
        let distance = (list_index as i32 - ITEMS_AROUND_SELECTED as i32).abs() as f32;
        let alpha = 1.0 - (distance / ITEMS_AROUND_SELECTED as f32);
        let mut text_color = TextColor::from(Color::srgba(
            GHOST_WHITE.red,
            GHOST_WHITE.green,
            GHOST_WHITE.blue,
            alpha,
        ));
        let mut font_size = 15.0;

        if list_index == ITEMS_AROUND_SELECTED {
            text_color = TextColor::from(GOLD);
            font_size = 25.0;
        }

        let top = match list_index.cmp(&ITEMS_AROUND_SELECTED) {
            Ordering::Less => Val::Px(25. + (((list_index as f32) + 1.) * 20.)),
            Ordering::Equal => Val::Px(255. + (((list_index as f32) - 10.5) * 20.)),
            Ordering::Greater => Val::Px(255. + (((list_index as f32) - 10.) * 20.)),
        };

        commands.spawn(MenuTextBundle {
            text: Text::new(""),
            text_font: TextFont {
                font_size,
                ..default()
            },
            text_color,
            text_node: Node {
                // Set the justification of the Text
                //.with_text_justify(JustifyText::Center)
                display: Display::Block,
                position_type: PositionType::Absolute,
                left: Val::Px(20.),
                top,
                right: Val::Px(0.),
                ..default()
            },
            table_text: TableText {
                list_index,
                table_text: "".to_string(),
            },
            text_item: TextItem,
        });
    }
}

fn handle_table_selection_changed(
    mut event_reader: MessageReader<TableSelectionChanged>,
    tables: Res<VpxTables>,
    mut text_items: Query<(&mut TableText, &mut Text), With<TextItem>>,
    selected_item: Res<SelectedItem>,
) {
    for _event in event_reader.read() {
        let selected_item = selected_item.index.unwrap_or(0);
        let table_indices = generate_table_indices(tables.indexed_tables.len(), selected_item);
        for (mut table_text, mut text) in text_items.iter_mut() {
            let list_index = table_text.list_index;
            let (table_name, table_description) = if tables.indexed_tables.is_empty() {
                ("".to_string(), "".to_string())
            } else {
                let table_index = table_indices[list_index];
                let table = &tables.indexed_tables[table_index];
                let table_name = display_table_line(table);
                let table_description = table
                    .table_info
                    .table_description
                    .clone()
                    .unwrap_or("Description missing".to_string());
                (table_name, table_description)
            };

            table_text.table_text = table_description;
            text.0 = table_name;
        }
    }
}

pub(crate) fn display_table_line(table: &IndexedTable) -> String {
    let file_name = table
        .path
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    Some(table.table_info.table_name.to_owned())
        .filter(|s| !s.clone().unwrap_or_default().trim().is_empty())
        .map(|s| {
            match s {
                Some(name) => capitalize_first_letter(&name),
                None => capitalize_first_letter(&file_name),
            }
            // TODO we probably want to show both the file name and the table name
        })
        .unwrap_or(file_name)
}

fn capitalize_first_letter(s: &str) -> String {
    s[0..1].to_uppercase() + &s[1..]
}

fn generate_table_indices(max_index: usize, selected_index: usize) -> [usize; ITEMS_SHOWN] {
    let mut table_indices = [0; ITEMS_SHOWN];
    for (i, item) in table_indices.iter_mut().enumerate() {
        let index = ITEMS_AROUND_SELECTED as i16 - i as i16;
        *item = wrap_around(selected_index as i16 - index, max_index);
    }
    table_indices
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_lookup_table_indices() {
        assert_eq!(
            generate_table_indices(10, 0),
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0,
            ]
        );
        assert_eq!(
            generate_table_indices(50, 8),
            [
                48, 49, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18
            ]
        );
    }

    #[test]
    fn test_wrap() {
        assert_eq!(wrap_around(0, 0), 0);
        assert_eq!(wrap_around(0, 10), 0);
        assert_eq!(wrap_around(10, 0), 0);
        assert_eq!(wrap_around(10, 10), 0);
        assert_eq!(wrap_around(11, 10), 1);
        assert_eq!(wrap_around(-1, 10), 9);
        assert_eq!(wrap_around(-10, 10), 0);
        assert_eq!(wrap_around(-11, 10), 9);
        assert_eq!(wrap_around(-123, 3), 0);
        assert_eq!(wrap_around(91, 9), 1);
    }
}
