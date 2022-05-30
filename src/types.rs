use efinity_contracts::{TokenBalance, TokenId};
use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout};
use scale::{Decode, Encode};
use scale_info::TypeInfo;

// Game

#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Clone)]
#[cfg_attr(
    feature = "std",
    derive(TypeInfo, ink_storage::traits::StorageLayout, Eq, PartialEq)
)]
pub struct Config {
    pub hero_max_health: u32,
    pub starting_weapon_strength_range: Range,
    pub purchased_weapon_strength_range: Range,
    /// The number of potions a hero starts with
    pub hero_initial_potion_count: u32,
    pub enemy_health_range: Range,
    pub enemy_strength_range: Range,
    pub enemy_gold_drop_range: Range,
    /// An attack will randomly be plus or minus this number or less
    /// For example, if it's 2, all attacks will be strength plus or minus 2, 1, or 0
    pub attack_variance: u32,
    /// Percentage between 0 and 100
    pub enemy_wearing_hat_chance: u32,
    pub hero_goes_first_chance: u32,
    pub rest_cost: TokenBalance,
    pub potion_cost: TokenBalance,
    pub weapon_cost: TokenBalance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hero_max_health: 30,
            starting_weapon_strength_range: (3, 8).into(),
            purchased_weapon_strength_range: (6, 11).into(),
            hero_initial_potion_count: 1,
            enemy_health_range: (15, 35).into(),
            enemy_strength_range: (5, 15).into(),
            enemy_gold_drop_range: (15, 40).into(),
            attack_variance: 2,
            enemy_wearing_hat_chance: 35,
            hero_goes_first_chance: 50,
            rest_cost: 15,
            potion_cost: 50,
            weapon_cost: 100,
        }
    }
}

#[derive(Debug, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct ConfigMutation {
    pub hero_max_health: Option<u32>,
    pub starting_weapon_strength_range: Option<Range>,
    pub purchased_weapon_strength_range: Option<Range>,
    pub hero_initial_potion_count: Option<u32>,
    pub enemy_health_range: Option<Range>,
    pub enemy_strength_range: Option<Range>,
    pub enemy_gold_drop_range: Option<Range>,
    pub enemy_wearing_hat_chance: Option<u32>,
    pub hero_goes_first_chance: Option<u32>,
    pub rest_cost: Option<TokenBalance>,
    pub potion_cost: Option<TokenBalance>,
    pub weapon_cost: Option<TokenBalance>,
}

impl ConfigMutation {
    pub fn apply_to(self, config: &mut Config) {
        /// Set the field on `config` if it is `Some` on `self`
        macro_rules! maybe_set_field {
            ($name:ident) => {
                if let Some($name) = self.$name {
                    config.$name = $name;
                }
            };
        }

        // set the fields that are `Some`
        maybe_set_field!(hero_max_health);
        maybe_set_field!(starting_weapon_strength_range);
        maybe_set_field!(hero_initial_potion_count);
        maybe_set_field!(purchased_weapon_strength_range);
        maybe_set_field!(enemy_health_range);
        maybe_set_field!(enemy_strength_range);
        maybe_set_field!(enemy_gold_drop_range);
        maybe_set_field!(enemy_wearing_hat_chance);
        maybe_set_field!(hero_goes_first_chance);
        maybe_set_field!(rest_cost);
        maybe_set_field!(potion_cost);
        maybe_set_field!(weapon_cost);
    }
}

/// The range is inclusive
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct Range {
    pub start: u32,
    pub end: u32,
}

impl Range {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
    pub fn contains(&self, value: u32) -> bool {
        value >= self.start && value <= self.end
    }
}

impl From<(u32, u32)> for Range {
    fn from(values: (u32, u32)) -> Self {
        Self {
            start: values.0,
            end: values.1,
        }
    }
}

// Battle

/// The entity that represents the player
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct Hero {
    /// Current health
    pub health: u32,
    /// `TokenId` of current weapon
    pub weapon_id: TokenId,
    /// `TokenId` of current hat
    pub hat_id: Option<TokenId>,
    pub potion_count: u32,
    /// The current battle
    pub battle: Option<Battle>,
    /// The highest number of battles won in a row achieved by this hero
    pub highest_consecutive_victory_count: u32,
    /// The number of battles won in a row, without defeat
    pub consecutive_victory_count: u32,
}

impl Hero {
    pub fn new(health: u32, weapon_id: TokenId, potion_count: u32) -> Self {
        Self {
            health,
            weapon_id,
            hat_id: None,
            potion_count,
            highest_consecutive_victory_count: 0,
            consecutive_victory_count: 0,
            battle: None,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.health == 0
    }
}

/// An action that can be taken in battle
#[derive(Encode, Decode, TypeInfo)]
pub enum Command {
    /// Deliver damage
    Attack,
    /// Recover damage received
    Heal,
}

/// An entity that can be fought
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct Enemy {
    pub hat_id: Option<TokenId>,
    /// Remaining health
    pub health: u32,
    /// Determines the power of a delivered attack
    pub strength: u32,
}

impl Enemy {
    pub fn is_dead(&self) -> bool {
        self.health == 0
    }
}

/// One battle per hero
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct Battle {
    pub round_number: u32,
    pub enemy: Enemy,
}

impl Battle {
    pub fn new(enemy: Enemy) -> Self {
        Self {
            round_number: 0,
            enemy,
        }
    }
}

// Tokens

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, Debug)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
#[repr(u8)]
pub enum TokenType {
    Weapon,
    Hat,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct TokenMetadata {
    pub token_type: TokenType,
    pub strength: u32,
}
