use std::time::Duration;
use crate::weapons::Weapon;

pub const NAME: &str = "Attack Speed Up";
pub const ASSET: &str = "rewards/AtkSpdBox.png";

pub fn apply(weapon: &mut Weapon) {
    let current = weapon.shoot_timer.duration().as_secs_f32();
    let new_duration = (current - 0.03).max(0.05);
    weapon.fire_rate = new_duration;
    weapon.shoot_timer.set_duration(Duration::from_secs_f32(new_duration));
}
