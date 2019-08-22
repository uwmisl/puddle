use wasm_bindgen::prelude::*;
use web_sys::console;

use serde::Deserialize;

use puddle_core::{
    grid::{DropletId, Grid, Location},
    process::{Manager, ProcessHandle, ProcessId},
};

#[wasm_bindgen]
pub struct System {
    manager: Manager,
    pid: ProcessId,
}

fn stringify(x: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&x.to_string())
}

fn serialize(x: impl serde::Serialize) -> JsValue {
    JsValue::from_serde(&x).unwrap()
}

#[derive(Deserialize)]
pub struct CreateArgs {
    location: Option<Location>,
    vol: f64,
    dim: Option<Location>,
}

#[wasm_bindgen]
impl System {
    #[wasm_bindgen]
    pub fn new() -> System {
        let blocking = false;
        let grid = Grid::rectangle(10, 10);
        let manager = Manager::new(blocking, grid);
        let pid = manager.new_process("js").unwrap();
        System { manager, pid }
    }

    fn get_process(&self) -> Result<ProcessHandle, JsValue> {
        self.manager.get_process(self.pid).map_err(stringify)
    }

    #[wasm_bindgen]
    pub fn create(&mut self, args: JsValue) -> Result<JsValue, JsValue> {
        let args: CreateArgs = args.into_serde().map_err(stringify)?;
        let p = self.get_process()?;
        p.create(args.location, args.vol, args.dim)
            .map(serialize)
            .map_err(stringify)
    }

    #[wasm_bindgen]
    pub fn mix(&mut self, d1: JsValue, d2: JsValue) -> Result<JsValue, JsValue> {
        let d1: DropletId = d1.into_serde().map_err(stringify)?;
        let d2: DropletId = d2.into_serde().map_err(stringify)?;
        let p = self.get_process()?;
        p.mix(d1, d2).map(serialize).map_err(stringify)
    }

    #[wasm_bindgen]
    pub fn split(&mut self, d: JsValue) -> Result<JsValue, JsValue> {
        let d: DropletId = d.into_serde().map_err(stringify)?;
        let p = self.get_process()?;
        p.split(d).map(serialize).map_err(stringify)
    }

    #[wasm_bindgen]
    pub fn flush(&mut self) -> Result<JsValue, JsValue> {
        let p = self.get_process()?;
        p.flush()
            .map_err(stringify)
            .map(|info| JsValue::from_serde(&info).unwrap())
    }

    #[wasm_bindgen(js_name = getLogs)]
    pub fn get_logs(&self) -> JsValue {
        let logs = self.manager.get_logs();
        JsValue::from_serde(&logs).unwrap()
    }
}
