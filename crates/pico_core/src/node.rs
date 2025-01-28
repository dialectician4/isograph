use std::fmt;
use std::hash::Hash;

use crate::database::Database;
use crate::dyn_eq::DynEq;
use crate::epoch::Epoch;
use crate::key::Key;
use crate::params::ParamId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DerivedNodeId {
    pub key: Key,
    pub param_id: ParamId,
}

impl DerivedNodeId {
    pub fn new(key: Key, param_id: ParamId) -> Self {
        Self { key, param_id }
    }
}

pub struct DerivedNode<Db: Database + ?Sized> {
    pub time_verified: Epoch,
    pub time_updated: Epoch,
    pub dependencies: Vec<Dependency>,
    pub inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
    pub value: Box<dyn DynEq>,
}

impl<Db: Database> fmt::Debug for DerivedNode<Db> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DerivedNode")
            .field("time_verified", &self.time_verified)
            .field("time_updated", &self.time_updated)
            .field("dependencies", &self.dependencies)
            .field("value", &self.value)
            .finish()
    }
}

impl <Db: Database + ?Sized> for DerivedNode<Db> {
    pub fn update(&mut self, other: DerivedNode<Db>) -> bool {
        self.dependencies = other.dependencies;
        self.time_verified = other.time_verified;
        if self.value != other.value {
            self.value = other.value;
            self.time_updated = other.time_updated;
            true
        } else {
            false
        }
    }
}
#[derive(Debug)]
pub struct SourceNode {
    pub time_updated: Epoch,
    pub value: Box<dyn DynEq>,
}

#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    pub node_to: NodeKind,
    pub time_verified_or_updated: Epoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Source(Key),
    Derived(DerivedNodeId),
}
