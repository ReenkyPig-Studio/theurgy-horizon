use horizon_client::items::ItemStack;
pub fn use_item(game:&mut crate::game::Game<'_>,stack:&ItemStack,claimed:&mut bool){let _=(game,stack,claimed);}
pub fn use_on(game:&mut crate::game::Game<'_>,stack:&ItemStack,pos:[i32;3],face:[i32;3],claimed:&mut bool){
 let _=(game,stack,pos,face,claimed);}
pub fn placed(game:&mut crate::game::Game<'_>,stack:&ItemStack,pos:[i32;3],state:horizon_world::registry::BlockStateId){
 let _=(game,stack,pos,state);}
pub fn destroy(game:&mut crate::game::Game<'_>,pos:[i32;3],state:horizon_world::registry::BlockStateId,canceled:&mut bool){
 let _=(game,pos,state,canceled);}
