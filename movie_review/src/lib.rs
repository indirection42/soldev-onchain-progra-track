use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

pub mod instruction;
use instruction::MovieInstruction;

pub mod state;
use state::MovieAccountState;

entrypoint!(process_instruction);

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

    // Since we need bump_seed, we still derive pda instead of using pda_account directly
    let (pda, bump_seed) =
        Pubkey::find_program_address(&[initializer.key.as_ref(), title.as_bytes()], program_id);

    // 1 byte each for rating and is_initialized
    let account_len = 1 + 1 + (4 + title.len()) + (4 + description.len());

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

    account_data.title = title;
    account_data.rating = rating;
    account_data.description = description;
    account_data.is_initialized = true;

    msg!("serializing account");
    account_data.serialize(&mut *pda_account.data.borrow_mut())?;
    msg!("state account serialized");

    Ok(())
}
