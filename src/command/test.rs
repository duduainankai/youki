use std::{any::Any, cell::RefCell};

use nix::sched::CloneFlags;

use super::Command;

#[derive(Clone)]
pub struct TestHelperCommand {
    set_ns_args: RefCell<Vec<(i32, CloneFlags)>>,
    unshare_args: RefCell<Vec<CloneFlags>>,
}

impl Default for TestHelperCommand {
    fn default() -> Self {
        TestHelperCommand {
            set_ns_args: RefCell::new(vec![]),
            unshare_args: RefCell::new(vec![]),
        }
    }
}

impl Command for TestHelperCommand {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn pivot_rootfs(&self, _path: &std::path::Path) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn set_ns(&self, rawfd: i32, nstype: CloneFlags) -> anyhow::Result<()> {
        let args = (rawfd, nstype);
        self.set_ns_args.borrow_mut().push(args);
        Ok(())
    }

    fn set_id(&self, _uid: nix::unistd::Uid, _gid: nix::unistd::Gid) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn unshare(&self, flags: CloneFlags) -> anyhow::Result<()> {
        self.unshare_args.borrow_mut().push(flags);
        Ok(())
    }
}

impl TestHelperCommand {
    pub fn get_setns_args(&self) -> Vec<(i32, CloneFlags)> {
        self.set_ns_args.borrow_mut().clone()
    }

    pub fn get_unshare_args(&self) -> Vec<CloneFlags> {
        self.unshare_args.borrow_mut().clone()
    }
}