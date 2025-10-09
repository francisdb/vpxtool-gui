use crate::guifrontend::VpxTables;
use crate::list::{SelectedItem, display_table_line};
use crate::loading::LoadingState;
use bevy::input::ButtonInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

const FONT_SIZE_TITLE: f32 = 20.;
const FONT_SIZE_BODY: f32 = 14.;
const LINE_HEIGHT: f32 = 21.;

// marker for the ui node
#[derive(Component)]
struct InfoNode;

#[derive(Component)]
struct TitleNode;

#[derive(Component)]
struct BodyNode;

pub(crate) fn info_plugin(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, send_scroll_events)
        .add_systems(Update, toggle_visibility)
        .add_systems(Update, update_ui.run_if(in_state(LoadingState::Ready)))
        .add_observer(on_scroll_handler);
}

fn setup(mut commands: Commands, _asset_server: Res<AssetServer>) {
    // TODO use bevy ui to create a window with a scrollable text area

    // Camera
    //commands.spawn((Camera2d, IsDefaultUiCamera));

    // Font
    // let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

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
            parent.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Start,
                    align_items: AlignItems::Center,
                    width: percent(100),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                children![
                    (
                        // Title
                        (
                            Node {
                                width: Val::Percent(100.0),
                                padding: UiRect::all(Val::Px(8.0)),
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.05, 0.05, 0.05))
                        ),
                        children![(
                            Text::new("[No title]"),
                            TextFont {
                                //font: font_handle.clone(),
                                font_size: FONT_SIZE_TITLE,
                                ..default()
                            },
                            Label,
                            TitleNode
                        )],
                    ),
                    (
                        // Scrolling description
                        Node {
                            padding: UiRect::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            align_self: AlignSelf::Stretch,
                            overflow: Overflow::scroll_y(), // n.b.
                            ..default()
                        },
                        children![(
                            Text::new("[no description]"),
                            TextFont {
                                font_size: FONT_SIZE_BODY,
                                ..default()
                            },
                            BodyNode
                        )],
                    ),
                ],
            ));
        });
}

fn toggle_visibility(
    mut query: Query<&mut Visibility, With<InfoNode>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::Digit1) {
        for mut visibility in &mut query {
            *visibility = if *visibility == Visibility::Visible {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }
    }
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

fn update_ui(
    selected_item_res: Res<SelectedItem>,
    tables: Res<VpxTables>,
    mut title_text: Single<&mut Text, With<TitleNode>>,
    mut body_text: Single<&mut Text, (With<BodyNode>, Without<TitleNode>)>,
) {
    // TODO this should only happen when the selected item changes
    // and also reset the scroll position of the body text area to the top

    // TODO when an item is selected, update the title and the text area with the table description
    let selected_item = selected_item_res.index.unwrap_or(0);
    let table = &tables.indexed_tables[selected_item];
    let gametext = table
        .table_info
        .table_description
        .clone()
        .filter(|x| !x.trim().is_empty())
        .unwrap_or("[no description]".to_string());
    let title = display_table_line(table);

    title_text.0 = title;
    body_text.0 = gametext;
}
