use arch_program::{
    account::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Debug, Clone, Copy, PartialEq, BorshSerialize, BorshDeserialize)]
#[repr(u8)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Order {
    /// Owner of the order
    pub owner: Pubkey,
    /// Token 1 account to credit/debit when order is executed
    pub token1_account: Pubkey,
    /// Token 2 account to credit/debit when order is executed
    pub token2_account: Pubkey,
    /// Price of the order
    pub price: u64,
    /// Size of the order
    pub size: u64,
    /// Side of the order (bid/ask)
    pub side: Side,
}

impl Sealed for Order {}

impl Pack for Order {
    const LEN: usize = std::mem::size_of::<Order>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        dst.copy_from_slice(unsafe {
            std::slice::from_raw_parts(self as *const Order as *const u8, Self::LEN)
        });
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        Ok(unsafe { *(src.as_ptr() as *const Order) })
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OrderbookState {
    /// Is initialized
    pub initialized: bool,
    /// Number of active orders
    pub num_orders: u32,
    /// The first token mint this orderbook is for
    pub first_token_mint: Pubkey,
    /// The second token mint this orderbook is for
    pub second_token_mint: Pubkey,
    // /// The current highest bid price
    // pub highest_bid: u64,
    // /// The current lowest ask price
    // pub lowest_ask: u64,
}

impl Sealed for OrderbookState {}

impl IsInitialized for OrderbookState {
    fn is_initialized(&self) -> bool {
        self.initialized
    }
}

impl OrderbookState {
    pub fn orders(&self) -> &[Order] {
        let orders_ptr = unsafe { (self as *const Self).offset(1) as *const Order };
        unsafe { std::slice::from_raw_parts(orders_ptr, self.num_orders as usize) }
    }

    pub fn orders_mut(&mut self) -> &mut [Order] {
        let orders_ptr = unsafe { (self as *mut Self).offset(1) as *mut Order };
        unsafe { std::slice::from_raw_parts_mut(orders_ptr, self.num_orders as usize) }
    }

    pub fn insert_order(
        &mut self,
        new_order: Order,
        orderbook_account: &AccountInfo,
    ) -> Result<(), ProgramError> {
        let orders = self.orders();
        let mut first_ask_index = 0;
        for order in orders {
            if order.side == Side::Ask {
                break;
            }
            first_ask_index += 1;
        }

        let mut input_index = match new_order.side {
            Side::Bid => 0,
            Side::Ask => first_ask_index,
        };
        let range = match new_order.side {
            Side::Bid => 0..first_ask_index,
            Side::Ask => first_ask_index..orders.len(),
        };
        for order in orders[range].iter() {
            if order.price > new_order.price {
                break;
            }
            input_index += 1;
        }
        msg!("input_index: {:?}", input_index);
        msg!("orders.len: {:?}", orders.len());

        let order_capacity = (orderbook_account.data_len() - std::mem::size_of::<OrderbookState>())
            / std::mem::size_of::<Order>();
        if self.num_orders as usize >= order_capacity {
            orderbook_account.realloc(OrderbookState::size_of(self.num_orders as u32 + 1), true)?;
        }
        self.num_orders += 1;

        let orders: &mut [Order] = self.orders_mut();
        msg!("orders.len: {:?}", orders.len());
        if input_index + 1 < orders.len() {
            orders.copy_within(input_index..orders.len() - 1, input_index + 1);
        }

        orders[input_index] = new_order;

        Ok(())
    }

    // Calculate total required size for n orders
    pub fn size_of(num_orders: u32) -> usize {
        std::mem::size_of::<OrderbookState>() + (num_orders as usize * std::mem::size_of::<Order>())
    }

    pub fn remove_order(
        &mut self,
        index: usize,
        orderbook_account: &AccountInfo,
    ) -> Result<Order, ProgramError> {
        if index >= self.num_orders as usize {
            return Err(ProgramError::InvalidArgument);
        }

        let orders = self.orders_mut();
        let removed_order = orders[index];

        // Shift remaining orders left
        orders.copy_within(index + 1.., index);
        self.num_orders -= 1;

        orderbook_account.realloc(OrderbookState::size_of(self.num_orders as u32), true)?;

        Ok(removed_order)
    }

    pub fn match_orders<'a>(
        &mut self,
        orderbook_account: &AccountInfo,
    ) -> Result<(u64, u64), ProgramError> {
        let orders = self.orders();
        let mut first_ask_index = 0;
        for order in orders.iter() {
            if order.side == Side::Ask {
                break;
            }
            first_ask_index += 1;
        }

        let result = if orders[0].price == orders[first_ask_index].price {
            if orders[0].size < orders[first_ask_index].size {
                (
                    orders[0].size,
                    orders[0].size * orders[first_ask_index].price,
                )
            } else {
                (
                    orders[first_ask_index].size,
                    orders[0].size * orders[first_ask_index].price,
                )
            }
        } else if orders[0].price > orders[first_ask_index].price {
            if orders[0].size < orders[first_ask_index].size {
                (
                    orders[0].size,
                    orders[0].size * orders[first_ask_index].price,
                )
            } else {
                (
                    orders[first_ask_index].size,
                    orders[0].size * orders[first_ask_index].price,
                )
            }
        } else {
            (0, 0)
        };

        let orders = self.orders_mut();
        orders[first_ask_index].size -= result.0;
        orders[0].size -= result.0;

        let bid_filled = orders[0].size == 0;
        let ask_filled = orders[first_ask_index].size == 0;
        if bid_filled {
            self.remove_order(0, orderbook_account)?;
        }

        if ask_filled {
            self.remove_order(first_ask_index - 1, orderbook_account)?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_serialization() {
        println!(
            "Orderbook size: {:?}",
            std::mem::size_of::<OrderbookState>()
        );

        let order = Order {
            owner: Pubkey([10; 32]),
            token1_account: Pubkey([11; 32]),
            token2_account: Pubkey([12; 32]),
            price: 100,
            size: 200,
            side: Side::Ask,
        };

        let serialized_order = unsafe {
            std::slice::from_raw_parts(
                &order as *const Order as *const u8,
                std::mem::size_of::<Order>(),
            )
        };

        println!("serialized_order: {:?}", serialized_order);

        let orders: &[Order] = &[order];

        let serialized = unsafe {
            std::slice::from_raw_parts(orders.as_ptr() as *const u8, std::mem::size_of::<Order>())
        };

        println!("serialized: {:?}", serialized);

        println!("{:?}", serialized_order == serialized);

        // let mut lamports = 0;
        // let pubkey = Pubkey::default();
        // let owner = Pubkey::default();
        // let utxo_meta = UtxoMeta::default();

        // let orderbook_account = AccountInfo::new(
        //     &pubkey,
        //     &mut lamports,
        //     &mut serialized,
        //     &owner,
        //     &utxo_meta,
        //     false,
        //     false,
        //     false,
        // );

        // let mut orderbook_data = orderbook_account.data.try_borrow_mut().unwrap();

        // let orderbook: &mut OrderbookState =
        //     unsafe { &mut *(orderbook_data.as_mut_ptr() as *mut OrderbookState) };

        // orderbook
        //     .insert_order(
        //         Order {
        //             owner: Pubkey::default(),
        //             token_account: Pubkey::default(),
        //             price: 100,
        //             size: 100,
        //             side: Side::Bid,
        //         },
        //         &orderbook_account,
        //     )
        //     .unwrap();

        // let orders = orderbook.orders();
        // println!("orders: {:?}", orders);

        // println!("serialized {:?}", serialized);
        // println!("Orderbook: {:?}", orderbook);

        // println!("orderbook pointer: {:?}", orderbook as *const _);
        // println!("serialized pointer: {:?}", &serialized as *const _);
        // println!(
        //     "diff: {:?}",
        //     orderbook as *const _ as usize - &serialized as *const _ as usize
        // );
    }
}
