// SPDX-License-Identifier: MIT
// Server-authoritative list/attribute filter editing used by the native UI.
// Every action revalidates the held item and the complete resulting config.

use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeCommand;
use horizon_client::text::Text;
use horizon_world::nbt::{Compound, JavaString, Tag};

const LIST_CONTENTS: &str = "codedefinedgui:list_filter_contents";
const LIST_CONFIG: &str = "codedefinedgui:list_filter_config";
const ATTRIBUTE_CONFIG: &str = "codedefinedgui:attribute_filter_config";

fn component(game: &crate::game::Game<'_>, stack: &ItemStack, id: &str) -> Option<Tag> {
    game.native_component(stack, id).ok().flatten().cloned()
}

fn json_tag(value: &serde_json::Value) -> Option<Tag> {
    Some(match value {
        serde_json::Value::Null => Tag::End,
        serde_json::Value::Bool(value) => Tag::Byte(i8::from(*value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64().and_then(|value| i32::try_from(value).ok()) {
                Tag::Int(value)
            } else {
                Tag::Double(value.as_f64()?.to_bits())
            }
        }
        serde_json::Value::String(value) => Tag::String(JavaString::from(value.as_str())),
        serde_json::Value::Array(values) => Tag::List(values.iter().map(json_tag).collect::<Option<_>>()?),
        serde_json::Value::Object(values) => Tag::Compound(values.iter()
            .map(|(key, value)| Some((JavaString::from(key.as_str()), json_tag(value)?))).collect::<Option<_>>()?),
    })
}

fn compound(tag: Option<Tag>) -> Compound {
    match tag {
        Some(Tag::Compound(map)) => map,
        _ => Compound::new(),
    }
}

fn list(tag: Option<&Tag>) -> Vec<Tag> {
    match tag {
        Some(Tag::List(values)) => values.clone(),
        _ => Vec::new(),
    }
}

fn feedback(game: &mut crate::game::Game<'_>, message: &str) {
    game.native_feedback(Text::literal(message));
}

fn write_held(game: &mut crate::game::Game<'_>, slot: i32, stack: ItemStack) -> bool {
    let attribute = stack.item.as_ref() == "theurgy:attribute_filter";
    if attribute && !game.native_attribute_filter(Some(&stack)).is_valid() {
        return false;
    }
    let Some(held) = game.native_carried_stack(slot) else { return false; };
    *held = stack;
    true
}

fn list_action(game: &mut crate::game::Game<'_>, mut filter: ItemStack, slot: i32, action: &str, rest: &str) -> bool {
    let mut config = compound(component(game, &filter, LIST_CONFIG));
    let mut contents = list(component(game, &filter, LIST_CONTENTS).as_ref());
    let success = match action {
        "mode" if matches!(rest, "allow" | "deny") => {
            config.insert(JavaString::from("mode"), Tag::String(JavaString::from(rest)));
            true
        }
        "components" if matches!(rest, "true" | "false") => {
            config.insert(JavaString::from("respect_data_components"), Tag::Byte(i8::from(rest == "true")));
            true
        }
        "clear" => {
            contents.clear();
            true
        }
        "add" => {
            if contents.len() >= 27 {
                return false;
            }
            let source_slot = if rest == "auto" {
                (0..36).find(|index| {
                    game.native_carried_stack(*index).is_some_and(|stack| {
                        stack.count > 0 && !matches!(stack.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter")
                    })
                }).unwrap_or(slot)
            } else {
                rest.parse::<i32>().unwrap_or(-1)
            };
            let Some(source) = game.native_carried_stack(source_slot).cloned() else { return false; };
            if source.count == 0 || matches!(source.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter") {
                return false;
            }
            let mut ghost = source;
            ghost.count = 1;
            let Tag::Compound(record) = crate::game::theurgy_logistics::stack_tag(&ghost) else { return false; };
            contents.push(Tag::Compound(record));
            true
        }
        "remove" => {
            let Ok(index) = rest.parse::<usize>() else { return false; };
            index < contents.len() && { contents.remove(index); true }
        }
        _ => false,
    };
    if !success {
        return false;
    }
    filter.data_mut().set_other(LIST_CONFIG, Some(Tag::Compound(config)));
    filter.data_mut().set_other(LIST_CONTENTS, Some(Tag::List(contents)));
    write_held(game, slot, filter)
}

fn attribute_action(game: &mut crate::game::Game<'_>, mut filter: ItemStack, slot: i32, action: &str, rest: &str) -> bool {
    let mut config = compound(component(game, &filter, ATTRIBUTE_CONFIG));
    let mut rules = list(config.get(&JavaString::from("rules")));
    let success = match action {
        "mode" if matches!(rest, "match_any" | "match_all" | "deny") => {
            config.insert(JavaString::from("mode"), Tag::String(JavaString::from(rest)));
            true
        }
        "clear" => {
            rules.clear();
            true
        }
        "rule_renamed" => {
            let mut payload = Compound::new();
            payload.insert(JavaString::from("value"), Tag::String(JavaString::from("renamed")));
            let mut rule = Compound::new();
            rule.insert(JavaString::from("type"), Tag::String(JavaString::from("codedefinedgui:standard")));
            rule.insert(JavaString::from("payload"), Tag::Compound(payload));
            rule.insert(JavaString::from("inverted"), Tag::Byte(0));
            rules.push(Tag::Compound(rule));
            true
        }
        "rule_tag" => {
            let tag = if rest == "auto" {
                let mut selected = None;
                for index in 0..36 {
                    let Some(stack) = game.native_carried_stack(index).cloned() else { continue; };
                    if stack.count == 0 || matches!(stack.item.as_ref(), "theurgy:list_filter" | "theurgy:attribute_filter") {
                        continue;
                    }
                    let mut tags: Vec<_> = game.res.items().item_tags.iter()
                        .filter(|(_, members)| members.contains(stack.item.as_ref()))
                        .map(|(tag, _)| tag.clone()).collect();
                    tags.sort();
                    if let Some(first) = tags.into_iter().next() {
                        selected = Some(first);
                        break;
                    }
                }
                selected
            } else {
                Some(rest.to_owned())
            };
            let Some(tag) = tag.filter(|tag| game.res.items().item_tags.contains_key(tag)) else { return false; };
            let mut payload = Compound::new();
            payload.insert(JavaString::from("id"), Tag::String(JavaString::from(tag.as_str())));
            let mut rule = Compound::new();
            rule.insert(JavaString::from("type"), Tag::String(JavaString::from("codedefinedgui:in_tag")));
            rule.insert(JavaString::from("payload"), Tag::Compound(payload));
            rule.insert(JavaString::from("inverted"), Tag::Byte(0));
            rules.push(Tag::Compound(rule));
            true
        }
        "rule_add" => {
            let Some((kind, json)) = rest.split_once(' ') else { return false; };
            if !matches!(kind, "codedefinedgui:in_tag" | "codedefinedgui:standard") && !kind.starts_with("codedefinedgui:") {
                return false;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else { return false; };
            if !value.is_object() {
                return false;
            }
            let Some(payload) = json_tag(&value) else { return false; };
            let mut rule = Compound::new();
            rule.insert(JavaString::from("type"), Tag::String(JavaString::from(kind)));
            rule.insert(JavaString::from("payload"), Tag::Compound(match payload {
                Tag::Compound(map) => map,
                _ => return false,
            }));
            rule.insert(JavaString::from("inverted"), Tag::Byte(0));
            rules.push(Tag::Compound(rule));
            true
        }
        "rule_invert" => {
            let Ok(index) = rest.parse::<usize>() else { return false; };
            let Some(Tag::Compound(rule)) = rules.get_mut(index) else { return false; };
            let inverted = match rule.get(&JavaString::from("inverted")) {
                Some(Tag::Byte(value)) => *value != 0,
                Some(Tag::Int(value)) => *value != 0,
                _ => false,
            };
            rule.insert(JavaString::from("inverted"), Tag::Byte(i8::from(!inverted)));
            true
        }
        "rule_remove" => {
            let Ok(index) = rest.parse::<usize>() else { return false; };
            index < rules.len() && { rules.remove(index); true }
        }
        _ => false,
    };
    if !success {
        return false;
    }
    config.insert(JavaString::from("rules"), Tag::List(rules));
    filter.data_mut().set_other(ATTRIBUTE_CONFIG, Some(Tag::Compound(config)));
    write_held(game, slot, filter)
}

pub fn register(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("theurgy_filter <hand:word> <action:word> [args:text]"));
}

pub fn perform(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    let Some(arguments) = command.strip_prefix("theurgy_filter ") else { return; };
    *ran = true;
    let mut parts = arguments.splitn(3, ' ');
    let hand = parts.next().unwrap_or("");
    let action = parts.next().unwrap_or("");
    let rest = parts.next().unwrap_or("");
    let slot = match hand {
        "main" => game.selected as i32,
        "off" => -106,
        _ => {
            feedback(game, "theurgy_filter: hand must be main or off");
            return;
        }
    };
    let Some(filter) = game.native_carried_stack(slot).cloned() else {
        feedback(game, "theurgy_filter: the original filter is no longer held");
        return;
    };
    let success = match filter.item.as_ref() {
        "theurgy:list_filter" => list_action(game, filter, slot, action, rest),
        "theurgy:attribute_filter" => attribute_action(game, filter, slot, action, rest),
        _ => false,
    };
    if !success {
        feedback(game, "theurgy_filter: rejected action or invalid resulting configuration");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_payloads_become_bounded_nbt() {
        let value = serde_json::json!({"value":"renamed","nested":[1,true]});
        let tag = json_tag(&value).expect("tag");
        let Tag::Compound(map) = tag else { panic!("compound expected"); };
        assert!(matches!(map.get(&JavaString::from("value")), Some(Tag::String(_))));
        assert!(matches!(map.get(&JavaString::from("nested")), Some(Tag::List(_))));
    }
}
