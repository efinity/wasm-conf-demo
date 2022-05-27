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
    pub initial_hero_health: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            collection_id: 1000,
            gold_token_id: 0,
            initial_token_id: 1,
            initial_hero_health: 100
        }
    }
}

#[derive(Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct ConfigMutation {
    pub collection_id: Option<CollectionId>,
    pub gold_token_id: Option<TokenId>,
    pub initial_token_id: Option<TokenId>,
    pub initial_hero_health: Option<u16>,
}

// Battle

/// A unique identifier for a battle
pub type BattleId = u128;

/// The entity that represents the player
#[derive(Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, SpreadLayout, PackedLayout)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Hero {
    /// Max health
    pub max_health: u16,
    /// Current health
    pub health: u16,
    /// `TokenId` of current weapon
    pub weapon_token_id: Option<TokenId>,
    /// `TokenId` of current armor
    pub armor_token_id: Option<TokenId>,
    pub potion_count: u8,
}

impl Hero {
    pub fn new(health: u16, weapon_token_id: Option<TokenId>, armor_token_id: Option<TokenId>) -> Self {
        Self {
            max_health: health,
            health,
            weapon_token_id,
            armor_token_id,
            potion_count: 0,
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
#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Enemy {

    /// Remaining health
    pub health: u16,
    /// Determines the power of a delivered attack
    pub strength: u16,
    /// Determines the power of a received attack
    pub defense: u16,
}

/// One battle per hero
#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(feature = "std", derive(TypeInfo, StorageLayout))]
pub struct Battle {
    pub hero_id: AccountId,
    pub enemy: Enemy,
}

// Tokens

#[derive(Encode, Decode)]
#[repr(u8)]
pub enum TokenType {
    Weapon,
    Armor,
}

#[derive(Encode, Decode)]
pub struct TokenMetadata {
    pub token_type: TokenType,
    pub value: u16,
}

pub enum WeaponType {
    Sword,
    Axe
}

pub struct Weapon {
    pub weapon_type: WeaponType,
    pub strength: u16,
}

pub struct Armor {
    pub defense: u16,
}
