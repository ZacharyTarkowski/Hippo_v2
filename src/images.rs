
use include_data::include_u16s;
// use crate::image_names2;

// const test : &'static str = include_str!("image_names2.rs");
// const test2 : &'static str = format!("{}",test);

//TODO FIX NAMES AND CHANGE SLEEP TO IDLE
pub const IDLE_1: &'static [u16] = include_u16s!("../Hippo_Images/Hippo_Asleep_1.png.bin");
pub const IDLE_2: &'static [u16] = include_u16s!("../Hippo_Images/Hippo_Asleep_2.png.bin");

pub const ACTIVE_1: &'static [u16] = include_u16s!("../Hippo_Images/Hippo_Awake_1.png.bin");
pub const ACTIVE_2: &'static [u16] = include_u16s!("../Hippo_Images/Hippo_Awake_2.png.bin");

