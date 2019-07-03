use support::{decl_module, decl_storage, StorageValue, StorageMap};
use parity_codec::{Encode, Decode};
use runtime_io::blake2_128;
use system::ensure_signed;

pub trait Trait: system::Trait {

}

#[derive(Encode, Decode, Default)]
pub struct Kitty(pub [u8; 16]);

decl_storage! {
	trait Store for Module<T: Trait> as Kitties {
        pub Kitties get(kitty): map u32 => Kitty;
        pub KittiesCount get(kitties_count): u32;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;
            let payload = (
                <system::Module<T>>::random_seed(),
                sender,
                <system::Module<T>>::extrinsic_index(),
                <system::Module<T>>::block_number()
            );
            let dna = payload.using_encoded(blake2_128);
            let kitty = Kitty(dna);
            let count = Self::kitties_count();
            <Kitties<T>>::insert(count, kitty);
            <KittiesCount<T>>::put(count + 1);
        }
    }
}