use log::*;
use wasmtime::component::{HasData, ResourceTable, bindgen};
use wasmtime_wasi::{WasiCtx, WasiView};

bindgen!("zu-mod" in "../../engine.wit");

pub struct ScriptingState {
    //Wasi spacific fields
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for ScriptingState {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

impl zu::engine::core::Host for ScriptingState {
    fn info(&mut self, text: String) -> () {
        info!("{}", text)
    }

    fn warn(&mut self, text: String) -> () {
        warn!("{}", text)
    }

    fn error(&mut self, text: String) -> () {
        error!("{}", text)
    }

    fn debug(&mut self, text: String) -> () {
        debug!("{}", text)
    }

    fn trace(&mut self, text: String) -> () {
        trace!("{}", text)
    }
}

impl HasData for ScriptingState {
    type Data<'a> = &'a mut ScriptingState;
}
