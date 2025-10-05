#[cfg(not(target_os = "android"))]
use wasmtime::component::Component;
use wasmtime::{Engine, Store, component::Linker};
use wasmtime_wasi::{ResourceTable, WasiCtxBuilder};

use crate::functions::{ScriptingState, ZuMod};

pub struct EngineMod {
    pub path: String,
    pub bindings: ZuMod,
    pub store: Store<ScriptingState>,
}

impl EngineMod {
    pub fn new(mod_path: &str, engine: &Engine) -> anyhow::Result<Self> {
        let component = Component::from_file(&engine, &mod_path)?;

        let mut linker: Linker<ScriptingState> = Linker::new(&engine);

        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        ZuMod::add_to_linker::<ScriptingState, ScriptingState>(&mut linker, |state| state)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();

        let scripting_state = ScriptingState {
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
        };

        let mut store = Store::new(&engine, scripting_state);

        let bindings = ZuMod::instantiate(&mut store, &component, &linker)?;

        bindings.call_init(&mut store)?;
        Ok(Self {
            path: mod_path.to_string(),
            bindings,
            store,
        })
    }
}
