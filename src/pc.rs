//AccountInfo expected for pushing to Incept:
//src_info for Oracle, initalizer
//dst_info for Amm
//Rent sysvar
//token program
use crate::*;

//Price in i64, confidence in u64
#[repr(C)]
pub struct Oracle_Price {
    pub price : i64,
    pub conf  : u64,
    pub status : PriceStatus,
    pub corp_act : CorpAction,

}




//Impl for u64 repersentation for abstraction to program account,
//Derived from Serum
impl Oracle_Price {
    #[inline]
    pub fn load<'a>(price_feed: &'a AccountInfo) -> Result<RefMut<'a, PriceInfo>, ProgramError> {
        let account_data: RefMut<'a, [u64]>;
        let state: RefMut<'a, Self>;
        //memory safety pattern here to unwrap to
        account_data = RefMut::map(price_feed.try_borrow_mut_data().unwrap(), |data| *data);

        state = RefMut::map(account_data, |data| {
            from_bytes_mut(cast_slice)})
    }
}
