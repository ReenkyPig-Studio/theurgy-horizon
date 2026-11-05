use super::*;
use crate::game::native_hooks::NativeRecipeKind;
use horizon_client::items::ItemInfo;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(in crate::game) enum NativeStandardRule {
    Placeable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Consumable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    FluidContainer {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Enchanted {
        inverted: bool,
    },
    Renamed {
        inverted: bool,
    },
    Damaged {
        inverted: bool,
    },
    BadlyDamaged {
        max_damage: HashMap<Arc<str>, u32>,
        inverted: bool,
    },
    NotStackable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Equipable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    FurnaceFuel {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Smeltable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Smokable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Blastable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
    Compostable {
        items: HashSet<Arc<str>>,
        inverted: bool,
    },
}

impl NativeStandardRule {
    pub fn allows_item(&self, stack: &ItemStack) -> bool {
        match self {
            Self::Placeable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Consumable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::FluidContainer { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Enchanted { inverted } => (!stack.enchantments().is_empty()) ^ *inverted,
            Self::Renamed { inverted } => stack.custom_name().is_some() ^ *inverted,
            Self::Damaged { inverted } => (stack.damage > 0) ^ *inverted,
            Self::BadlyDamaged {
                max_damage,
                inverted,
            } => {
                let matched = max_damage.get(&stack.item).is_some_and(|&max_damage| {
                    max_damage > 0
                        && (stack.damage as f64 / max_damage as f64) > 0.75
                });
                matched ^ *inverted
            }
            Self::NotStackable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Equipable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::FurnaceFuel { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Smeltable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Smokable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Blastable { items, inverted } => set_match(items, stack) ^ *inverted,
            Self::Compostable { items, inverted } => set_match(items, stack) ^ *inverted,
        }
    }
}

fn set_match(items: &HashSet<Arc<str>>, stack: &ItemStack) -> bool {
    items.contains(&stack.item)
}

fn owned_item_id(info: &ItemInfo) -> Arc<str> {
    info.id.clone()
}

fn collect_recipe_items(game: &mut Game, kind: NativeRecipeKind) -> HashSet<Arc<str>> {
    let wanted = match kind {
        NativeRecipeKind::Smelting => "minecraft:smelting",
        NativeRecipeKind::Smoking => "minecraft:smoking",
        NativeRecipeKind::Blasting => "minecraft:blasting",
        _ => return HashSet::new(),
    };
    let mut items = HashSet::new();
    // Recipe-book displays are not the complete cooking index. Match the
    // authoritative cooking recipes against actual registered stacks instead.
    for recipe in &game.recipes().cooking {
        if recipe.kind != wanted { continue; }
        for id in game.res.items().items.keys() {
            if recipe.ingredient.test(&ItemStack::new(id, 1)) {
                items.insert(id.clone());
            }
        }
    }
    items
}

impl Game<'_> {
    pub(in crate::game) fn native_standard_rule(
        &mut self,
        value: &str,
        inverted: bool,
    ) -> Option<NativeStandardRule> {
        let mut placeable = HashSet::new();
        let mut consumable = HashSet::new();
        let mut badly_damaged = HashMap::new();
        let mut not_stackable = HashSet::new();
        let mut equipable = HashSet::new();
        let mut furnace_fuel = HashSet::new();
        let mut compostable = HashSet::new();
        let mut item_ids = Vec::new();

        for info in self.res.items().items.values() {
            let id = owned_item_id(info);
            item_ids.push(id.clone());

            if info.block.is_some() {
                placeable.insert(id.clone());
            }
            if info.food.is_some() {
                consumable.insert(id.clone());
            }
            if info.max_stack == 1 {
                not_stackable.insert(id.clone());
            }
            if info.equip_slot.is_some() {
                equipable.insert(id.clone());
            }
            if info.fuel.is_some() {
                furnace_fuel.insert(id.clone());
            }
            if info.compostable.is_some() {
                compostable.insert(id.clone());
            }
            if info.max_damage > 0 {
                badly_damaged.insert(id.clone(), info.max_damage);
            }
        }

        let key = value.to_ascii_lowercase();
        match key.as_str() {
            "placeable" => Some(NativeStandardRule::Placeable {
                items: placeable,
                inverted,
            }),
            "consumable" => Some(NativeStandardRule::Consumable {
                items: consumable,
                inverted,
            }),
            "fluid_container" => {
                let mut items = HashSet::new();
                for id in item_ids {
                    let stack = ItemStack::new(&*id, 1);
                    if self.native_item_fluid(&stack).is_some() {
                        items.insert(id);
                    }
                }
                Some(NativeStandardRule::FluidContainer { items, inverted })
            }
            "enchanted" => Some(NativeStandardRule::Enchanted { inverted }),
            "renamed" => Some(NativeStandardRule::Renamed { inverted }),
            "damaged" => Some(NativeStandardRule::Damaged { inverted }),
            "badly_damaged" => Some(NativeStandardRule::BadlyDamaged {
                max_damage: badly_damaged,
                inverted,
            }),
            "not_stackable" => Some(NativeStandardRule::NotStackable {
                items: not_stackable,
                inverted,
            }),
            "equipable" => Some(NativeStandardRule::Equipable {
                items: equipable,
                inverted,
            }),
            "furnace_fuel" => Some(NativeStandardRule::FurnaceFuel {
                items: furnace_fuel,
                inverted,
            }),
            "smeltable" => Some(NativeStandardRule::Smeltable {
                items: collect_recipe_items(self, NativeRecipeKind::Smelting),
                inverted,
            }),
            "smokable" => Some(NativeStandardRule::Smokable {
                items: collect_recipe_items(self, NativeRecipeKind::Smoking),
                inverted,
            }),
            "blastable" => Some(NativeStandardRule::Blastable {
                items: collect_recipe_items(self, NativeRecipeKind::Blasting),
                inverted,
            }),
            "compostable" => Some(NativeStandardRule::Compostable {
                items: compostable,
                inverted,
            }),
            _ => None,
        }
    }
}
