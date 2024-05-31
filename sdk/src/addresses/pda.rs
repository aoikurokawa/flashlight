use solana_sdk::pubkey::Pubkey;

pub fn get_user_account_pubkey(
    program_id: &Pubkey,
    authority: Pubkey,
    sub_account_id: Option<u64>,
) -> Pubkey {
    let sub_account_id = sub_account_id.unwrap_or(0);

    Pubkey::find_program_address(
        &[b"user", authority.as_ref(), &sub_account_id.to_le_bytes()],
        program_id,
    )
    .0
}

pub fn get_user_stats_account_pubkey(program_id: &Pubkey, authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"user_stats", authority.as_ref()], program_id).0
}
