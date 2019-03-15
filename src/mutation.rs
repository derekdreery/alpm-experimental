//! This module is for mutating the system. You come up with a plan, and then execute it. Execution
//! happens in a background thread, and you get a channel that receives important events, and one
//! for cancelling. It's important to use the channel to cancel the operation, or else you might
//! end up with an unuseable system!
//!
//! These are Transactions in alpm.

use bitflags::bitflags;

use std::collections::BTreeSet as Set;

use crate::util::PackageKey;

/// This struct holds a plan for a system mutation.
pub struct MutationPlan {
    packages_to_add: Set<PackageKey<'static>>,
    packages_to_remove: Set<PackageKey<'static>>,
    packages_to_upgrade: Set<PackageKey<'static>>,
}
