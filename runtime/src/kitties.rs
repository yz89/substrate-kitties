use parity_codec::{Decode, Encode};
use rstd::result;
use runtime_io::blake2_128;
use runtime_primitives::traits::{Bounded, One, SimpleArithmetic};
use support::{
    decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap, StorageValue,
};
use system::ensure_signed;

pub trait Trait: system::Trait {
    type KittyIndex: Parameter + Default + SimpleArithmetic + Bounded + Copy;
}

#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

decl_storage! {
    trait Store for Module<T: Trait> as Kitties {
        /// Stores all the kitties, key is the kitty id / index
        pub Kitties get(kitty): map T::KittyIndex => Option<Kitty>;
        /// Stores the total number of kitties. i.e. the next kitty index
        pub KittiesCount get(kitties_count): T::KittyIndex;
        /// Get kitty owner by kitty index
        pub OwnedKitties get(owned_kitties): map T::KittyIndex => T::AccountId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Create a new kitty
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;
            let kitty_id = Self::next_kitty_id()?;

            // Generate a random 128 bit value
            let dna = Self::random_value(&sender);

            // Create and store kitty
            let kitty = Kitty(dna);
            Self::insert_kitty(sender, kitty_id, kitty);
        }

        /// Breed kitties
        pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
            let sender = ensure_signed(origin)?;
            Self::do_breed(sender, kitty_id_1, kitty_id_2)?;
        }

        /// Transfer kitty
        pub fn transfer(origin, recipient: T::AccountId, kitty_id: T::KittyIndex) {
            let sender = ensure_signed(origin)?;
            Self::do_transfer(sender, recipient, kitty_id)?;
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    ((selector & dna1) | (!selector & dna2))
}

impl<T: Trait> Module<T> {
    fn random_value(sender: &T::AccountId) -> [u8; 16] {
        let payload = (
            <system::Module<T>>::random_seed(),
            sender.clone(),
            <system::Module<T>>::extrinsic_index(),
            <system::Module<T>>::block_number(),
        );
        payload.using_encoded(blake2_128)
    }

    fn next_kitty_id() -> result::Result<T::KittyIndex, &'static str> {
        let kitty_id = Self::kitties_count();
        if kitty_id == T::KittyIndex::max_value() {
            return Err("Kitties count overflow");
        }
        Ok(kitty_id)
    }

    fn insert_kitty(owner: T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) {
        // Create and store kitty
        <Kitties<T>>::insert(kitty_id, kitty);
        <KittiesCount<T>>::put(kitty_id + One::one());

        // Store the ownership information
        <OwnedKitties<T>>::insert(kitty_id, owner);
    }

    fn do_breed(
        sender: T::AccountId,
        kitty_id_1: T::KittyIndex,
        kitty_id_2: T::KittyIndex,
    ) -> Result {
        let kitty1 = Self::kitty(kitty_id_1);
        let kitty2 = Self::kitty(kitty_id_2);

        ensure!(kitty1.is_some(), "Invalid kitty_id_1");
        ensure!(kitty2.is_some(), "Invalid kitty_id_2");
        ensure!(kitty_id_1 != kitty_id_2, "Needs different parents");

        let new_kitty_id = Self::next_kitty_id()?;

        let kitty1_dna = kitty1.unwrap().0;
        let kitty2_dna = kitty2.unwrap().0;
        let selector = Self::random_value(&sender);

        let mut new_dna = [0u8; 16];
        for i in 0..kitty1_dna.len() {
            new_dna[i] = combine_dna(kitty1_dna[i], kitty2_dna[i], selector[i]);
        }

        let new_kitty = Kitty(new_dna);
        Self::insert_kitty(sender, new_kitty_id, new_kitty);
        Ok(())
    }

    fn do_transfer(
        sender: T::AccountId,
        recipient: T::AccountId,
        kitty_id: T::KittyIndex,
    ) -> Result {
        // Check if the kitty exsit
        let transfer_kitty = Self::kitty(kitty_id);
        ensure!(transfer_kitty.is_some(), "Invalid transfer kitty");

        // Check if the sender own this kitty
        ensure!(Self::owned_kitties(kitty_id) == sender, "Sender must own the transfer kitty");

        // Store the ownership information
        <OwnedKitties<T>>::insert(kitty_id, recipient);

        Ok(())
    }
}
