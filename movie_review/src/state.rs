use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{IsInitialized, Sealed};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct MovieAccountState {
    pub discriminator: String,
    pub is_initialized: bool,
    pub reviewer: Pubkey,
    pub rating: u8,
    pub title: String,
    pub description: String,
}

impl Sealed for MovieAccountState {}

impl IsInitialized for MovieAccountState {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl MovieAccountState {
    pub const DISCRIMINATOR: &'static str = "review";
    // pub const MAX_TITLE_LEN: usize = 100;
    // pub const MAX_DESCRIPTION_LEN: usize = 1000;
    pub const MAX_ACCOUNT_SIZE: usize = 1000;

    pub fn get_account_size(title: &str, description: &str) -> usize {
        (4 + MovieAccountState::DISCRIMINATOR.len())
            + 1
            + 1
            + (4 + title.len())
            + (4 + description.len())
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct MovieCommentCounter {
    pub discriminator: String,
    pub is_initialized: bool,
    pub counter: u64,
}

impl IsInitialized for MovieCommentCounter {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl MovieCommentCounter {
    pub const DISCRIMINATOR: &'static str = "counter";

    pub fn get_account_size() -> usize {
        (4 + MovieCommentCounter::DISCRIMINATOR.len()) + 1 + 8
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct MovieComment {
    pub discriminator: String,
    pub is_initialized: bool,
    pub reviewer: Pubkey,
    pub commenter: Pubkey,
    pub comment: String,
    pub count: u64,
}

impl IsInitialized for MovieComment {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl MovieComment {
    pub const DISCRIMINATOR: &'static str = "comment";
    // pub const MAX_COMMENT_LEN: usize = 1000;
    pub const MAX_ACCOUNT_SIZE: usize = 1000;

    pub fn get_account_size(comment: &str) -> usize {
        (4 + MovieComment::DISCRIMINATOR.len()) + 1 + 32 + 32 + (4 + comment.len()) + 8
    }
}
