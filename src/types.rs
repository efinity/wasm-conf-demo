use efinity_contracts::{AccountId, CollectionId, TokenId};
use ink_storage::traits::SpreadAllocate;
use ink_storage::traits::{PackedLayout, SpreadLayout, StorageLayout};
use scale::{Decode, Encode};
use scale_info::TypeInfo;

// Game

#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Config {
    pub collection_id: CollectionId,
    pub gold_token_id: TokenId,
    pub initial_token_id: TokenId,
    pub initial_hero_health: u32,
    pub initial_hero_stats_range: Range<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            collection_id: 1000,
            gold_token_id: 0,
            initial_token_id: 1,
            initial_hero_health: 100,
            initial_hero_stats_range: Range { start: 1, end: 6 },
        }
    }
}

#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct ConfigMutation {
    pub collection_id: Option<CollectionId>,
    pub gold_token_id: Option<TokenId>,
    pub initial_token_id: Option<TokenId>,
    pub initial_hero_health: Option<u32>,
}

#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone)]
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
    /// The current battle
    pub battle: Option<Battle>,
}

impl Hero {
    pub fn new(health: u32, weapon_id: TokenId, hat_id: Option<TokenId>) -> Self {
        Self {
            max_health: health,
            health,
            weapon_id,
            hat_id,
            potion_count: 0,
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
#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Enemy {
    /// Remaining health
    pub health: u16,
    /// Determines the power of a delivered attack
    pub strength: u16,
}

/// One battle per hero
#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq)]
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
