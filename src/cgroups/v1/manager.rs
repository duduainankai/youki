use std::fs;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use nix::unistd::Pid;

use procfs::process::Process;

use super::{
    blkio::Blkio, cpu::Cpu, cpuset::CpuSet, devices::Devices, hugetlb::Hugetlb, memory::Memory,
    network_classifier::NetworkClassifier, network_priority::NetworkPriority, pids::Pids,
    Controller, ControllerType,
};

use crate::cgroups::common::CGROUP_PROCS;
use crate::utils;
use crate::{cgroups::common::CgroupManager, utils::PathBufExt};
use oci_spec::LinuxResources;

const CONTROLLERS: &[ControllerType] = &[
    ControllerType::Cpu,
    ControllerType::CpuSet,
    ControllerType::Devices,
    ControllerType::HugeTlb,
    ControllerType::Memory,
    ControllerType::Pids,
    ControllerType::Blkio,
    ControllerType::NetworkPriority,
    ControllerType::NetworkClassifier,
];

pub struct Manager {
    subsystems: HashMap<String, PathBuf>,
}

impl Manager {
    pub fn new(cgroup_path: PathBuf) -> Result<Self> {
        let mut subsystems = HashMap::<String, PathBuf>::new();
        for subsystem in CONTROLLERS.iter().map(|c| c.to_string()) {
            subsystems.insert(
                subsystem.to_owned(),
                Self::get_subsystem_path(&cgroup_path, &subsystem)?,
            );
        }

        Ok(Manager { subsystems })
    }

    fn get_subsystem_path(cgroup_path: &Path, subsystem: &str) -> anyhow::Result<PathBuf> {
        log::debug!("Get path for subsystem: {}", subsystem);
        let mount = Process::myself()?
            .mountinfo()?
            .into_iter()
            .find(|m| {
                if m.fs_type == "cgroup" {
                    // Some systems mount net_prio and net_cls in the same directory
                    // other systems mount them in their own diretories. This
                    // should handle both cases.
                    if subsystem == "net_cls" {
                        return m.mount_point.ends_with("net_cls,net_prio")
                            || m.mount_point.ends_with("net_prio,net_cls")
                            || m.mount_point.ends_with("net_cls");
                    } else if subsystem == "net_prio" {
                        return m.mount_point.ends_with("net_cls,net_prio")
                            || m.mount_point.ends_with("net_prio,net_cls")
                            || m.mount_point.ends_with("net_prio");
                    }

                    if subsystem == "cpu" {
                        return m.mount_point.ends_with("cpu,cpuacct")
                            || m.mount_point.ends_with("cpu");
                    }
                }
                m.mount_point.ends_with(subsystem)
            })
            .unwrap();

        let cgroup = Process::myself()?
            .cgroups()?
            .into_iter()
            .find(|c| c.controllers.contains(&subsystem.to_owned()))
            .unwrap();

        let p = if cgroup_path.to_string_lossy().into_owned().is_empty() {
            mount
                .mount_point
                .join_absolute_path(Path::new(&cgroup.pathname))?
        } else if cgroup_path.is_absolute() {
            mount.mount_point.join_absolute_path(&cgroup_path)?
        } else {
            mount.mount_point.join(cgroup_path)
        };

        Ok(p)
    }
}

impl CgroupManager for Manager {
    fn apply(&self, linux_resources: &LinuxResources, pid: Pid) -> Result<()> {
        for subsys in &self.subsystems {
            match subsys.0.as_str() {
                "cpu" => Cpu::apply(linux_resources, &subsys.1, pid)?,
                "cpuset" => CpuSet::apply(linux_resources, &subsys.1, pid)?,
                "devices" => Devices::apply(linux_resources, &subsys.1, pid)?,
                "hugetlb" => Hugetlb::apply(linux_resources, &subsys.1, pid)?,
                "memory" => Memory::apply(linux_resources, &subsys.1, pid)?,
                "pids" => Pids::apply(linux_resources, &subsys.1, pid)?,
                "blkio" => Blkio::apply(linux_resources, &subsys.1, pid)?,
                "net_prio" => NetworkPriority::apply(linux_resources, &subsys.1, pid)?,
                "net_cls" => NetworkClassifier::apply(linux_resources, &subsys.1, pid)?,
                _ => unreachable!("every subsystem should have an associated controller"),
            }
        }

        Ok(())
    }

    fn remove(&self) -> Result<()> {
        for cgroup_path in &self.subsystems {
            if cgroup_path.1.exists() {
                log::debug!("remove cgroup {:?}", cgroup_path.1);
                let procs_path = cgroup_path.1.join(CGROUP_PROCS);
                let procs = fs::read_to_string(&procs_path)?;

                for line in procs.lines() {
                    let pid: i32 = line.parse()?;
                    let _ = nix::sys::signal::kill(Pid::from_raw(pid), nix::sys::signal::SIGKILL);
                }

                utils::delete_with_retry(cgroup_path.1)?;
            }
        }

        Ok(())
    }
}
