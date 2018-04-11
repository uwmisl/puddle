use std::sync::Arc;
use jsonrpc_core as rpc;

use ::*;

impl From<PuddleError> for rpc::Error {
    fn from(p_err: PuddleError) -> Self {
        let code = rpc::ErrorCode::ServerError(0);
        let mut err = rpc::Error::new(code);
        err.message = format!("PuddleError: {:?}", p_err);
        err
    }
}

build_rpc_trait! {
    pub trait Rpc {

        #[rpc(name = "new_process")]
        fn new_process(
            &self,
            String
        ) -> PuddleResult<ProcessId>;

        #[rpc(name = "close_process")]
        fn close_process(
            &self,
            ProcessId
        ) -> PuddleResult<()>;

        #[rpc(name = "droplet_info")]
        fn droplet_info(
            &self,
            ProcessId
        ) -> PuddleResult<Vec<DropletInfo>>;

        #[rpc(name = "visualizer_droplet_info")]
        fn visualizer_droplet_info(
            &self
        ) -> PuddleResult<Vec<DropletInfo>>;

        #[rpc(name = "flush")]
        fn flush(
            &self,
            ProcessId
        ) -> PuddleResult<()>;

        #[rpc(name = "input")]
        fn input(
            &self,
            ProcessId,
            Option<Location>,
            f64,
            Option<Location>
        ) -> PuddleResult<DropletId>;

        #[rpc(name = "move")]
        fn move_droplet(
            &self,
            ProcessId,
            DropletId,
            Location
        ) -> PuddleResult<DropletId>;

        #[rpc(name = "mix")]
        fn mix(
            &self,
            ProcessId,
            DropletId,
            DropletId
        ) -> PuddleResult<DropletId>;

        #[rpc(name = "split")]
        fn split(
            &self,
            ProcessId,
            DropletId
        ) -> PuddleResult<(DropletId, DropletId)>;
    }
}

impl Rpc for Arc<Manager> {
    //
    // process management commands
    //

    fn new_process(&self, name: String) -> PuddleResult<ProcessId> {
        // can't the function being implemented, use fully qualified name
        Manager::new_process(&self, name)
    }

    fn close_process(&self, pid: ProcessId) -> PuddleResult<()> {
        // can't the function being implemented, use fully qualified name
        Manager::close_process(&self, pid)
    }

    //
    // status commands
    //

    fn droplet_info(&self, pid: ProcessId) -> PuddleResult<Vec<DropletInfo>> {
        let p = self.get_process(pid)?;
        p.flush()
    }

    fn visualizer_droplet_info(&self) -> PuddleResult<Vec<DropletInfo>> {
        // can't the function being implemented, use fully qualified name
        Manager::visualizer_droplet_info(&self)
    }

    //
    // Droplet manipulation
    // delegate to process
    //

    fn flush(&self, pid: ProcessId) -> PuddleResult<()> {
        let p = self.get_process(pid)?;
        p.flush().map(|_result| ())
    }

    fn input(
        &self,
        pid: ProcessId,
        loc: Option<Location>,
        vol: f64,
        dim: Option<Location>,
    ) -> PuddleResult<DropletId> {
        let p = self.get_process(pid)?;
        p.input(loc, vol, dim)
    }

    fn move_droplet(&self, pid: ProcessId, d: DropletId, loc: Location) -> PuddleResult<DropletId> {
        let p = self.get_process(pid)?;
        p.move_droplet(d, loc)
    }

    fn mix(&self, pid: ProcessId, d1: DropletId, d2: DropletId) -> PuddleResult<DropletId> {
        let p = self.get_process(pid)?;
        p.mix(d1, d2)
    }

    fn split(&self, pid: ProcessId, d: DropletId) -> PuddleResult<(DropletId, DropletId)> {
        let p = self.get_process(pid)?;
        p.split(d)
    }
}
