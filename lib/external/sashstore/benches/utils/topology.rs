//! Allows to query information about the CPU topology.

use std::fmt;

use hwloc::*;

pub type Node = u64;
pub type Socket = u64;
pub type Core = u64;
pub type Cpu = u64;
pub type L1 = u64;
pub type L2 = u64;
pub type L3 = u64;

/// The strategy how threads are allocated in the system.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ThreadMapping {
    /// Don't do any pinning.
    #[allow(unused)]
    None,
    /// Allocate threads on the same socket (as much as possible).
    Sequential,
    /// Spread thread allocation out across sockets (as much as possible).
    #[allow(unused)]
    Interleave,
}

impl fmt::Display for ThreadMapping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ThreadMapping::None => write!(f, "None"),
            ThreadMapping::Sequential => write!(f, "Sequential"),
            ThreadMapping::Interleave => write!(f, "Interleave"),
        }
    }
}

impl fmt::Debug for ThreadMapping {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ThreadMapping::None => write!(f, "TM=None"),
            ThreadMapping::Sequential => write!(f, "TM=Sequential"),
            ThreadMapping::Interleave => write!(f, "TM=Interleave"),
        }
    }
}

/// NUMA Node information.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct NodeInfo {
    /// Node index
    pub node: Node,
    /// Memory in bytes
    pub memory: u64,
}

/// Information about a CPU in the system.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct CpuInfo {
    pub node: Option<NodeInfo>,
    pub socket: Socket,
    pub core: Core,
    pub cpu: Cpu,
    pub l1: L1,
    pub l2: L2,
    pub l3: L3,
}

impl std::fmt::Debug for CpuInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "CpuInfo {{ core/l1/l2: {}/{}/{}, cpu: {}, socket/l3/node: {}/{}/{:?} }}",
            self.core, self.l1, self.l2, self.cpu, self.socket, self.l3, self.node
        )
    }
}

#[derive(Debug)]
pub struct MachineTopology {
    data: Vec<CpuInfo>,
}

impl MachineTopology {
    pub fn new() -> MachineTopology {
        let mut data: Vec<CpuInfo> = Default::default();

        let topo = Topology::new();
        let cpus = topo
            .objects_with_type(&hwloc::ObjectType::PU)
            .expect("Can't find CPUs");

        for cpu in cpus {
            let mut parent = cpu.parent();

            // Find the parent core of the CPU
            while parent.is_some() && parent.unwrap().object_type() != ObjectType::Core {
                parent = parent.unwrap().parent();
            }
            let core = parent.expect("PU has no Core?");

            // Find the parent L1 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 1)
            {
                parent = parent.unwrap().parent();
            }
            let l1 = parent.expect("Core doesn't have a L1 cache?");

            // Find the parent L2 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 2)
            {
                parent = parent.unwrap().parent();
            }
            let l2 = parent.expect("Core doesn't have a L2 cache?");

            // Find the parent socket/L3 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 3)
            {
                parent = parent.unwrap().parent();
            }
            let socket = parent.expect("Core doesn't have a L3 cache (socket)?");

            // Find the parent NUMA node of the CPU
            while parent.is_some() && parent.unwrap().object_type() != ObjectType::NUMANode {
                parent = parent.unwrap().parent();
            }
            let numa_node = parent.map(|n| NodeInfo {
                node: n.os_index() as Node,
                memory: n.memory().total_memory(),
            });

            let cpu_info = CpuInfo {
                node: numa_node,
                socket: socket.logical_index() as Socket,
                core: core.logical_index() as Core,
                cpu: cpu.os_index() as Cpu,
                l1: l1.logical_index() as L1,
                l2: l2.logical_index() as L2,
                l3: socket.logical_index() as L3,
            };

            data.push(cpu_info);
        }

        MachineTopology { data }
    }

    pub fn allocate(&self, strategy: ThreadMapping, how_many: usize, use_ht: bool) -> Vec<CpuInfo> {
        let v = Vec::with_capacity(how_many);
        let mut cpus = self.data.clone();

        if !use_ht {
            cpus.sort_by_key(|c| c.core);
            cpus.dedup_by(|a, b| a.core == b.core);
        }

        match strategy {
            ThreadMapping::None => v,
            ThreadMapping::Interleave => {
                cpus.sort_by_key(|c| c.cpu);
                let c = cpus.iter().take(how_many).map(|c| *c).collect();
                c
            }
            ThreadMapping::Sequential => {
                cpus.sort_by(|a, b| {
                    if a.socket != b.socket {
                        // Allocate from the same socket first
                        a.socket.partial_cmp(&b.socket).unwrap()
                    } else {
                        // But avoid placing on hyper-threads core until all cores are used
                        a.cpu.partial_cmp(&b.cpu).unwrap()
                    }
                });
                let c = cpus.iter().take(how_many).map(|c| *c).collect();
                c
            }
        }
    }
}
