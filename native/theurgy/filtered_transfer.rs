// SPDX-License-Identifier: MIT
use super::*;
use horizon_world::nbt::{Compound, JavaString, Tag};

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
}
