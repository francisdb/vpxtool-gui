/// Handles the communication between ano other thread and Bevy
use bevy::app::{App, PreStartup, Update};
use bevy::prelude::{Commands, Deref, Event, Message, MessageWriter, Res, Resource};
use crossbeam_channel::{Receiver, Sender, bounded};
use vpxtool::indexer::IndexedTable;

#[derive(Event, Debug)]
pub(crate) enum ChannelExternalEvent {
    VpxDone,
    ProgressLength(u64),
    ProgressPosition(u64),
    ProgressFinishAndClear,
    TablesLoaded(Vec<IndexedTable>),
}

#[derive(Resource, Deref)]
pub(crate) struct StreamReceiver(Receiver<ChannelExternalEvent>);

#[derive(Resource, Deref)]
pub(crate) struct StreamSender(Sender<ChannelExternalEvent>);

#[derive(Message, Debug)]
pub(crate) struct ExternalEvent(pub(crate) ChannelExternalEvent);

pub(crate) fn plugin(app: &mut App) {
    app.add_message::<ExternalEvent>();
    // startup systems need access to the channel
    app.add_systems(PreStartup, setup_channel);
    app.add_systems(Update, forward_events_to_bevy);
}

// This system reads from the receiver and sends events to Bevy
pub(crate) fn forward_events_to_bevy(
    receiver: Res<StreamReceiver>,
    mut events: MessageWriter<ExternalEvent>,
) {
    let _event_writer = &events;
    for from_stream in receiver.try_iter() {
        events.write(ExternalEvent(from_stream));
    }
}

fn setup_channel(mut commands: Commands) {
    let (tx, rx) = bounded::<ChannelExternalEvent>(10);
    commands.insert_resource(StreamSender(tx));
    commands.insert_resource(StreamReceiver(rx));
}
