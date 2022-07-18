use efinity_contracts::{TokenBalance, TokenId};
use ink_storage::traits::{PackedLayout, SpreadAllocate, SpreadLayout};
use scale::{Decode, Encode};
use scale_info::TypeInfo;

/// Error types for the game
#[derive(Debug, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    /// An error from Efinity
    EfinityError(efinity_contracts::Error),
    /// The caller does not have permission for this operation
    NoPermission,
    /// The equipment being equipped is invalid
    InvalidEquipment,
    /// The attribute could not be decoded
    AttributeDecodeFailed,
    /// A hero does not exist for the provided account id
    HeroNotFound,
    /// This operation is not allowed while in battle
    HeroIsInBattle,
    /// This operation is only allowed while in battle
    HeroNotInBattle,
    /// The hero does not have any potions
    HeroHasNoPotions,
    /// The provided account id does not have enough gold
    NotEnoughGold,
}

impl From<efinity_contracts::Error> for Error {
    fn from(error: efinity_contracts::Error) -> Self {
        Self::EfinityError(error)
    }
}

/// Result type for the game
pub type Result<T> = core::result::Result<T, Error>;

// Game

/// Coniguration values for the game
#[derive(Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Clone)]
#[cfg_attr(
    feature = "std",
    derive(TypeInfo, ink_storage::traits::StorageLayout, Eq, PartialEq)
)]
pub struct Config {
    /// Max health of the hero
    pub hero_max_health: u32,
    /// Strength range of the weapon the hero starts with
    pub starting_weapon_strength_range: Range,
    /// Strength range of a weapon that is bought
    pub purchased_weapon_strength_range: Range,
    /// The number of potions a hero starts with
    pub hero_initial_potion_count: u32,
    /// Health range of enemies
    pub enemy_health_range: Range,
    /// Strength range of enemies
    pub enemy_strength_range: Range,
    /// Range of amount of gold enemies drop
    pub enemy_gold_drop_range: Range,
    /// An attack will randomly be plus or minus this number or less
    /// For example, if it's 2, all attacks will be strength plus or minus 2, 1, or 0
    pub attack_variance: u32,
    /// Percentage of chance enemy will be wearing a hat
    pub enemy_wearing_hat_chance: u32,
    /// Percentage of chance the hero will go first each round in battle
    pub hero_goes_first_chance: u32,
    /// Cost in gold of resting
    pub rest_cost: TokenBalance,
    /// Cost in gold of a potion
    pub potion_cost: TokenBalance,
    /// Cost in gold of a weapon
    pub weapon_cost: TokenBalance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hero_max_health: 50,
            starting_weapon_strength_range: (5, 10).into(),
            purchased_weapon_strength_range: (6, 13).into(),
            hero_initial_potion_count: 2,
            enemy_health_range: (30, 60).into(),
            enemy_strength_range: (5, 15).into(),
            enemy_gold_drop_range: (20, 50).into(),
            attack_variance: 2,
            enemy_wearing_hat_chance: 35,
            hero_goes_first_chance: 50,
            rest_cost: 15,
            potion_cost: 50,
            weapon_cost: 125,
        }
    }
}

/// Can be used to update config values. See config docs for info on each field.
#[derive(Debug, Encode, Decode, Default, Clone)]
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
    /// Applies the mutation to `config`
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
    /// The start of the range
    pub start: u32,
    /// The end of a range
    pub end: u32,
}

impl Range {
    /// Create a new range
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
    /// True if `value` is between `start` and `end`, inclusive
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
    /// `TokenId` of the hero's equipped weapon
    pub weapon_id: TokenId,
    /// `TokenId` of the hero's equipped hat
    pub hat_id: Option<TokenId>,
    /// The number of potions the hero has
    pub potion_count: u32,
    /// The current battle the hero is engaged in
    pub battle: Option<Battle>,
    /// The highest number of battles won in a row achieved by this hero
    pub highest_consecutive_victory_count: u32,
    /// The number of battles won in a row, without defeat
    pub consecutive_victory_count: u32,
}

impl Hero {
    /// Create a new hero
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

    /// Returns true if the hero has no health
    pub fn is_dead(&self) -> bool {
        self.health == 0
    }
}

/// An action that can be taken in battle
#[derive(Encode, Decode, TypeInfo)]
pub enum Command {
    /// Damage the enemy
    Attack,
    /// Recover health to maximum
    Heal,
}

/// An entity that can be fought
#[derive(
    Debug, Encode, Decode, SpreadLayout, PackedLayout, SpreadAllocate, Copy, Clone, Eq, PartialEq,
)]
#[cfg_attr(feature = "std", derive(TypeInfo, ink_storage::traits::StorageLayout))]
pub struct Enemy {
    /// The token id of the hat the enemy is wearing
    pub hat_id: Option<TokenId>,
    /// Remaining health
    pub health: u32,
    /// Determines the power of a delivered attack
    pub strength: u32,
}

impl Enemy {
    /// Returns true if the enemy has no health
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
    /// The current round number of this battle
    pub round_number: u32,
    /// The enemy involved in this battle
    pub enemy: Enemy,
}

impl Battle {
    /// Create a new battle
    pub fn new(enemy: Enemy) -> Self {
        Self {
            round_number: 0,
            enemy,
        }
    }
}

// Tokens

/// A type that a token can be
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, Debug)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
#[allow(clippy::unnecessary_cast)]
#[repr(u8)]
pub enum TokenType {
    /// The token is a weapon
    Weapon = 1,
    /// The token is a hat
    Hat = 2,
}

impl TokenType {
    /// Create from a u8
    pub fn from_value(value: u8) -> Option<Self> {
        const WEAPON_VALUE: u8 = TokenType::Weapon as _;
        const HAT_VALUE: u8 = TokenType::Hat as _;

        match value {
            WEAPON_VALUE => Some(Self::Weapon),
            HAT_VALUE => Some(Self::Hat),
            _ => None,
        }
    }
}

/// Wraps the `TokenId` so we can encode the `TokenType` in it
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub(crate) struct WrappedTokenId(pub TokenId);

impl WrappedTokenId {
    /// The bit index to store the token type
    const TOKEN_TYPE_INDEX: u128 = 124;
    /// The bit-mask of the token type
    const TOKEN_TYPE_BIT_MASK: u128 = 0b1111 << Self::TOKEN_TYPE_INDEX;

    /// Create a new instance
    pub fn new(id: TokenId, token_type: Option<TokenType>) -> Self {
        let mut value = Self(id);
        value.set_token_type(token_type);
        value
    }

    /// Set the token type
    pub fn set_token_type(&mut self, token_type: Option<TokenType>) {
        let token_type_value: u128 = token_type.map(|x| x as u8).unwrap_or_default().into();

        // clear bits
        self.0 &= !Self::TOKEN_TYPE_BIT_MASK;

        // set mask
        self.0 |= (token_type_value << Self::TOKEN_TYPE_INDEX) as u128;
    }

    /// Get the token type
    pub fn token_type(&self) -> Option<TokenType> {
        let value = (self.0 & Self::TOKEN_TYPE_BIT_MASK) >> Self::TOKEN_TYPE_INDEX;
        TokenType::from_value(value as u8)
    }

    /// The id without any encoding
    pub fn id(&self) -> TokenId {
        self.0 & !Self::TOKEN_TYPE_BIT_MASK
    }
}

/// Metadata stored for the token as an attribute
#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct TokenMetadata {
    /// The strength value
    pub strength: u32,
}

/// Returned from `get_token_info` message. Contains info about a token id.
#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(TypeInfo))]
pub struct TokenIdInfo {
    /// The type of the token
    pub token_type: Option<TokenType>,
    /// The id of the token
    pub id: TokenId,
}
