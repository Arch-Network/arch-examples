use borsh::{BorshDeserialize, BorshSerialize};

use crate::state::Side;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub enum OrderbookInstruction {
    /// Initialize a new orderbook
    ///
    /// Accounts expected:
    /// 0. `[writable]` The orderbook account to initialize
    /// 1. `[]` The token mint account this orderbook will be for
    /// 2. `[]` The token program ID
    InitializeOrderbook,

    /// Place a new order
    ///
    /// Accounts expected:
    /// 0. `[writable]` The orderbook account
    /// 1. `[writable]` The order account to store the order
    /// 2. `[signer]` The order owner
    /// 3. `[writable]` The token account to debit/credit
    /// 4. `[]` The token program ID
    PlaceLimitOrder { side: Side, price: u64, size: u64 },

    /// Place a market order
    ///
    /// Accounts expected:
    /// 0. `[writable]` The orderbook account
    /// 1. `[writable]` The order account
    /// 2. `[signer]` The order owner
    /// 3. `[writable]` The token account to debit/credit
    PlaceMarketOrder { side: Side, size: u64 },

    /// Cancel an existing order
    ///
    /// Accounts expected:
    /// 0. `[writable]` The orderbook account
    /// 1. `[writable]` The order account
    /// 2. `[signer]` The order owner
    /// 3. `[writable]` The token account to refund
    /// 4. `[]` The token program ID
    CancelOrder { order_index: u32 },

    /// Match orders
    ///
    /// Accounts expected:
    /// 0. `[writable]` The orderbook account
    /// 1. `[writable]` The token account to debit/credit
    /// 2. `[writable]` The token account to debit/credit
    MatchOrders,
}
