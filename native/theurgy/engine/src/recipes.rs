// SPDX-License-Identifier: MIT
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub count: u32,
    pub max_stack: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Tank {
    pub fluid: Option<String>,
    pub amount: i32,
    pub capacity: i32,
}

#[derive(Clone, Debug)]
pub struct Recipe<'a> {
    pub id: &'a str,
    pub json: &'a Value,
    pub time: i32,
}

pub fn sized_ingredient(value: &Value) -> (&Value, u32) {
    if let Some(inner) = value.get("ingredient") {
        (inner, value.get("count").and_then(Value::as_u64).unwrap_or(1).min(u64::from(u32::MAX)) as u32)
    } else {
        (value, 1)
    }
}

pub fn ingredient_parts<'a>(kind: &str, json: &'a Value) -> Vec<(&'a Value, u32)> {
    match kind {
        "theurgy:calcination" | "theurgy:distillation" => {
            json.get("ingredient").map(sized_ingredient).into_iter().collect()
        }
        "theurgy:liquefaction" => json.get("ingredient").map(sized_ingredient).into_iter().collect(),
        "theurgy:fermentation" | "theurgy:digestion" => json
            .get("ingredients").and_then(Value::as_array).into_iter().flatten()
            .map(sized_ingredient).collect(),
        _ => Vec::new(),
    }
}

pub fn fluid_part<'a>(kind: &str, json: &'a Value) -> Option<(&'a str, i32)> {
    let field = match kind {
        "theurgy:liquefaction" => "solvent",
        "theurgy:fermentation" | "theurgy:digestion" => "fluid",
        _ => return None,
    };
    let value = json.get(field)?;
    let id = value.get("ingredient")?.as_str()?;
    Some((id, value.get("amount")?.as_i64()?.clamp(0, i64::from(i32::MAX)) as i32))
}

/// Finds a recipe from the current server table. Ingredient tags/components are
/// resolved by the host callback; all count and fluid checks remain atomic here.
pub fn find_recipe<'a, F>(
    kind: &str,
    recipes: &'a [Recipe<'a>],
    items: &[Option<Item>],
    tank: Option<&Tank>,
    mut matches: F,
) -> Option<&'a Recipe<'a>>
where F: FnMut(&Value, &Item) -> bool {
    recipes.iter().find(|recipe| {
        let parts = ingredient_parts(kind, recipe.json);
        if parts.is_empty() || parts.iter().any(|(ingredient, count)| {
            !items.iter().flatten().any(|item| item.count >= *count && matches(ingredient, item))
        }) {
            return false;
        }
        if let Some((fluid, amount)) = fluid_part(kind, recipe.json) {
            return tank.is_some_and(|t| t.amount >= amount && t.fluid.as_deref() == Some(fluid));
        }
        true
    })
}

pub fn can_fit(output: Option<&Item>, result: &Item) -> bool {
    match output {
        None => result.count <= result.max_stack,
        Some(old) => old.id == result.id && u64::from(old.count) + u64::from(result.count) <= u64::from(old.max_stack.min(result.max_stack)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn item(id: &str, count: u32) -> Item {
        Item { id: id.into(), count, max_stack: 64 }
    }
    #[test]
    fn sized_count_is_required_not_just_item_type() {
        let json = json!({"ingredient":{"ingredient":"minecraft:beef","count":2}});
        let recipes = [Recipe { id: "beef", json: &json, time: 100 }];
        assert!(find_recipe("theurgy:distillation", &recipes,
            &[Some(item("minecraft:beef", 1))], None, |_, _| true).is_none());
        assert!(find_recipe("theurgy:distillation", &recipes,
            &[Some(item("minecraft:beef", 2))], None, |_, _| true).is_some());
    }
    #[test]
    fn solvent_identity_and_amount_are_both_required() {
        let json = json!({"ingredient":"minecraft:oak_log",
            "solvent":{"ingredient":"theurgy:sal_ammoniac","amount":10}});
        let recipes = [Recipe { id: "x", json: &json, time: 100 }];
        let items = [Some(item("minecraft:oak_log", 1))];
        assert!(find_recipe("theurgy:liquefaction", &recipes, &items,
            Some(&Tank { fluid: Some("minecraft:water".into()), amount: 1000, capacity: 10000 }), |_, _| true).is_none());
        assert!(find_recipe("theurgy:liquefaction", &recipes, &items,
            Some(&Tank { fluid: Some("theurgy:sal_ammoniac".into()), amount: 9, capacity: 10000 }), |_, _| true).is_none());
    }
    #[test]
    fn full_or_wrong_output_never_overflows() {
        assert!(can_fit(None, &item("x", 64)));
        assert!(!can_fit(Some(&item("y", 1)), &item("x", 1)));
        assert!(!can_fit(Some(&item("x", 64)), &item("x", 1)));
        assert!(can_fit(Some(&item("x", 63)), &item("x", 1)));
    }
}
