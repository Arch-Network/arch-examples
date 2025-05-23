use arch_program::{
    account::{next_account_info, AccountInfo, MIN_ACCOUNT_LAMPORTS},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction::create_account,
};

use borsh::BorshDeserialize;

use crate::{
    instruction::OrderbookInstruction,
    state::{Order, OrderbookState, Side},
};

pub struct Processor {}

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let orderbook_account = next_account_info(account_info_iter)?;
        let first_token_mint = next_account_info(account_info_iter)?;
        let second_token_mint = next_account_info(account_info_iter)?;

        let orderbook_bump = derive_and_verify_pda(
            program_id,
            &[
                b"orderbook",
                first_token_mint.key.as_ref(),
                second_token_mint.key.as_ref(),
            ],
            orderbook_account.key,
        )?;

        match OrderbookInstruction::try_from_slice(instruction_data).unwrap() {
            OrderbookInstruction::InitializeOrderbook => {
                msg!("Instruction: InitializeOrderbook");

                let payer = next_account_info(account_info_iter)?;
                let system_program = next_account_info(account_info_iter)?;

                Self::process_initialize_orderbook(
                    program_id,
                    orderbook_account,
                    first_token_mint,
                    second_token_mint,
                    payer,
                    system_program,
                    orderbook_bump,
                )
            }
            OrderbookInstruction::PlaceLimitOrder { side, price, size } => {
                msg!("Instruction: PlaceLimitOrder");

                let owner = next_account_info(account_info_iter)?;
                let token1_account = next_account_info(account_info_iter)?;
                let token2_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;

                Self::process_place_order(
                    program_id,
                    orderbook_account,
                    owner,
                    first_token_mint,
                    second_token_mint,
                    token1_account,
                    token2_account,
                    token_program,
                    side,
                    price,
                    size,
                )
            }
            OrderbookInstruction::PlaceMarketOrder { side, size } => {
                msg!("Instruction: PlaceMarketOrder");

                let owner = next_account_info(account_info_iter)?;
                let bid_side_token1_account = next_account_info(account_info_iter)?;
                let bid_side_token2_account = next_account_info(account_info_iter)?;
                let ask_side_token1_account = next_account_info(account_info_iter)?;
                let ask_side_token2_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;

                let orderbook_bump = derive_and_verify_pda(
                    program_id,
                    &[
                        b"orderbook",
                        first_token_mint.key.as_ref(),
                        second_token_mint.key.as_ref(),
                    ],
                    orderbook_account.key,
                )?;

                Self::process_place_order(
                    program_id,
                    orderbook_account,
                    owner,
                    first_token_mint,
                    second_token_mint,
                    if side == Side::Bid {
                        bid_side_token1_account
                    } else {
                        ask_side_token1_account
                    },
                    if side == Side::Bid {
                        bid_side_token2_account
                    } else {
                        ask_side_token2_account
                    },
                    token_program,
                    side,
                    if side == Side::Bid { u64::MAX } else { 0 },
                    size,
                )?;

                Self::process_match_orders(
                    program_id,
                    orderbook_account,
                    bid_side_token1_account,
                    bid_side_token2_account,
                    ask_side_token1_account,
                    ask_side_token2_account,
                    first_token_mint,
                    second_token_mint,
                    token_program,
                    orderbook_bump,
                )
            }
            OrderbookInstruction::CancelOrder { order_index } => {
                msg!("Instruction: CancelOrder");

                let owner = next_account_info(account_info_iter)?;
                let token1_account = next_account_info(account_info_iter)?;
                let token2_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;

                Self::process_cancel_order(
                    program_id,
                    orderbook_account,
                    owner,
                    token1_account,
                    token2_account,
                    token_program,
                    order_index,
                )
            }
            OrderbookInstruction::MatchOrders => {
                msg!("Instruction: MatchOrders");

                let bid_side_token1_account = next_account_info(account_info_iter)?;
                let bid_side_token2_account = next_account_info(account_info_iter)?;
                let ask_side_token1_account = next_account_info(account_info_iter)?;
                let ask_side_token2_account = next_account_info(account_info_iter)?;
                let token_program = next_account_info(account_info_iter)?;

                Self::process_match_orders(
                    program_id,
                    orderbook_account,
                    bid_side_token1_account,
                    bid_side_token2_account,
                    ask_side_token1_account,
                    ask_side_token2_account,
                    first_token_mint,
                    second_token_mint,
                    token_program,
                    orderbook_bump,
                )
            }
        }
    }

    fn _check_pda_accounts(
        program_id: &Pubkey,
        payer_key: &Pubkey,
        first_token_mint_key: &Pubkey,
        second_token_mint_key: &Pubkey,
        orderbook_account: &AccountInfo,
        first_token_account: &AccountInfo,
        second_token_account: &AccountInfo,
    ) -> Result<(u8, u8, u8), ProgramError> {
        // Derive PDA and bump
        let seeds = &[b"orderbook", payer_key.as_ref()];
        let (orderbook_pda, orderbook_bump) = Pubkey::find_program_address(seeds, program_id);

        let seeds = &[b"orderbook", first_token_mint_key.as_ref()];
        let (first_token_pda, first_token_bump) = Pubkey::find_program_address(seeds, program_id);

        let seeds = &[b"orderbook", second_token_mint_key.as_ref()];
        let (second_token_pda, second_token_bump) = Pubkey::find_program_address(seeds, program_id);

        // Verify PDA
        if orderbook_pda != *orderbook_account.key {
            msg!("Error: PDA does not match");
            return Err(ProgramError::InvalidSeeds);
        }
        if first_token_pda != *first_token_account.key {
            msg!("Error: PDA does not match");
            return Err(ProgramError::InvalidSeeds);
        }
        if second_token_pda != *second_token_account.key {
            msg!("Error: PDA does not match");
            return Err(ProgramError::InvalidSeeds);
        }

        Ok((orderbook_bump, first_token_bump, second_token_bump))
    }

    // fn create_associated_token_account<'a>(
    //     funder_info: &AccountInfo<'a>,
    //     associated_token_account_info: &AccountInfo<'a>,
    //     wallet_account_info: &AccountInfo<'a>,
    //     spl_token_mint_info: &AccountInfo<'a>,
    //     system_program_info: &AccountInfo<'a>,
    //     spl_token_program_info: &AccountInfo<'a>,
    //     txid: [u8; 32],
    //     vout: u32,
    // ) -> ProgramResult {
    //     let mut data = vec![];
    //     data.extend_from_slice(&txid);
    //     data.extend_from_slice(&vout.to_le_bytes());
    //     invoke(
    //         &Instruction {
    //             program_id: apl_associated_token_account::id(),
    //             accounts: vec![
    //                 AccountMeta::new(*funder_info.key, true),
    //                 AccountMeta::new(*associated_token_account_info.key, false),
    //                 AccountMeta::new(*wallet_account_info.key, false),
    //                 AccountMeta::new(*spl_token_mint_info.key, false),
    //                 AccountMeta::new(*system_program_info.key, false),
    //                 AccountMeta::new(*spl_token_program_info.key, false),
    //             ],
    //             data,
    //         },
    //         &[
    //             funder_info.clone(),
    //             associated_token_account_info.clone(),
    //             wallet_account_info.clone(),
    //             spl_token_mint_info.clone(),
    //             system_program_info.clone(),
    //             spl_token_program_info.clone(),
    //         ],
    //     )
    // }

    fn _create_and_initialize_token_account<'a>(
        payer: &AccountInfo<'a>,
        token_account: &AccountInfo<'a>,
        token_mint: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        current_program: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
        token_bump: u8,
    ) -> ProgramResult {
        msg!("Creating token account");
        invoke_signed(
            &create_account(
                payer.key,
                token_account.key,
                MIN_ACCOUNT_LAMPORTS,
                apl_token::state::Account::LEN as u64,
                &apl_token::id(),
            ),
            &[payer.clone(), token_account.clone(), system_program.clone()],
            &[&[b"orderbook", token_mint.key.as_ref(), &[token_bump]]],
        )?;
        msg!("Initializing token account");
        invoke_signed(
            &apl_token::instruction::initialize_account(
                token_program.key,
                token_account.key,
                token_mint.key,
                current_program.key,
            )
            .unwrap(),
            &[
                payer.clone(),
                token_account.clone(),
                token_mint.clone(),
                token_program.clone(),
                current_program.clone(),
            ],
            &[&[b"orderbook", token_mint.key.as_ref(), &[token_bump]]],
        )
    }

    fn process_initialize_orderbook<'a>(
        program_id: &Pubkey,
        orderbook_account: &AccountInfo<'a>,
        first_token_mint: &AccountInfo<'a>,
        second_token_mint: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
        orderbook_bump: u8,
    ) -> ProgramResult {
        msg!("Creating orderbook account");

        // Create orderbook account
        invoke_signed(
            &create_account(
                payer.key,
                orderbook_account.key,
                MIN_ACCOUNT_LAMPORTS,
                std::mem::size_of::<OrderbookState>() as u64,
                program_id,
            ),
            &[
                payer.clone(),
                orderbook_account.clone(),
                system_program.clone(),
            ],
            &[&[
                b"orderbook",
                first_token_mint.key.as_ref(),
                second_token_mint.key.as_ref(),
                &[orderbook_bump],
            ]],
        )?;

        msg!("Initializing orderbook account");

        let orderbook = unsafe {
            &mut *(orderbook_account.data.borrow_mut().as_mut_ptr() as *mut OrderbookState)
        };

        orderbook.initialized = true;
        orderbook.num_orders = 0;
        orderbook.first_token_mint = first_token_mint.key.clone();
        orderbook.second_token_mint = second_token_mint.key.clone();

        msg!("Orderbook initialized: {:?}", orderbook);

        Ok(())
    }

    fn process_place_order<'a>(
        _program_id: &Pubkey,
        orderbook_account: &AccountInfo<'a>,
        owner: &AccountInfo<'a>,
        first_token_mint: &AccountInfo<'a>,
        second_token_mint: &AccountInfo<'a>,
        token1_account: &AccountInfo<'a>,
        token2_account: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        side: Side,
        price: u64,
        size: u64,
    ) -> ProgramResult {
        if !owner.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let orderbook_state = unsafe {
            &mut *(orderbook_account.data.borrow_mut().as_mut_ptr() as *mut OrderbookState)
        };

        assert_eq!(orderbook_state.first_token_mint, *first_token_mint.key);
        assert_eq!(orderbook_state.second_token_mint, *second_token_mint.key);

        let order = Order {
            owner: *owner.key,
            token1_account: *token1_account.key,
            token2_account: *token2_account.key,
            side: side.clone(),
            price,
            size,
        };
        msg!("Inserting order {:?}", order);
        orderbook_state.insert_order(order, orderbook_account)?;

        msg!("Delegating tokens to orderbook");
        invoke(
            &apl_token::instruction::approve(
                token_program.key,
                if side == Side::Bid {
                    token2_account.key
                } else {
                    token1_account.key
                },
                orderbook_account.key,
                owner.key,
                &[],
                size,
            )
            .unwrap(),
            &[
                token_program.clone(),
                if side == Side::Bid {
                    token2_account.clone()
                } else {
                    token1_account.clone()
                },
                owner.clone(),
                orderbook_account.clone(),
            ],
        )?;

        Ok(())
    }

    fn process_cancel_order<'a>(
        _program_id: &Pubkey,
        orderbook_account: &AccountInfo<'a>,
        owner: &AccountInfo<'a>,
        token1_account: &AccountInfo<'a>,
        token2_account: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        order_index: u32,
    ) -> ProgramResult {
        if !owner.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let orderbook_state = unsafe {
            &mut *(orderbook_account.data.borrow_mut().as_mut_ptr() as *mut OrderbookState)
        };

        let order = orderbook_state.remove_order(order_index as usize, orderbook_account)?;

        // Verify order owner
        if order.owner != *owner.key {
            msg!(
                "Error: Order owner does not match {} {}",
                order.owner,
                owner.key
            );
            return Err(ProgramError::Custom(0x01));
        }

        if order.token1_account != *token1_account.key {
            msg!(
                "Error: Order token1 account does not match {} {}",
                order.token1_account,
                token1_account.key
            );
            return Err(ProgramError::Custom(0x02));
        }

        if order.token2_account != *token2_account.key {
            msg!(
                "Error: Order token2 account does not match {} {}",
                order.token2_account,
                token2_account.key
            );
            return Err(ProgramError::Custom(0x03));
        }

        invoke(
            &apl_token::instruction::revoke(
                token_program.key,
                if order.side == Side::Bid {
                    token2_account.key
                } else {
                    token1_account.key
                },
                owner.key,
                &[],
            )
            .unwrap(),
            &[
                token_program.clone(),
                if order.side == Side::Bid {
                    token2_account.clone()
                } else {
                    token1_account.clone()
                },
                owner.clone(),
            ],
        )?;
        msg!("Order canceled: {:?}", order);

        Ok(())
    }

    fn process_match_orders<'a>(
        _program_id: &Pubkey,
        orderbook_account: &AccountInfo<'a>,
        bid_side_token1_account: &AccountInfo<'a>,
        bid_side_token2_account: &AccountInfo<'a>,
        ask_side_token1_account: &AccountInfo<'a>,
        ask_side_token2_account: &AccountInfo<'a>,
        token1_mint: &AccountInfo<'a>,
        token2_mint: &AccountInfo<'a>,
        token_program: &AccountInfo<'a>,
        orderbook_bump: u8,
    ) -> Result<(), ProgramError> {
        let orderbook_state = unsafe {
            &mut *(orderbook_account.data.borrow_mut().as_mut_ptr() as *mut OrderbookState)
        };

        assert_eq!(*token1_mint.key, orderbook_state.first_token_mint);
        assert_eq!(*token2_mint.key, orderbook_state.second_token_mint);

        let (token1_amount, token2_amount) = orderbook_state.match_orders(orderbook_account)?;

        if token1_amount > 0 && token2_amount > 0 {
            invoke_signed(
                &apl_token::instruction::transfer(
                    token_program.key,
                    ask_side_token1_account.key,
                    bid_side_token1_account.key,
                    orderbook_account.key,
                    &[],
                    token1_amount,
                )
                .unwrap(),
                &[
                    token_program.clone(),
                    ask_side_token1_account.clone(),
                    bid_side_token1_account.clone(),
                    orderbook_account.clone(),
                ],
                &[&[
                    b"orderbook",
                    token1_mint.key.as_ref(),
                    token2_mint.key.as_ref(),
                    &[orderbook_bump],
                ]],
            )?;

            invoke_signed(
                &apl_token::instruction::transfer(
                    token_program.key,
                    bid_side_token2_account.key,
                    ask_side_token2_account.key,
                    orderbook_account.key,
                    &[],
                    token2_amount,
                )
                .unwrap(),
                &[
                    token_program.clone(),
                    ask_side_token2_account.clone(),
                    bid_side_token2_account.clone(),
                    orderbook_account.clone(),
                ],
                &[&[
                    b"orderbook",
                    token1_mint.key.as_ref(),
                    token2_mint.key.as_ref(),
                    &[orderbook_bump],
                ]],
            )
        } else {
            msg!("No match found");
            Err(ProgramError::Custom(0x04))
        }
    }
}

fn derive_and_verify_pda(
    program_id: &Pubkey,
    seeds: &[&[u8]],
    expected_pda: &Pubkey,
) -> Result<u8, ProgramError> {
    let (pda, bump) = Pubkey::find_program_address(seeds, program_id);
    if pda != *expected_pda {
        msg!("Error: PDA does not match {} {}", pda, expected_pda);
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}
