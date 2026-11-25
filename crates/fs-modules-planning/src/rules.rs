//! The rights of the planning module, for `fs_rpc::Guarded`.
//!
//! Pattern: Strategy (the `RightsRules` of one data kind, asked by the wrapper before every change).
//! Honest, as in `write.rs`: the core has no rights per person (only the owner reaches the socket), so what a person may do is what
//! the provider supports (`TaskItems::capabilities`); rights per person (groups, several users) come later and replace this
//! Strategy, not the wrapper. A command that is not a command of this module is NOT refused here: no right is asked for what does
//! not exist, and the core answers `unknown-command` 404 (rules 8 and 17 of the suite). `form.read` (the read right that `Guarded`
//! asks for every record of a page and for a blob) is named and allowed on purpose (`form.read -> Allow`): there are no rights per person
//! yet, only the owner reaches the socket, so reading is what it was; it does not rely on the command being unknown.
//! A command of this module that the provider cannot do is `not-possible` (403, `apply` never runs).

use fs_interfaces::Capabilities;
use fs_interfaces::module::{Ask, HiddenReason, RightsRules, Verdict};
use fs_rpc::{READ_COMMAND, REVERT_COMMAND, WATCH_COMMAND};

use crate::write::is_known;

/// The rules of planning: the commands of `catalog/commands.json` the provider supports, and taking back and watching.
pub struct PlanningRules {
    caps: Capabilities,
}

impl PlanningRules {
    /// The rules for a provider that says `caps` (`TaskItems::capabilities`).
    #[must_use]
    pub fn new(caps: Capabilities) -> Self {
        Self { caps }
    }
}

impl RightsRules for PlanningRules {
    fn may(&self, ask: &Ask<'_>) -> Verdict {
        let command = ask.command;
        if command == REVERT_COMMAND || command == WATCH_COMMAND || command == READ_COMMAND {
            return Verdict::Allow;
        }
        if is_known(command) && !self.caps.is_supported(command) {
            return Verdict::deny("not-possible");
        }
        Verdict::Allow
    }

    fn hidden(
        &self,
        _who: &fs_interfaces::module::Context,
        _record: &str,
        _field: &str,
    ) -> Option<HiddenReason> {
        None
    }
}
