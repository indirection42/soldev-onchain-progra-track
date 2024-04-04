use borsh::BorshDeserialize;
use solana_program::program_error::ProgramError;

pub enum MovieInstruction {
    AddMovieReview {
        title: String,
        rating: u8,
        description: String,
    },
}

#[derive(BorshDeserialize)]
struct MovieReviewPayload {
    title: String,
    rating: u8,
    description: String,
}

impl MovieInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&variant, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;
        let instruction = MovieReviewPayload::try_from_slice(rest).unwrap();
        Ok(match variant {
            0 => MovieInstruction::AddMovieReview {
                title: instruction.title,
                rating: instruction.rating,
                description: instruction.description,
            },
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}
