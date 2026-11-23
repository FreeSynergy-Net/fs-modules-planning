//! The command catalog of the module: command id to contract call.
//!
//! Pattern: Registry fed by data — the file `catalog/commands.json` of the module repository says which call of the contract a
//! command of the frontend description runs and which capability it needs; nothing of it is program code. One call per command
//! (a chain of several steps becomes a workflow block later, ADR 0104).

use std::collections::BTreeMap;

const BUNDLED: &str = include_str!("../../../catalog/commands.json");

/// One command of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The call of the contract, `Interface.method` (`Boards.moveCard`).
    pub call: String,
    /// The capability the service must report for the command to be possible.
    pub capability: String,
}

impl Command {
    /// The capability id as the SERVICE reports it (the id of the contract `tasks.capnp`, `card.move`). The file carries it with the
    /// prefix of the module (`planning.card.move`) because the host asks for that there; the contract has no such prefix.
    #[must_use]
    pub fn contract_capability(&self) -> &str {
        let module = crate::ensure::SOCKET_PROGRAM;
        self.capability
            .strip_prefix(module)
            .and_then(|rest| rest.strip_prefix('.'))
            .unwrap_or(&self.capability)
    }
}

/// The catalog: command ids to commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catalog {
    commands: BTreeMap<String, Command>,
}

impl Catalog {
    /// The catalog of this module, as shipped in `catalog/commands.json`.
    ///
    /// # Panics
    /// Never for the shipped file (a test reads it); a broken file is a mistake of the build.
    #[must_use]
    pub fn bundled() -> Self {
        Self::parse(BUNDLED).expect("catalog/commands.json is valid")
    }

    /// Read a catalog from its JSON text.
    ///
    /// # Errors
    /// The text is no JSON, or an entry lacks `id`, `call` or `capability`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let list = value
            .get("commands")
            .and_then(serde_json::Value::as_array)
            .ok_or("no `commands` list")?;
        let mut commands = BTreeMap::new();
        for entry in list {
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| format!("a command without `{key}`"))
            };
            let id = text("id")?;
            let command = Command {
                call: text("call")?,
                capability: text("capability")?,
            };
            if commands.insert(id.clone(), command).is_some() {
                return Err(format!("command {id:?} twice"));
            }
        }
        Ok(Self { commands })
    }

    /// The command with this id, if the catalog has it.
    #[must_use]
    pub fn command(&self, id: &str) -> Option<&Command> {
        self.commands.get(id)
    }

    /// The ids of all commands.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }
}
