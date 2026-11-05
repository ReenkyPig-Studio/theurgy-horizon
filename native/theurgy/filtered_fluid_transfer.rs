// SPDX-License-Identifier: MIT
use super::*;

impl Game<'_> {
    /// Mod-fluid counterpart of native_fluid_insert. Vanilla water/lava retain
    /// the host implementation; mod fluids use the declared ModFluidTanks and
    /// their saved NBT without silently discarding non-vanilla ids.
    fn theurgy_fluid_insert(&mut self, pos: [i32; 3], face: Direction, fluid: &str, amount: i32) -> i32 {
        if fluid == "minecraft:water" || fluid == "minecraft:lava" {
            return self.native_fluid_insert(pos, face, fluid, amount);
        }
        if !self.native_simulates() { return amount; }
        let Some(FluidAt::Mod(tanks)) = self.native_fluid_at(self.dim.id(), pos) else { return amount; };
        let key = (pos[0], pos[1], pos[2]);
        let n = tanks.tanks.len();
        let reached = tanks.faces[face as usize].clone();
        let mut state = {
            let Some(horizon_client::containers::BlockEntity::Mod(m)) = self.world.block_entities.get(&key) else { return amount; };
            read_tanks(&m.nbt, n)
        };
        let mut left = amount.max(0);
        for pass in 0..2 {
            for &tank in &reached {
                if left == 0 || !tanks.inputs.contains(&tank) || tank >= n { continue; }
                let entry = &mut state[tank];
                let fits = match pass { 0 => entry.0.as_deref() == Some(fluid), _ => entry.0.is_none() };
                if !fits { continue; }
                let room = tanks.tanks[tank] - entry.1;
                if room <= 0 { continue; }
                let add = room.min(left);
                entry.1 += add;
                left -= add;
                entry.0 = Some(fluid.to_owned());
            }
        }
        if left < amount {
            let Some(horizon_client::containers::BlockEntity::Mod(m)) = self.world.block_entities.get_mut(&key) else { return amount; };
            write_tanks(&mut m.nbt, &state);
            self.world.modified.insert((key.0 >> 4, key.2 >> 4));
        }
        left
    }

    /// Atomic filtered move, including mod fluids. A failed predicate occurs
    /// before extraction; rejected insertion returns the exact source receipt.
    pub(in crate::game) fn native_move_fluids_filtered(
        &mut self,
        dimension: &str,
        from: [i32; 3],
        from_face: Direction,
        to: [i32; 3],
        to_face: Direction,
        accept: &dyn Fn(&str) -> bool,
        max: i32,
    ) -> Option<(String, i32)> {
        if from == to { return None; }
        let fluid = self.native_fluid_tanks(dimension, from, from_face)?
            .into_iter().find(|tank| tank.can_drain && tank.amount > 0)?.fluid?;
        if !accept(&fluid) { return None; }
        let max = max.min(self.fluid_room(dimension, from, from_face, to, to_face)?);
        let (fluid, taken, receipt) = self.fluid_take(dimension, from, from_face, max)?;
        let left = self.theurgy_fluid_insert(to, to_face, &fluid, taken);
        if left > 0 {
            let all_back = if receipt.is_empty() {
                self.native_fluid_insert(from, from_face, &fluid, left) == 0
            } else {
                self.fluid_return(dimension, from, &fluid, left, &receipt)
            };
            if !all_back { log::warn!("Theurgy: filtered fluid move left {left} mB of {fluid} nowhere to go"); }
        }
        Some((fluid, taken - left))
    }
}
