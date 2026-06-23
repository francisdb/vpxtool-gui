use crate::guifrontend::VpxTables;
use crate::input::TableSelectionChanged;
use crate::mediascroller::{SelectedItem, display_table_line};
use bevy::input::ButtonInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;
use bevy::ui_widgets::{ControlOrientation, Scrollbar, ScrollbarThumb};

const FONT_SIZE_TITLE: f32 = 20.;
const FONT_SIZE_BODY: f32 = 14.;
const LINE_HEIGHT: f32 = 21.;

/// Scrollbar track: a dark, subtle gutter behind the thumb.
const SCROLLBAR_TRACK: Color = Color::srgba(0.0, 0.0, 0.0, 0.25);
/// Scrollbar thumb: the draggable handle.
const SCROLLBAR_THUMB: Color = Color::srgba(0.45, 0.55, 0.70, 0.9);

// marker for the ui node
#[derive(Component)]
struct InfoNode;

#[derive(Component)]
struct TitleNode;

#[derive(Component)]
struct BodyNode;

#[derive(Component)]
struct BodyScroller;

/// Which section the info panel shows; `1` cycles Off -> Description -> Rules.
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
enum InfoMode {
    #[default]
    Off,
    Description,
    Rules,
}

pub(crate) fn info_plugin(app: &mut App) {
    app.init_resource::<InfoMode>()
        .add_systems(Startup, setup)
        .add_systems(Update, send_scroll_events)
        .add_systems(Update, cycle_info_mode)
        .add_systems(Update, update_info)
        .add_observer(on_scroll_handler);
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Percent(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            GlobalZIndex(1000), // on top of everything
            Visibility::Hidden,
            InfoNode,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                ))
                .with_children(|panel| {
                    // Title bar
                    panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                        ))
                        .with_children(|title| {
                            title.spawn((
                                Text::new("[No title]"),
                                TextFont {
                                    font_size: FontSize::Px(FONT_SIZE_TITLE),
                                    ..default()
                                },
                                Label,
                                TitleNode,
                            ));
                        });

                    // Scrolling body sits in a row next to a draggable scrollbar.
                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Stretch,
                            align_self: AlignSelf::Stretch,
                            flex_grow: 1.0,
                            min_height: px(0),
                            padding: UiRect::all(Val::Px(8.0)),
                            column_gap: px(6),
                            ..default()
                        })
                        .with_children(|row| {
                            let scroller = row
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        flex_grow: 1.0,
                                        min_height: px(0),
                                        overflow: Overflow::scroll_y(), // n.b.
                                        ..default()
                                    },
                                    BodyScroller,
                                ))
                                .with_children(|body| {
                                    body.spawn((
                                        Text::new("[no description]"),
                                        TextFont {
                                            font_size: FontSize::Px(FONT_SIZE_BODY),
                                            ..default()
                                        },
                                        BodyNode,
                                    ));
                                })
                                .id();

                            // Draggable scrollbar (bevy_ui_widgets headless widget): it
                            // writes the scroller's ScrollPosition when the thumb is
                            // dragged. We own the visuals: a track gutter plus a thumb.
                            row.spawn((
                                Node {
                                    width: px(10),
                                    border_radius: BorderRadius::all(px(5)),
                                    ..default()
                                },
                                BackgroundColor(SCROLLBAR_TRACK),
                                Scrollbar::new(scroller, ControlOrientation::Vertical, 32.0),
                                children![(
                                    ScrollbarThumb {
                                        border_radius: BorderRadius::all(px(5)),
                                        ..default()
                                    },
                                    BackgroundColor(SCROLLBAR_THUMB),
                                )],
                            ));
                        });
                });
        });
}

/// `1` cycles the info panel: Off -> Description -> Rules -> Off.
fn cycle_info_mode(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<InfoMode>) {
    if keys.just_pressed(KeyCode::Digit1) {
        *mode = match *mode {
            InfoMode::Off => InfoMode::Description,
            InfoMode::Description => InfoMode::Rules,
            InfoMode::Rules => InfoMode::Off,
        };
    }
}

/// Update the panel's visibility and contents when the mode or selection changes.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_info(
    mode: Res<InfoMode>,
    mut selection_changed: MessageReader<TableSelectionChanged>,
    selected_item_res: Res<SelectedItem>,
    tables: Res<VpxTables>,
    mut info_visibility: Query<&mut Visibility, With<InfoNode>>,
    mut title_text: Single<&mut Text, With<TitleNode>>,
    mut body_text: Single<&mut Text, (With<BodyNode>, Without<TitleNode>)>,
    mut scroll_query: Query<&mut ScrollPosition, With<BodyScroller>>,
) {
    let selection_changed = selection_changed.read().count() > 0;
    if !mode.is_changed() && !selection_changed {
        return;
    }

    if let Ok(mut visibility) = info_visibility.single_mut() {
        *visibility = if *mode == InfoMode::Off {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }

    if *mode == InfoMode::Off || tables.indexed_tables.is_empty() {
        return;
    }

    let selected = selected_item_res.index.unwrap_or(0);
    let table = &tables.indexed_tables[selected];
    let name = display_table_line(table);
    let (title, body) = match *mode {
        InfoMode::Description => (
            name,
            table
                .table_info
                .table_description
                .clone()
                .filter(|x| !x.trim().is_empty())
                .unwrap_or_else(|| "[no description]".to_string()),
        ),
        InfoMode::Rules => (
            format!("{name} - Rules"),
            table
                .table_info
                .table_rules
                .clone()
                .filter(|x| !x.trim().is_empty())
                .unwrap_or_else(|| "[no rules]".to_string()),
        ),
        InfoMode::Off => unreachable!(),
    };

    // Start new content at the top.
    if let Ok(mut scroll_position) = scroll_query.single_mut() {
        scroll_position.x = 0.0;
        scroll_position.y = 0.0;
    }
    title_text.0 = title;
    body_text.0 = body;
}

/// UI scrolling event.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    /// Scroll delta in logical coordinates.
    delta: Vec2,
}

fn on_scroll_handler(
    mut scroll: On<Scroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    let delta = &mut scroll.delta;
    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.x > 0. {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.
        };

        if !max {
            scroll_position.x += delta.x;
            // Consume the X portion of the scroll delta.
            delta.x = 0.;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.y > 0. {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.
        };

        if !max {
            scroll_position.y += delta.y;
            // Consume the Y portion of the scroll delta.
            delta.y = 0.;
        }
    }

    // Stop propagating when the delta is fully consumed.
    if *delta == Vec2::ZERO {
        scroll.propagate(false);
    }
}

/// Injects scroll events into the UI hierarchy.
fn send_scroll_events(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= LINE_HEIGHT;
        }

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}
