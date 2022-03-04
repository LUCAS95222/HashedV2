
use crate::state::NftLockContract;
use cosmwasm_std::Addr;


impl<'a> NftLockContract<'a> {
  pub fn gen_key(&self, nft_address: Addr, token_id: String) -> Vec<u8> {
    [nft_address.as_bytes(), token_id.as_bytes()].concat()
  }
}