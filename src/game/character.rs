use bevy::prelude::*;
use std::fmt::Debug;
use std::ops::Index;

#[derive(Debug)]
pub enum CharacterStat {
    STR,
    ARC,
    INT,
    WIS,
    WIL,
    AGI,
}

#[derive(Component)]
pub struct Character {
    pub strength: i32,
    pub arcane: i32,
    pub intelligence: i32,
    pub wisdom: i32,
    pub willpower: i32,
    pub agility: i32,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            strength: 3,
            arcane: 3,
            intelligence: 3,
            wisdom: 3,
            willpower: 3,
            agility: 3,
        }
    }
}

impl Debug for Character {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "STR[{}] ARC[{}] INT[{}] WIS[{}] WIL[{}] AGI[{}]",
            self.strength,
            self.arcane,
            self.intelligence,
            self.wisdom,
            self.willpower,
            self.agility
        ))
    }
}

impl Index<CharacterStat> for Character {
    type Output = i32;

    fn index(&self, index: CharacterStat) -> &Self::Output {
        match index {
            CharacterStat::STR => &self.strength,
            CharacterStat::ARC => &self.arcane,
            CharacterStat::INT => &self.intelligence,
            CharacterStat::WIS => &self.wisdom,
            CharacterStat::WIL => &self.willpower,
            CharacterStat::AGI => &self.agility,
        }
    }
}

impl Character {
    pub fn calculate_cost(&self, stat: CharacterStat) -> i32 {
        match self[stat] {
            i32::MIN..=0_i32 => 200,
            1 => 150,
            2 => 125,
            3 => 100,
            4 => 75,
            5 => 60,
            6 => 50,
            7 => 40,
            8 => 30,
            9 => 25,
            10_i32..=i32::MAX => 20,
        }
    }
}