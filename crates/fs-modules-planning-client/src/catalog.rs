//! The command catalog of the module: capability key (the id of the contract) to contract call and arguments.
//!
//! Pattern: Registry fed by data — the file `catalog/commands.json` of the module repository says which call of the contract a
//! command of the frontend description runs (the command is named by its capability key of the contract) and which arguments it takes; nothing of it is program code. One call per command
//! (a chain of several steps becomes a workflow block later, ADR 0104); `task.create` names `Items.create` and also places the card (`Boards.moveCard`) inside the one command.

use std::collections::BTreeMap;

const BUNDLED: &str = include_str!("../../../catalog/commands.json");

/// One argument of a command: the path the module reads, the kind of value (`text`, `number` ...) and whether it must be given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    /// The name of the argument as `FormSource.invoke` carries it.
    pub path: String,
    /// The kind of the value (the words of the command parameters of the components).
    pub kind: String,
    /// The command is refused without it.
    pub required: bool,
}

/// One command of the catalog, named by its capability key of the contract (`card.move`); the host binds it as `<module>.<key>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The call of the contract, `Interface.method` (`Boards.moveCard`).
    pub call: String,
    /// The arguments the command takes.
    pub params: Vec<Param>,
}

/// The catalog: capability keys to commands.
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
    /// The text is no JSON, an entry lacks `capability` or `call`, has an `id`, or has a broken `params` list.
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
            if entry.get("id").is_some() {
                return Err(
                    "a command with `id`: the host derives the id from `capability`".into(),
                );
            }
            let key = text("capability")?;
            let command = Command {
                call: text("call")?,
                params: params_of(entry, &key)?,
            };
            if commands.insert(key.clone(), command).is_some() {
                return Err(format!("command {key:?} twice"));
            }
        }
        Ok(Self { commands })
    }

    /// The command with this capability key, if the catalog has it.
    #[must_use]
    pub fn command(&self, key: &str) -> Option<&Command> {
        self.commands.get(key)
    }

    /// The capability keys of all commands.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }
}

fn params_of(entry: &serde_json::Value, key: &str) -> Result<Vec<Param>, String> {
    let Some(list) = entry.get("params").and_then(serde_json::Value::as_array) else {
        return Err(format!("command {key:?} without a `params` list"));
    };
    let mut out: Vec<Param> = Vec::new();
    for p in list {
        let text = |name: &str| {
            p.get(name)
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("command {key:?}: a param without `{name}`"))
        };
        let path = text("path")?;
        if out.iter().any(|q| q.path == path) {
            return Err(format!("command {key:?}: param {path:?} twice"));
        }
        let required = match p.get("required") {
            None => false,
            Some(v) => v
                .as_bool()
                .ok_or_else(|| format!("command {key:?}: `required` is no flag"))?,
        };
        out.push(Param {
            path,
            kind: text("type")?,
            required,
        });
    }
    Ok(out)
}
