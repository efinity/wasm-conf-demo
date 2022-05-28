use efinity_contracts::{AccountId, CollectionId, TokenBalance, TokenId};
use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout, StorageLayout};
use scale::{Decode, Encode};
use scale_info::TypeInfo;

// Game

#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Clone)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout, Eq, PartialEq))]
pub struct Config {
    pub collection_id: CollectionId,
    pub gold_token_id: TokenId,
    pub initial_token_id: TokenId,
    pub initial_hero_health: u32,
    pub initial_hero_stats_range: Range<u32>,
    pub enemy_health_range: Range<u32>,
    pub enemy_strength_range: Range<u32>,
    /// Percentage between 0 and 100
    pub enemy_wearing_hat_chance: u32,
    pub potion_cost: TokenBalance,
    pub weapon_cost: TokenBalance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            collection_id: 1000,
            gold_token_id: 0,
            initial_token_id: 1,
            initial_hero_health: 15,
            initial_hero_stats_range: (1, 6).into(),
            enemy_health_range: (10, 30).into(),
            enemy_strength_range: (5, 15).into(),
            enemy_wearing_hat_chance: 40,
            potion_cost: 50,
            weapon_cost: 200,
        }
    }
}

#[derive(Debug, Encode, Decode, Default)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct ConfigMutation {
    pub initial_hero_health: Option<u32>,
    pub initial_hero_stats_range: Option<Range<u32>>,
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
        maybe_set_field!(initial_hero_health);
        maybe_set_field!(initial_hero_stats_range);
        maybe_set_field!(potion_cost);
        maybe_set_field!(weapon_cost);
    }
}

#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Range<T> {
    pub start: T,
    pub end: T,
}

impl<T> From<(T, T)> for Range<T> {
    fn from(values: (T, T)) -> Self {
        Self {
            start: values.0,
            end: values.1,
        }
    }
}

// Battle

/// The entity that represents the player
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Hero {
    /// Max health
    pub max_health: u32,
    /// Current health
    pub health: u32,
    /// `TokenId` of current weapon
    pub weapon_id: TokenId,
    /// `TokenId` of current hat
    pub hat_id: Option<TokenId>,
    pub potion_count: u32,
    pub enemies_defeated_count: u32,
    /// The current battle
    pub battle: Option<Battle>,
}

impl Hero {
    pub fn new(health: u32, weapon_id: TokenId) -> Self {
        Self {
            max_health: health,
            health,
            weapon_id,
            hat_id: None,
            potion_count: 0,
            enemies_defeated_count: 0,
            battle: None,
        }
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

pub enum EnemyType {
    Skeleton,
    Goblin,
}

/// An entity that can be fought
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Enemy {
    pub hat_id: Option<TokenId>,
    /// Remaining health
    pub health: u32,
    /// Determines the power of a delivered attack
    pub strength: u32,
}

/// One battle per hero
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Battle {
    pub hero_id: AccountId,
    pub enemy: Enemy,
}

// Tokens

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
#[repr(u8)]
pub enum TokenType {
    Weapon,
    Hat,
}

#[derive(Encode, Decode)]
pub struct TokenMetadata {
    pub token_type: TokenType,
    pub value: u32,
}

pub enum WeaponType {
    Sword,
    Axe,
}

pub struct Weapon {
    pub weapon_type: WeaponType,
    pub strength: u16,
}
