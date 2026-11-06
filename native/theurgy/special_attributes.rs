// SPDX-License-Identifier: MIT
use super::*;
use horizon_world::nbt::{Compound, JavaString, Tag};

#[derive(Clone, Debug)]
pub(in crate::game) enum NativeSpecialRule {
    AddedBy { namespace: String, inverted: bool },
    Enchantment { id: String, inverted: bool },
    FluidContents { id: String, inverted: bool },
    ItemName { value: String, inverted: bool },
}

impl NativeSpecialRule {
    pub(in crate::game) fn allows_item(&self, stack: &ItemStack) -> bool {
        match self {
            Self::AddedBy { namespace, inverted } => {
                (stack.item.split(':').next().unwrap_or("minecraft") == namespace) ^ *inverted
            }
            Self::Enchantment { id, inverted } => {
                stack.enchantments().iter().any(|(enchantment, _)| enchantment.as_ref() == id) ^ *inverted
            }
            Self::FluidContents { id, inverted } => (fluid_id(stack).as_deref() == Some(id.as_str())) ^ *inverted,
            Self::ItemName { value, inverted } => {
                let matches = match stack.data().and_then(|data| data.other("minecraft:item_name")) {
                    Some(Tag::String(name)) => name.to_string_lossy() == *value,
                    _ => value.is_empty(),
                };
                matches ^ *inverted
            }
        }
    }
}

impl Game<'_> {
    pub(in crate::game) fn native_special_rule(
        &mut self,
        kind: &str,
        payload: &Compound,
        inverted: bool,
    ) -> Option<NativeSpecialRule> {
        let _ = self;
        match kind {
            "codedefinedgui:added_by" => payload_string(payload, "id")
                .map(|namespace| NativeSpecialRule::AddedBy { namespace, inverted }),
            "codedefinedgui:enchantment" => payload_string(payload, "id")
                .map(|id| NativeSpecialRule::Enchantment { id, inverted }),
            "codedefinedgui:fluid_contents" => payload_string(payload, "id")
                .map(|id| NativeSpecialRule::FluidContents { id, inverted }),
            "codedefinedgui:item_name" => payload_string(payload, "value")
                .map(|value| NativeSpecialRule::ItemName { value, inverted }),
            _ => None,
        }
    }
}

fn payload_string(payload: &Compound, key: &str) -> Option<String> {
    match payload.get(&JavaString::from(key)) {
        Some(Tag::String(value)) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn fluid_id(stack: &ItemStack) -> Option<String> {
    match stack.item.as_ref() {
        "minecraft:water_bucket" => return Some("minecraft:water".to_owned()),
        "minecraft:lava_bucket" => return Some("minecraft:lava".to_owned()),
        "minecraft:bucket" => return None,
        _ => {}
    }
    let Some(Tag::Compound(custom)) = stack.data().and_then(|data| data.other("minecraft:custom_data")) else { return None; };
    let Some(Tag::Compound(fluid)) = custom.get(&JavaString::from("horizon_fluid")) else { return None; };
    let amount = match fluid.get(&JavaString::from("Amount")) {
        Some(Tag::Int(amount)) => *amount,
        _ => 0,
    };
    if amount <= 0 { return None; }
    match fluid.get(&JavaString::from("Fluid")) {
        Some(Tag::String(id)) => Some(id.to_string_lossy()),
        _ => None,
    }
}
