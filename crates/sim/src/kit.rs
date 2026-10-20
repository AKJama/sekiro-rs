//! Wolf's start kit from FwLo's RustSekiro `config.toml`, with Sen and skill points at their
//! new-game zero (CharaInitParam `soul` and `skillLv` are 0 on every default row).
//! Kills pay their NpcParam `getSoul` Sen and `skillPoint` experience ([`Kit::earn`]); a full
//! experience gauge is a skill point, the gauge per point from [`xp_to_next`].
//! gap: nothing spends from the kit yet (no gourd drinking, shops, skill purchase), and death
//! does not take its half of the Sen and experience (TentativePlayerParam `MoneyLostRate` /
//! `SkillPointLostRate` 50).

/// Experience from skill point n to n + 1 (n = points earned so far, spent or not); past the
/// table the last step repeats. The exe computes it, not the params: the table is the one the
/// Fextralife wiki measured (Skill Points and Experience), as FwLo's RustSekiro
/// `src/experience.rs` (MIT) ships it.
const XP_TO_NEXT: [u32; 128] = [
    500, 514, 528, 542, 557, 572, 587, 602, 618, 634, 650, 666, 682, 698, 715, 732, 749, 766, 784,
    802, 820, 838, 856, 874, 893, 1093, 1300, 1515, 1738, 1970, 2210, 2458, 2715, 2980, 3254, 3538,
    3830, 4131, 4442, 4762, 5092, 5431, 5780, 6139, 6507, 6886, 7276, 7675, 8085, 8506, 8938, 9380,
    9833, 10297, 10773, 11260, 11758, 12268, 12789, 13322, 13868, 14425, 14994, 15576, 16170,
    16777, 17396, 18028, 18673, 19331, 20002, 20686, 21383, 22094, 22819, 23558, 24310, 25076,
    25856, 26651, 27460, 28283, 29121, 29973, 30840, 31723, 32620, 33532, 34460, 35403, 36362,
    37336, 38326, 39332, 40354, 41392, 42446, 43516, 44603, 45707, 46828, 47965, 49119, 50290,
    51479, 52685, 53908, 55149, 56407, 57683, 58977, 60290, 61620, 62969, 64336, 65722, 67126,
    68549, 69991, 71452, 72932, 74431, 75949, 77487, 79045, 80622, 82220, 83837,
];

/// Experience needed from `earned` skill points to the next.
fn xp_to_next(earned: u32) -> u32 {
    XP_TO_NEXT[(earned as usize).min(XP_TO_NEXT.len() - 1)]
}

/// One defeat's gain, as [`Kit::earn`] recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Reward {
    /// Experience gained.
    pub xp: u32,
    /// Sen gained.
    pub sen: u32,
    /// Skill points gained.
    pub points: u32,
}

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
    /// Skill points earned in all, spent or not: it picks the gauge size ([`xp_to_next`]).
    /// It equals the points earned, so it differs from `skill_points` once skills are bought.
    pub skill_level: u32,
    /// Experience toward the next skill point.
    pub skill_xp: u32,
    /// Whether every skill counts as learned (FwLo's `unlock_all_moves`).
    pub all_skills: bool,
    /// Learned SkillParam ids when `all_skills` is false.
    pub skills: Vec<i32>,
    /// Earns that gained anything, so a screen can tell a new reward from later spending.
    pub rewards: u32,
    /// The gain of the latest of those earns.
    pub last_reward: Reward,
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
            sen: 0,
            skill_points: 0,
            skill_level: 0,
            skill_xp: 0,
            all_skills: true,
            skills: vec![280],
            rewards: 0,
            last_reward: Reward::default(),
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

    /// A defeated enemy's reward: its experience fills the gauge, each full gauge a skill point
    /// with the rest carried over. Returns the skill points gained.
    /// Records the gain in `rewards` and `last_reward`.
    pub fn earn(&mut self, xp: u32, sen: u32) -> u32 {
        let before = self.skill_points;
        self.sen = self.sen.saturating_add(sen);
        self.skill_xp = self.skill_xp.saturating_add(xp);
        while self.skill_xp >= self.xp_needed() {
            self.skill_xp -= self.xp_needed();
            self.skill_level += 1;
            self.skill_points += 1;
        }
        let points = self.skill_points - before;
        if xp > 0 || sen > 0 {
            self.rewards += 1;
            self.last_reward = Reward { xp, sen, points };
        }
        points
    }

    /// The experience gauge's size at the current skill level.
    fn xp_needed(&self) -> u32 {
        xp_to_next(self.skill_level)
    }

    /// Experience still needed for the next skill point.
    pub fn xp_left(&self) -> u32 {
        self.xp_needed().saturating_sub(self.skill_xp)
    }

    /// How full the experience gauge is, 0 to 1.
    pub fn xp_fill(&self) -> f32 {
        (self.skill_xp as f32 / self.xp_needed() as f32).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_gauges_roll_over_into_skill_points() {
        let mut kit = Kit::default();
        assert_eq!(kit.earn(499, 7), 0);
        assert_eq!(kit.earn(1, 0), 1);
        assert_eq!((kit.skill_points, kit.skill_level, kit.skill_xp), (1, 1, 0));
        // 514 + 528 for the next two, 10 over; the gauge grows with the points earned.
        assert_eq!(kit.earn(514 + 528 + 10, 0), 2);
        assert_eq!((kit.skill_points, kit.skill_xp, kit.sen), (3, 10, 7));
        assert_eq!(
            (kit.rewards, kit.last_reward.xp, kit.last_reward.points),
            (3, 1052, 2)
        );
        assert_eq!(xp_to_next(25), 1093);
        assert_eq!(xp_to_next(1000), 83837);
    }

    #[test]
    fn counts_the_gourd_goods_and_missing_items() {
        let kit = Kit::default();
        assert_eq!(kit.count(3000), 3);
        assert_eq!(kit.count(3020), 3);
        assert_eq!(kit.count(1), 0);
    }
}
