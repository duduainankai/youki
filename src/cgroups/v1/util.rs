use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Result};
use procfs::process::Process;

use super::controller_type::CONTROLLERS;

pub fn list_subsystem_mount_points() -> Result<HashMap<String, PathBuf>> {
    let mut mount_paths = HashMap::with_capacity(CONTROLLERS.len());

    for controller in CONTROLLERS {
        if let Ok(mount_point) = get_subsystem_mount_points(&controller.to_string()) {
            mount_paths.insert(controller.to_string(), mount_point);
        }
    }

    Ok(mount_paths)
}

pub fn get_subsystem_mount_points(subsystem: &str) -> Result<PathBuf> {
    Process::myself()?
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
            m.mount_point.ends_with(&subsystem)
        })
        .map(|m| m.mount_point)
        .ok_or_else(|| anyhow!("could not find mountpoint for {}", subsystem))
}