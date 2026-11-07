// SPDX-License-Identifier: MIT
use super::*;
use horizon_world::nbt::{Compound, JavaString, Tag};
use std::collections::HashSet;
use std::sync::Arc;

pub(in crate::game) struct NativeListFilter {
    pub deny: bool,
    pub respect: bool,
    pub entries: Vec<ItemStack>,
}

impl NativeListFilter {
    pub(in crate::game) fn allows_item(&self, stack: &ItemStack) -> bool {
        if self.entries.is_empty() {
            return true;
        }

        let matched = self.entries.iter().any(|entry| {
            entry.item == stack.item
                && (!self.respect
                    || (entry.damage == stack.damage && entry.extra == stack.extra))
        });

        if self.deny { !matched } else { matched }
    }

    pub(in crate::game) fn allows_fluid(&self, game: &Game<'_>, fluid: &str) -> bool {
        if self.entries.is_empty() {
            return true;
        }

        let mut matched = false;
        for entry in &self.entries {
            if let Some(found) = game.native_item_fluid(entry) {
                if found.fluid.as_deref() == Some(fluid) {
                    matched = true;
                    break;
                }
            }
        }

        if self.deny { !matched } else { matched }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::game) enum NativeAttributeMode {
    MatchAny,
    MatchAll,
    Deny,
}

pub(in crate::game) enum NativeAttributeRule {
    InTag {
        members: HashSet<Arc<str>>,
        inverted: bool,
    },
    StandardRenamed {
        inverted: bool,
    },
    Standard(super::theurgy_standard_attributes::NativeStandardRule),
    Special(super::theurgy_special_attributes::NativeSpecialRule),
    Unknown,
}

impl NativeAttributeRule {
    fn matches(&self, stack: &ItemStack) -> Option<bool> {
        match self {
            NativeAttributeRule::InTag { members, inverted } => {
                Some(members.contains(&stack.item) ^ *inverted)
            }
            NativeAttributeRule::StandardRenamed { inverted } => {
                let renamed = stack
                    .data()
                    .and_then(|d| d.other("minecraft:custom_name"))
                    .is_some();
                Some(renamed ^ *inverted)
            }
            NativeAttributeRule::Standard(rule) => Some(rule.allows_item(stack)),
            NativeAttributeRule::Special(rule) => Some(rule.allows_item(stack)),
            NativeAttributeRule::Unknown => None,
        }
    }
}

pub(in crate::game) struct NativeAttributeFilter {
    mode: NativeAttributeMode,
    rules: Vec<NativeAttributeRule>,
}

impl NativeAttributeFilter {
    pub(in crate::game) fn is_valid(&self) -> bool {
        !self.rules.iter().any(|rule| matches!(rule, NativeAttributeRule::Unknown))
    }

    pub(in crate::game) fn allows_item(&self, stack: &ItemStack) -> bool {
        if self.rules.is_empty() {
            return true;
        }

        let mut any = false;
        let mut all = true;
        for rule in &self.rules {
            let matched = rule.matches(stack).unwrap_or(false);
            any |= matched;
            all &= matched;
        }

        match self.mode {
            NativeAttributeMode::MatchAny => any,
            NativeAttributeMode::MatchAll => all,
            NativeAttributeMode::Deny => !any,
        }
    }
}

fn native_attribute_tag_bool(value: Option<&Tag>) -> bool {
    match value {
        Some(Tag::Byte(value)) => *value != 0,
        Some(Tag::Int(value)) => *value != 0,
        _ => false,
    }
}

impl Game<'_> {
    /// native_move_items with Java's extractor/inserter filter predicate. An
    /// item is checked before take_one; rejected or uninsertable stacks stay
    /// authoritative in their source slot. The rest of the transaction is the
    /// same bounded slot walk and rollback as native_move_items.
    pub(in crate::game) fn native_move_items_filtered(
        &mut self,
        dimension: &str,
        from: [i32; 3],
        from_face: Direction,
        to: [i32; 3],
        to_face: Direction,
        accept: &dyn Fn(&ItemStack) -> bool,
        max: u32,
    ) -> u32 {
        let Some(source) = self.native_item_source(dimension, from) else { return 0; };
        let Some(target) = self.containers().view((to[0], to[1], to[2]), false) else { return 0; };
        if source.parts.iter().any(|p| target.parts.contains(p)) { return 0; }
        let max = if matches!(target.kind, Kind::Composter(_)) { max.min(1) } else { max };
        let into = target.slots_to_add(Some(to_face));
        let mut moved = 0;
        for slot in source.slots_for_face(Some(from_face)) {
            while moved < max {
                let c = self.containers();
                let Some(stack) = c.get(&source, slot) else { break; };
                if !accept(stack) || !c.can_take(&source, slot, stack, from_face) { break; }
                let Some(one) = self.take_one(&source, slot) else { break; };
                if let Some(rest) = self.add_one_quietly(&target, &into, one, to_face) {
                    self.put_back(&source, slot, rest);
                    break;
                }
                moved += 1;
            }
        }
        if moved > 0 {
            self.view_changed(&source);
            self.view_changed(&target);
        }
        moved
    }

    pub(in crate::game) fn native_list_filter(&self, filter: Option<&ItemStack>) -> NativeListFilter {
        let mut result = NativeListFilter {
            deny: false,
            respect: false,
            entries: Vec::new(),
        };

        let Some(filter) = filter else { return result; };

        let contents = self
            .native_component(filter, "codedefinedgui:list_filter_contents")
            .ok()
            .flatten();
        let config = self
            .native_component(filter, "codedefinedgui:list_filter_config")
            .ok()
            .flatten();

        if let Some(Tag::Compound(config)) = config.as_ref() {
            let config: &Compound = config;

            if let Some(Tag::String(mode)) = config.get(&JavaString::from("mode")) {
                result.deny = mode.to_string_lossy() == "deny";
            }

            result.respect = match config.get(&JavaString::from("respect_data_components")) {
                Some(Tag::Byte(value)) => *value != 0,
                Some(Tag::Int(value)) => *value != 0,
                _ => false,
            };
        }

        let slots: Option<&[Tag]> = match contents.as_ref() {
            Some(Tag::List(list)) => Some(&list[..]),
            Some(Tag::Compound(map)) => match map.get(&JavaString::from("slots")) {
                Some(Tag::List(list)) => Some(&list[..]),
                _ => None,
            },
            _ => None,
        };

        let Some(slots) = slots else { return result; };

        for slot in slots {
            let Tag::Compound(slot_map) = slot else { continue; };
            let item_tag = match slot_map.get(&JavaString::from("item")) {
                Some(item) => item,
                None => slot,
            };
            let Tag::Compound(item_map) = item_tag else { continue; };
            let Some(Tag::String(id)) = item_map.get(&JavaString::from("id")) else { continue; };

            let mut stack = ItemStack::new(id.to_string_lossy().as_str(), 1);

            if let Some(Tag::Compound(components)) = item_map.get(&JavaString::from("components")) {
                for (key, value) in components {
                    stack
                        .data_mut()
                        .set_other(key.to_string_lossy().as_str(), Some(value.clone()));
                }
            }

            result.entries.push(stack);
        }

        result
    }

    pub(in crate::game) fn native_attribute_filter(
        &mut self,
        filter: Option<&ItemStack>,
    ) -> NativeAttributeFilter {
        let mut result = NativeAttributeFilter {
            mode: NativeAttributeMode::MatchAny,
            rules: Vec::new(),
        };

        let Some(filter) = filter else { return result; };

        let config = self
            .native_component(filter, "codedefinedgui:attribute_filter_config")
            .ok()
            .flatten()
            .cloned();

        let Some(Tag::Compound(config)) = config.as_ref() else { return result; };
        let config: &Compound = config;

        if let Some(Tag::String(mode)) = config.get(&JavaString::from("mode")) {
            let mode = mode.to_string_lossy();
            if mode == "match_all" {
                result.mode = NativeAttributeMode::MatchAll;
            } else if mode == "deny" {
                result.mode = NativeAttributeMode::Deny;
            } else {
                result.mode = NativeAttributeMode::MatchAny;
            }
        }

        let Some(Tag::List(rules)) = config.get(&JavaString::from("rules")) else {
            return result;
        };

        for rule in &rules[..] {
            let Tag::Compound(rule_map) = rule else {
                result.rules.push(NativeAttributeRule::Unknown);
                continue;
            };

            let inverted = native_attribute_tag_bool(rule_map.get(&JavaString::from("inverted")));

            let Some(Tag::String(kind)) = rule_map.get(&JavaString::from("type")) else {
                result.rules.push(NativeAttributeRule::Unknown);
                continue;
            };
            let kind = kind.to_string_lossy();

            if kind == "codedefinedgui:in_tag" {
                let Some(Tag::Compound(payload)) = rule_map.get(&JavaString::from("payload")) else {
                    result.rules.push(NativeAttributeRule::Unknown);
                    continue;
                };
                let Some(Tag::String(id)) = payload.get(&JavaString::from("id")) else {
                    result.rules.push(NativeAttributeRule::Unknown);
                    continue;
                };

                let id = id.to_string_lossy();
                let members: HashSet<Arc<str>> = self
                    .res
                    .items()
                    .item_tags
                    .get(id.as_str())
                    .map(|values| {
                        values
                            .iter()
                            .map(|item| {
                                let item: &str = item.as_ref();
                                Arc::<str>::from(item)
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                result.rules.push(NativeAttributeRule::InTag { members, inverted });
            } else if kind == "codedefinedgui:standard" {
                let Some(Tag::Compound(payload)) = rule_map.get(&JavaString::from("payload")) else {
                    result.rules.push(NativeAttributeRule::Unknown);
                    continue;
                };

                let Some(Tag::String(value)) = payload.get(&JavaString::from("value")) else {
                    result.rules.push(NativeAttributeRule::Unknown);
                    continue;
                };
                match self.native_standard_rule(&value.to_string_lossy(), inverted) {
                    Some(rule) => result.rules.push(NativeAttributeRule::Standard(rule)),
                    None => result.rules.push(NativeAttributeRule::Unknown),
                }
            } else if let Some(Tag::Compound(payload)) = rule_map.get(&JavaString::from("payload")) {
                match self.native_special_rule(kind.as_str(), payload, inverted) {
                    Some(rule) => result.rules.push(NativeAttributeRule::Special(rule)),
                    None => result.rules.push(NativeAttributeRule::Unknown),
                }
            } else {
                result.rules.push(NativeAttributeRule::Unknown);
            }
        }

        result
    }
}
