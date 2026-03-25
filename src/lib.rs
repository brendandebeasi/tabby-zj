mod click;
mod colors;
mod config;
mod grouping;
mod indicators;
mod input;
mod menus;
mod persistence;
mod render;
mod state;
mod widgets;
mod workers;

use state::PluginState;
use std::collections::BTreeMap;
use zellij_tile::prelude::*;

register_plugin!(TabbyZjPlugin);

// Required for Zellij WASM compatibility
#[no_mangle]
pub extern "C" fn _start() {}

#[derive(Default)]
struct TabbyZjPlugin {
    state: PluginState,
}

impl ZellijPlugin for TabbyZjPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        let ids = get_plugin_ids();
        self.state.plugin_id = ids.plugin_id;

        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::Key,
            EventType::Mouse,
            EventType::Timer,
            EventType::RunCommandResult,
            EventType::CwdChanged,
            EventType::Visible,
        ]);

        set_timeout(1.0);
        self.state.load_config(configuration);
        self.state.load_persisted_state();
    }

    fn update(&mut self, event: Event) -> bool {
        input::handle_event(&mut self.state, event)
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        input::handle_pipe(&mut self.state, pipe_message)
    }

    fn render(&mut self, rows: usize, cols: usize) {
        self.state.rows = rows;
        self.state.cols = cols;
        render::render_sidebar(&mut self.state, rows, cols);
    }
}
