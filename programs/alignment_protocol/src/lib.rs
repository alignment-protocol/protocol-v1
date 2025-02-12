use anchor_lang::prelude::*;

declare_id!("FtHfXYCwuVEb8LVkNwNMmqMVooyg2fxkoT8i9bkEcvKW");

#[program]
pub mod alignment_protocol {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
