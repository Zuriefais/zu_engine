use egui::{self, Context};

use wasmtime::Engine;

use super::engine_mod::EngineMod;

pub struct ModManager {
    engine: Engine,
    modules: Vec<EngineMod>,
}

impl ModManager {
    fn load_mod(&mut self, mod_path: &str) -> anyhow::Result<()> {
        self.modules.push(EngineMod::new(mod_path, &self.engine)?);
        Ok(())
    }

    pub fn new() -> Self {
        let engine = Engine::default();

        Self {
            engine,
            modules: Default::default(),
        }
    }
}
