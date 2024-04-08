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
use crate::state::{MovieAccountState, MovieComment, MovieCommentCounter};

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
        MovieInstruction::AddComment { comment } => {
            add_comment(program_id, accounts, comment)?;
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
    let reviewer = next_account_info(account_info_iter)?;
    let pda_review = next_account_info(account_info_iter)?;
    let pda_comment_counter = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    // Check if the instruction is signed
    if !reviewer.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Derive review PDA
    let (pda_review_key, bump_seed) =
        Pubkey::find_program_address(&[reviewer.key.as_ref(), title.as_bytes()], program_id);

    // Check derived review PDA equals given review PDA
    if pda_review_key != *pda_review.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    // Check rating is between 1 and 5
    if !(1..=5).contains(&rating) {
        msg!("Invalid rating");
        return Err(ReviewError::InvalidRating.into());
    };

    // Check the content of the review does not exceed the maximum length
    let total_len = MovieAccountState::get_account_size(&title, &description);
    if total_len > MovieAccountState::MAX_ACCOUNT_SIZE {
        msg!("Input data exceeds max length");
        return Err(ReviewError::InvalidDataLength.into());
    }

    msg!("creating review pda account");
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(MovieAccountState::MAX_ACCOUNT_SIZE);

    let create_account = system_instruction::create_account(
        reviewer.key,
        pda_review.key,
        rent_lamports,
        MovieAccountState::MAX_ACCOUNT_SIZE.try_into().unwrap(),
        program_id,
    );
    // Create the account CPI
    invoke_signed(
        &create_account,
        &[reviewer.clone(), pda_review.clone(), system_program.clone()],
        &[&[reviewer.key.as_ref(), title.as_bytes(), &[bump_seed]]],
    )?;
    msg!("review PDA created at: {}", pda_review_key);

    let mut account_data =
        try_from_slice_unchecked::<MovieAccountState>(&pda_review.data.borrow())?;

    msg!("checking if account is initialized");
    if account_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    account_data.discriminator = MovieAccountState::DISCRIMINATOR.to_string();
    account_data.reviewer = *reviewer.key;
    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    msg!("serializing account");
    account_data.serialize(&mut *pda_review.data.borrow_mut())?;
    msg!("state account serialized");

    let (pda_counter_key, counter_bump_seed) =
        Pubkey::find_program_address(&[pda_review.key.as_ref(), "comment".as_ref()], program_id);

    if pda_comment_counter.key != &pda_counter_key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    msg!("creating comment counter");
    let rent = Rent::get()?;
    let counter_rent_lamports = rent.minimum_balance(MovieCommentCounter::get_account_size());

    let create_pda_counter = system_instruction::create_account(
        reviewer.key,
        pda_comment_counter.key,
        counter_rent_lamports,
        MovieCommentCounter::get_account_size().try_into().unwrap(),
        program_id,
    );

    invoke_signed(
        &create_pda_counter,
        &[
            reviewer.clone(),
            pda_comment_counter.clone(),
            system_program.clone(),
        ],
        &[&[
            pda_review.key.as_ref(),
            "comment".as_ref(),
            &[counter_bump_seed],
        ]],
    )?;
    msg!("comment counter PDA created at: {}", pda_counter_key);

    let mut counter_data =
        try_from_slice_unchecked::<MovieCommentCounter>(&pda_comment_counter.data.borrow())?;

    if counter_data.is_initialized() {
        msg!("Counter account already initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    msg!("initializing counter account");
    counter_data.discriminator = MovieCommentCounter::DISCRIMINATOR.to_string();
    counter_data.counter = 0;
    counter_data.is_initialized = true;

    counter_data.serialize(&mut *pda_comment_counter.data.borrow_mut())?;
    msg!("counter account initialized");

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

    let total_len = MovieAccountState::get_account_size(&title, &description);
    if total_len > MovieAccountState::MAX_ACCOUNT_SIZE {
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

pub fn add_comment(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    comment: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let commenter = next_account_info(account_info_iter)?;
    let pda_review = next_account_info(account_info_iter)?;
    let pda_counter = next_account_info(account_info_iter)?;
    let pda_comment = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;

    let mut counter_data =
        try_from_slice_unchecked::<MovieCommentCounter>(&pda_counter.data.borrow()).unwrap();

    // Check if counter_data is initialized
    if !counter_data.is_initialized() {
        msg!("Counter account not initialized yet");
        return Err(ReviewError::UninitializedAccount.into());
    }

    // Check if the instruction is signed
    if !commenter.is_signer {
        msg!("Missing required signature");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Check if the account owner is the program
    if pda_review.owner != program_id {
        msg!("Invalid account owner");
        return Err(ProgramError::IllegalOwner);
    }

    let (pda, bump_seed) = Pubkey::find_program_address(
        &[
            pda_review.key.as_ref(),
            counter_data.counter.to_be_bytes().as_ref(),
        ],
        program_id,
    );

    if pda != *pda_comment.key {
        msg!("Invalid seeds for PDA");
        return Err(ReviewError::InvalidPDA.into());
    }

    let total_len = MovieComment::get_account_size(&comment);
    if total_len > MovieComment::MAX_ACCOUNT_SIZE {
        msg!("Input data exceeds max length");
        return Err(ReviewError::InvalidDataLength.into());
    }

    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(MovieComment::MAX_ACCOUNT_SIZE);

    let create_pda_comment = system_instruction::create_account(
        commenter.key,
        pda_comment.key,
        rent_lamports,
        MovieComment::MAX_ACCOUNT_SIZE.try_into().unwrap(),
        program_id,
    );

    invoke_signed(
        &create_pda_comment,
        &[
            commenter.clone(),
            pda_comment.clone(),
            system_program.clone(),
        ],
        &[&[
            pda_review.key.as_ref(),
            counter_data.counter.to_le_bytes().as_ref(),
            &[bump_seed],
        ]],
    )?;
    msg!("comment PDA created: {}", pda);

    let mut comment_data = try_from_slice_unchecked::<MovieComment>(&pda_comment.data.borrow())?;

    msg!("checking if comment account is initialized");
    if comment_data.is_initialized() {
        msg!("Account already initialized");
        return Err(ReviewError::UninitializedAccount.into());
    }

    comment_data.discriminator = MovieComment::DISCRIMINATOR.to_string();
    comment_data.is_initialized = true;
    comment_data.reviewer = *pda_review.key;
    comment_data.commenter = *commenter.key;
    comment_data.comment = comment;
    comment_data.count = counter_data.counter;

    comment_data.serialize(&mut *pda_comment.data.borrow_mut())?;

    msg!("incrementing counter");
    counter_data.counter += 1;
    counter_data.serialize(&mut *pda_counter.data.borrow_mut())?;

    Ok(())
}
