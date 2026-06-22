use crate::loading::LoadingState;
use bevy::color::Alpha;
use bevy::color::palettes::css::GHOST_WHITE;
use bevy::prelude::*;
use std::time::Duration;

#[derive(Component)]
pub struct KeysBar;

#[derive(Component)]
struct KeysBarText;

/// How long the keys bar stays fully visible after the last key activity.
const VISIBLE_SECS: f32 = 2.5;
/// Time to fade fully in or out.
const FADE_SECS: f32 = 0.3;
/// Background opacity of the bar when fully faded in.
const BG_ALPHA: f32 = 0.5;

pub(crate) fn keys_bar_plugin(app: &mut App) {
    app.add_systems(Startup, create_keys_bar);
    app.add_systems(
        Update,
        keys_bar_update.run_if(in_state(LoadingState::Ready)),
    );
}

/// Fade the keys bar in while keys are being pressed and out a short moment after.
fn keys_bar_update(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut timer: Local<Timer>,
    mut fade: Local<f32>,
    mut bar_query: Query<(&mut BackgroundColor, &mut Visibility), With<KeysBar>>,
    mut text_query: Query<&mut TextColor, With<KeysBarText>>,
) {
    // Any key held or just pressed (re)arms the visibility timer.
    if keys.get_pressed().next().is_some() {
        timer.set_duration(Duration::from_secs_f32(VISIBLE_SECS));
        timer.reset();
    }
    timer.tick(time.delta());

    // Ease the fade factor toward 1 while active, 0 once the timer elapses.
    let target = if timer.is_finished() { 0.0 } else { 1.0 };
    let step = time.delta_secs() / FADE_SECS;
    *fade = if *fade < target {
        (*fade + step).min(target)
    } else {
        (*fade - step).max(target)
    };

    for (mut bg, mut visibility) in bar_query.iter_mut() {
        bg.0.set_alpha(BG_ALPHA * *fade);
        // Skip rendering entirely once fully faded out.
        *visibility = if *fade <= 0.0 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    for mut color in text_query.iter_mut() {
        color.0.set_alpha(*fade);
    }
}

fn create_keys_bar(mut commands: Commands) {
    commands.spawn((
        // Full-width bar anchored to the bottom, centering its text child.
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(0.),
            left: Val::Px(0.),
            width: Val::Percent(100.),
            padding: UiRect::axes(Val::Px(12.), Val::Px(5.)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        // Dark, 50% opaque overlay; starts fully transparent and fades in.
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        // Hidden until the first key press; toggled by keys_bar_update.
        Visibility::Hidden,
        KeysBar,
        children![(
            Text::new(
                "q: quit    |    1: table info    |    left-shift / right-shift: scroll    |    enter: launch",
            ),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(GHOST_WHITE.with_alpha(0.0).into()),
            TextLayout {
                justify: Justify::Center,
                linebreak: LineBreak::NoWrap,
            },
            KeysBarText,
        )],
    ));
}
