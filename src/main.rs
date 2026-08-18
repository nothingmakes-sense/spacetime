mod app;
mod assets;
mod config;
mod game_mode;
mod input;
mod multiplayer;
mod physics;
mod player;
mod vulkan;
mod world;
mod module_bindings;

use anyhow::{Context, Result};
use log::info;
use winit::event_loop::{ControlFlow, EventLoop};

use app::App;
use config::{SPACETIME_DB_NAME, SPACETIME_URI};
use game_mode::GameMode;
use multiplayer::Multiplayer;
use world::LocalWorld;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_default_filter()).init();
    info!("Starting game client…");

    let mode = match try_connect_multiplayer() {
        Ok(net) => {
            info!("Multiplayer mode active");
            GameMode::Multiplayer {
                net,
                remote_players: Vec::new(),
            }
        }
        Err(e) => {
            log::warn!("Could not connect to SpacetimeDB ({SPACETIME_URI}): {e}");
            log::warn!("Falling back to Single-Player mode.");
            GameMode::SinglePlayer {
                world: LocalWorld::generate_basic(),
            }
        }
    };

    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(mode)?;
    event_loop.run_app(&mut app).context("Event loop error")?;

    Ok(())
}

fn env_default_filter() -> env_logger::Env<'static> {
    env_logger::Env::default().default_filter_or("info")
}

fn try_connect_multiplayer() -> Result<Multiplayer> {
    let net = Multiplayer::connect(SPACETIME_URI, SPACETIME_DB_NAME)
        .context("SpacetimeDB connection failed")?;

    std::thread::sleep(std::time::Duration::from_millis(150));

    if !net.is_connected() {
        return Err(anyhow::anyhow!(
            "Connected but identity not received – server may be down or rejecting clients"
        ));
    }

    net.subscribe_players();
    net.register_callbacks();
    Ok(net)
}
