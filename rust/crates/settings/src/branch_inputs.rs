//! `branches.inputs`: what the `${input:<id>}` of `branches.format` ask for (`branch.format-name`).

use json::Value;
use model::branch::{BranchInput, InputKind};

use crate::ProblemReason;

/// The inputs of a list of objects `{ "id", "type": "promptString" | "pickString", "description"?, "default"?,
/// "pattern"?, "options"?, "allowCustom"? }`; an entry with no id, or of another type, is left out.
///
/// # Errors
///
/// When the value is not a list.
pub(crate) fn branch_inputs(value: &Value) -> Result<Vec<BranchInput>, ProblemReason> {
    let entries = value
        .as_array()
        .ok_or(ProblemReason::WrongType { expected: "a list" })?;
    Ok(entries.iter().filter_map(branch_input).collect())
}

/// One input, when it has an id and a type of the two.
fn branch_input(entry: &Value) -> Option<BranchInput> {
    let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
    let kind = match entry.get("type").and_then(Value::as_str)? {
        "promptString" => InputKind::PromptString,
        "pickString" => InputKind::PickString,
        _ => return None,
    };
    let options = entry
        .get("options")
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    Some(BranchInput {
        id: text("id")?,
        kind,
        description: text("description"),
        default: text("default"),
        pattern: text("pattern"),
        options,
        allow_custom: entry.get("allowCustom").and_then(Value::as_bool).unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use model::branch::InputKind;

    use super::branch_inputs;

    #[test]
    fn reads_each_input_with_an_id_and_a_type_of_the_two() {
        let value = json::parse(
            r#"[
                { "id": "ticket", "type": "promptString", "description": "Ticket", "pattern": "^\\d+$" },
                { "id": "kind", "type": "pickString", "options": ["feat", "fix"], "allowCustom": true },
                { "type": "promptString" },
                { "id": "other", "type": "command" }
            ]"#,
        )
        .unwrap();
        let inputs = branch_inputs(&value).unwrap();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id, "ticket");
        assert_eq!(inputs[0].pattern.as_deref(), Some(r"^\d+$"));
        assert_eq!(inputs[1].kind, InputKind::PickString);
        assert_eq!(inputs[1].options, ["feat", "fix"]);
        assert!(inputs[1].allow_custom);
        assert!(branch_inputs(&json::parse("{}").unwrap()).is_err());
    }
}
