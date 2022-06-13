use bevy::{ecs::event::Events, prelude::*};

use async_std::channel::{bounded, Receiver, Sender};

use bevy_egui::{
    egui::{self},
    EguiContext, EguiPlugin,
};
use web3::{
    transports::eip_1193::{self},
    types::H160,
};

fn startup_system(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

#[derive(Default)]
pub struct WalletAddress(Option<H160>);

#[derive(Default)]
pub struct Web3State {
    pub wallet_address: WalletAddress,
}

impl Web3State {
    pub fn new(n: Option<H160>) -> Self {
        Self {
            wallet_address: WalletAddress(n),
        }
    }
}

#[derive(Default)]
pub struct Send(Web3State);

#[derive(Default)]
pub struct Receive(Web3State);

fn receive(
    receiver: ResMut<Receiver<Web3State>>,
    mut events: ResMut<Events<Receive>>,
    mut state: ResMut<Web3State>,
) {
    if let Ok(ev) = receiver.try_recv() {
        println!("Receiving");
        state.wallet_address.0 = ev.wallet_address.0;
        events.send(Receive(ev));
    }
}

fn send(sender: ResMut<Sender<Web3State>>, mut reader: EventReader<Send>) {
    for ev in reader.iter() {
        let ev = Web3State::new(ev.0.wallet_address.0);
        if let Err(_e) = sender.try_send(ev) {
            println!("Error sending event");
        }
    }
}

pub fn header(
    mut egui_context: ResMut<EguiContext>,
    state: Res<Web3State>,
    sender: ResMut<Sender<Web3State>>,
) {
    egui::CentralPanel::default().show(egui_context.ctx_mut(), |ui| {
        let sender = sender.clone();

        let provider = eip_1193::Provider::default().unwrap().unwrap();
        let transport = eip_1193::Eip1193::new(provider);
        let web3 = web3::Web3::new(transport);

        if ui.button("MetaMask").clicked() {
            wasm_bindgen_futures::spawn_local(async move {
                let accounts = web3.eth().request_accounts().await;
                if let Ok(addr) = accounts {
                    if !addr.is_empty() {
                        sender
                            .send(Web3State {
                                wallet_address: WalletAddress(Some(addr[0])),
                            })
                            .await
                            .unwrap();
                    }
                }
            });
        }

        if let Some(addr) = &state.wallet_address.0 {
            ui.label(addr.to_string());
        }
    });
}

fn main() {
    let (sender, receiver) = bounded::<Web3State>(1);
    let state = Web3State::default();
    let mut builder = App::new();
    // add plugins
    builder.add_plugins(DefaultPlugins);
    builder.add_plugin(EguiPlugin);

    // add systems
    builder
        .add_startup_system(startup_system)
        .insert_resource(sender)
        .insert_resource(receiver)
        .insert_resource(state)
        .add_event::<Send>()
        .add_event::<Receive>()
        .add_system(send)
        .add_system(receive)
        .add_system(header);
    builder.run();
}
