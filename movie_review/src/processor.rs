use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

use crate::error::ReviewError;
use crate::instruction::MovieInstruction;
use crate::state::MovieAccountState;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = MovieInstruction::unpack(instruction_data)?;

    match instruction {
        MovieInstruction::AddMovieReview {
            title,
            rating,
            description,
        } => {
            add_movie_review(program_id, accounts, title, rating, description)?;
        }
        MovieInstruction::UpdateMovieReview {
            title,
            rating,
            description,
        } => {
            update_movie_review(program_id, accounts, title, rating, description)?;
        }
    }
    Ok(())
}
pub fn add_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let initializer = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Check if the instruction is signed
    if !initializer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Since we need bump_seed, we still derive pda instead of using pda_account directly
    let (pda, bump_seed) =
        Pubkey::find_program_address(&[initializer.key.as_ref(), title.as_bytes()], program_id);

    // Check derived PDA equals given PDA
    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    // Check rating is between 1 and 5
    if !(1..=5).contains(&rating) {
        msg!("Invalid rating");
        return Err(ReviewError::InvalidRating.into());
    };

    // Check the content of the review does not exceed the maximum length
    let total_len = 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        msg!("Input data exceeds max length");
        return Err(ReviewError::InvalidDataLength.into());
    }

    let account_len = 1000;

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_len);

    let create_account = system_instruction::create_account(
        initializer.key,
        pda_account.key,
        rent_lamports,
        account_len.try_into().unwrap(),
        program_id,
    );

    let keys = [
        initializer.clone(),
        pda_account.clone(),
        system_program.clone(),
    ];

    let seeds = [initializer.key.as_ref(), title.as_bytes(), &[bump_seed]];

    // Create the account
    invoke_signed(&create_account, keys.as_ref(), &[seeds.as_ref()])?;

    msg!("PDA created: {}", pda);

    msg!("unpacking state account");
    let mut account_data =
        try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow())?;
    msg!("borrowed account data");

    msg!("checking if account is initialized");
    if account_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    msg!("serializing account");
    account_data.serialize(&mut *pda_account.data.borrow_mut())?;
    msg!("state account serialized");

    Ok(())
}

pub fn update_movie_review(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    title: String,
    rating: u8,
    description: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let updater = next_account_info(account_info_iter)?;
    let pda_account = next_account_info(account_info_iter)?;

    // Check if the instruction is signed
    if pda_account.owner != program_id {
        msg!("Invalid account owner");
        return Err(ProgramError::IllegalOwner);
    }

    if !updater.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, _bump_seed) =
        Pubkey::find_program_address(&[updater.key.as_ref(), title.as_bytes()], program_id);

    if pda != *pda_account.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    if !(1..=5).contains(&rating) {
        msg!("Invalid rating");
        return Err(ReviewError::InvalidRating.into());
    };

    let total_len = 1 + 1 + (4 + title.len()) + (4 + description.len());
    if total_len > 1000 {
        msg!("Input data exceeds max length");
        return Err(ReviewError::InvalidDataLength.into());
    }

    msg!("unpacking state account");
    let mut account_data =
        try_from_slice_unchecked::<MovieAccountState>(&pda_account.data.borrow())?;
    msg!("borrowed account data");

    if !account_data.is_initialized() {
        msg!("Account not initialized yet");
        return Err(ReviewError::UninitializedAccount.into());
    }

    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;

    account_data.serialize(&mut *pda_account.data.borrow_mut())?;

    Ok(())
}
