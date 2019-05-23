use std::sync::Arc;

use jsonrpc_core::{Error, ErrorCode};
use jsonrpc_derive::rpc;

use log::*;

use puddle_core::{DropletId, DropletInfo, Location, Manager, ProcessId, PuddleError};

pub struct RpcError(PuddleError);

type RpcResult<T> = std::result::Result<T, RpcError>;

impl From<PuddleError> for RpcError {
    fn from(p_err: PuddleError) -> Self {
        Self(p_err)
    }
}

impl From<RpcError> for Error {
    fn from(p_err: RpcError) -> Self {
        let code = ErrorCode::ServerError(0);
        let mut err = Error::new(code);
        err.message = format!("PuddleError: {:?}", p_err.0);
        err
    }
}

#[rpc]
pub trait Rpc {
    #[rpc(name = "new_process")]
    fn new_process(&self, name: String) -> RpcResult<ProcessId>;

    #[rpc(name = "close_process")]
    fn close_process(&self, pid: ProcessId) -> RpcResult<()>;

    #[rpc(name = "droplet_info")]
    fn droplet_info(&self, pid: ProcessId) -> RpcResult<Vec<DropletInfo>>;

    #[rpc(name = "visualizer_droplet_info")]
    fn visualizer_droplet_info(&self) -> RpcResult<Vec<DropletInfo>>;

    #[rpc(name = "flush")]
    fn flush(&self, pid: ProcessId) -> RpcResult<()>;

    #[rpc(name = "create")]
    fn create(
        &self,
        pid: ProcessId,
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
    ) -> RpcResult<DropletId>;

    #[rpc(name = "input")]
    fn input(&self, pid: ProcessId, name: String, vol: f64, dim: Location) -> RpcResult<DropletId>;

    #[rpc(name = "output")]
    fn output(&self, pid: ProcessId, name: String, d: DropletId) -> RpcResult<()>;

    #[rpc(name = "move")]
    fn move_droplet(&self, pid: ProcessId, d: DropletId, loc: Location) -> RpcResult<DropletId>;

    #[rpc(name = "mix")]
    fn mix(&self, pid: ProcessId, d1: DropletId, d2: DropletId) -> RpcResult<DropletId>;

    #[rpc(name = "combine_into")]
    fn combine_into(&self, pid: ProcessId, d1: DropletId, d2: DropletId) -> RpcResult<DropletId>;

    #[rpc(name = "split")]
    fn split(&self, pid: ProcessId, d: DropletId) -> RpcResult<(DropletId, DropletId)>;

    #[rpc(name = "heat")]
    fn heat(
        &self,
        pid: ProcessId,
        d: DropletId,
        temperature: f32,
        seconds: f64,
    ) -> RpcResult<DropletId>;
}

impl Rpc for Arc<Manager> {
    //
    // process management commands
    //

    fn new_process(&self, name: String) -> RpcResult<ProcessId> {
        // can't the function being implemented, use fully qualified name
        debug!("new_process(name={})", name);
        let pid = Manager::new_process(&self, name)?;
        Ok(pid)
    }

    fn close_process(&self, pid: ProcessId) -> RpcResult<()> {
        // can't the call function being implemented, use fully qualified name
        debug!("close_process(pid={})", pid);
        Manager::close_process(&self, pid)?;
        Ok(())
    }

    //
    // status commands
    //

    fn droplet_info(&self, pid: ProcessId) -> RpcResult<Vec<DropletInfo>> {
        debug!("droplet_info(pid={})", pid);
        let p = self.get_process(pid)?;
        let info = p.flush()?;
        Ok(info)
    }

    fn visualizer_droplet_info(&self) -> RpcResult<Vec<DropletInfo>> {
        debug!("visualizer_droplet_info()");
        // can't the function being implemented, use fully qualified name
        let info = Manager::visualizer_droplet_info(&self)?;
        Ok(info)
    }

    //
    // Droplet manipulation
    // delegate to process
    //

    fn flush(&self, pid: ProcessId) -> RpcResult<()> {
        debug!("flush(pid={})", pid);
        let p = self.get_process(pid)?;
        let _info = p.flush()?;
        Ok(())
    }

    fn create(
        &self,
        pid: ProcessId,
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
    ) -> RpcResult<DropletId> {
        debug!(
            "create(pid={}, loc={:?}, vol={}, dim={:?})",
            pid, loc, vol, dim
        );
        let p = self.get_process(pid)?;
        let id = p.create(loc, vol, dim)?;
        Ok(id)
    }

    fn input(&self, pid: ProcessId, name: String, vol: f64, dim: Location) -> RpcResult<DropletId> {
        debug!(
            "input(pid={}, name={}, vol={}, dim={:?})",
            pid, name, vol, dim
        );
        let p = self.get_process(pid)?;
        let id = p.input(name, vol, dim)?;
        Ok(id)
    }

    fn output(&self, pid: ProcessId, name: String, d: DropletId) -> RpcResult<()> {
        debug!("output(pid={}, name={}, d={:?})", pid, name, d);
        let p = self.get_process(pid)?;
        p.output(name, d)?;
        Ok(())
    }

    fn move_droplet(&self, pid: ProcessId, d: DropletId, loc: Location) -> RpcResult<DropletId> {
        debug!("move_droplet(pid={}, d={:?}, loc={:?})", pid, d, loc);
        let p = self.get_process(pid)?;
        let id = p.move_droplet(d, loc)?;
        Ok(id)
    }

    fn mix(&self, pid: ProcessId, d1: DropletId, d2: DropletId) -> RpcResult<DropletId> {
        debug!("mix(pid={}, d1={:?}, d2={:?})", pid, d1, d2);
        let p = self.get_process(pid)?;
        let id = p.mix(d1, d2)?;
        Ok(id)
    }

    fn combine_into(&self, pid: ProcessId, d1: DropletId, d2: DropletId) -> RpcResult<DropletId> {
        debug!("combine_into(pid={}, d1={:?}, d2={:?})", pid, d1, d2);
        let p = self.get_process(pid)?;
        let id = p.combine_into(d1, d2)?;
        Ok(id)
    }

    fn split(&self, pid: ProcessId, d: DropletId) -> RpcResult<(DropletId, DropletId)> {
        debug!("split(pid={}, d={:?})", pid, d);
        let p = self.get_process(pid)?;
        let id = p.split(d)?;
        Ok(id)
    }

    fn heat(
        &self,
        pid: ProcessId,
        d: DropletId,
        temperature: f32,
        seconds: f64,
    ) -> RpcResult<DropletId> {
        debug!(
            "heat(pid={}, d={:?}, temp={}, seconds={})",
            pid, d, temperature, seconds
        );
        let p = self.get_process(pid)?;
        let id = p.heat(d, temperature, seconds)?;
        Ok(id)
    }
}
