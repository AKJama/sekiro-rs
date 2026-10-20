//! Wolf's start kit from FwLo's RustSekiro `config.toml`.
//! gap: nothing spends or adds to the kit yet (no gourd drinking, pickups, shops); it is what
//! the menu shows.

/// What Wolf carries and has learned, independent of the screen that shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Kit {
    /// Healing Gourd uses left.
    pub gourd_charges: u32,
    /// Spirit Emblems carried (the menu's starting capacity is 15).
    pub spirit_emblems: u32,
    /// Resurrection nodes available.
    pub resurrections: u32,
    /// Equipped combat art, an EquipParamWeapon id.
    pub combat_art: i32,
    /// Fitted tools, up to three EquipParamWeapon ids.
    pub prosthetics: Vec<i32>,
    /// Quick-item EquipParamGoods ids, with 0 for an empty slot.
    pub quick_items: [i32; 5],
    /// Goods held besides the gourd and emblems: EquipParamGoods id and count.
    pub goods: Vec<(i32, u32)>,
    /// Sen carried.
    pub sen: u32,
    /// Skill points available.
    pub skill_points: u32,
    /// Whether every skill counts as learned (FwLo's `unlock_all_moves`).
    pub all_skills: bool,
    /// Learned SkillParam ids when `all_skills` is false.
    pub skills: Vec<i32>,
}

impl Default for Kit {
    fn default() -> Self {
        Self {
            gourd_charges: 3,
            spirit_emblems: 15,
            resurrections: 2,
            combat_art: 5100,
            prosthetics: vec![70000],
            quick_items: [3000, 3400, 3020, 3070, 3980],
            goods: vec![
                (3020, 3),
                (3040, 1),
                (3050, 3),
                (3060, 1),
                (3070, 3),
                (3080, 3),
                (3200, 3),
                (3210, 3),
                (3220, 3),
                (3230, 3),
                (3400, 3),
                (3410, 3),
                (3420, 3),
                (3430, 3),
                (3440, 3),
                (3450, 5),
                (3500, 10),
                (3510, 5),
                (3520, 5),
                (3580, 5),
                (3600, 3),
                (3610, 3),
                (3620, 3),
                (3630, 3),
                (3700, 3),
                (3704, 2),
                (3708, 1),
                (3800, 1),
                (3980, 1),
                (6000, 20),
                (6010, 5),
                (6100, 10),
                (6200, 5),
                (6300, 10),
                (6400, 3),
                (9720, 1),
                (9750, 1),
            ],
            sen: 3000,
            skill_points: 14,
            all_skills: true,
            skills: vec![280],
        }
    }
}

impl Kit {
    /// The gourd's charges, or the number of this goods id held (zero when absent).
    pub fn count(&self, id: i32) -> u32 {
        if id == 3000 {
            self.gourd_charges
        } else {
            self.goods
                .iter()
                .find(|(i, _)| *i == id)
                .map_or(0, |(_, n)| *n)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_the_gourd_goods_and_missing_items() {
        let kit = Kit::default();
        assert_eq!(kit.count(3000), 3);
        assert_eq!(kit.count(3020), 3);
        assert_eq!(kit.count(1), 0);
    }
}
