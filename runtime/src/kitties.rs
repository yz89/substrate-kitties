use parity_codec::{Decode, Encode};
use rstd::result;
use runtime_io::blake2_128;
use runtime_primitives::traits::{Bounded, Member, One, SimpleArithmetic};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Currency, Parameter,
    StorageMap, StorageValue,
};
use system::ensure_signed;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type KittyIndex: Parameter + Member + Default + SimpleArithmetic + Bounded + Copy;
    type Currency: Currency<Self::AccountId>;
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

#[derive(Encode, Decode)]
pub struct Kitty(pub [u8; 16]);

#[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq))]
#[derive(Encode, Decode)]
pub struct KittyLinkedItem<T: Trait> {
    pub prev: Option<T::KittyIndex>,
    pub next: Option<T::KittyIndex>,
}

decl_storage! {
    trait Store for Module<T: Trait> as Kitties {
        /// Stores all the kitties, key is the kitty id / index
        pub Kitties get(kitty): map T::KittyIndex => Option<Kitty>;
        /// Stores the total number of kitties. i.e. the next kitty index
        pub KittiesCount get(kitties_count): T::KittyIndex;
        /// Get kitty ownership. Stored in a linked map.
        pub OwnedKitties get(owned_kitties): map (T::AccountId, Option<T::KittyIndex>) =>
        Option<KittyLinkedItem<T>>;

        pub KittyOwners get(kitty_owner): map T::KittyIndex => Option<T::AccountId>;
        pub KittyPrices get(kitty_price): map T::KittyIndex => Option<BalanceOf<T>>;
    }
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::KittyIndex,
		Balance = BalanceOf<T>,
	{
		/// A kitty is created. (owner, kitty_id)
		Created(AccountId, KittyIndex),
		/// A kitty is transferred. (from, to, kitty_id)
		Transferred(AccountId, AccountId, KittyIndex),
		/// A kitty is available for sale. (owner, kitty_id, price)
		Ask(AccountId, KittyIndex, Option<Balance>),
		/// A kitty is sold. (from, to, kitty_id, price)
		Sold(AccountId, AccountId, KittyIndex, Balance),
	}
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        /// Create a new kitty
        pub fn create(origin) {
            let sender = ensure_signed(origin)?;
            let kitty_id = Self::next_kitty_id()?;

            // Generate a random 128 bit value
            let dna = Self::random_value(&sender);

            // Create and store kitty
            let kitty = Kitty(dna);
            Self::insert_kitty(&sender, kitty_id, kitty);

            Self::deposit_event(RawEvent::Created(sender, kitty_id));
        }

        /// Breed kitties
        pub fn breed(origin, kitty_id_1: T::KittyIndex, kitty_id_2: T::KittyIndex) {
            let sender = ensure_signed(origin)?;
            let new_kitty_id = Self::do_breed(&sender, kitty_id_1, kitty_id_2)?;

            Self::deposit_event(RawEvent::Created(sender, new_kitty_id));
        }

        /// Transfer kitty
        pub fn transfer(origin, recipient: T::AccountId, kitty_id: T::KittyIndex) {
            let sender = ensure_signed(origin)?;
            Self::do_transfer(&sender, &recipient, kitty_id)?;

            Self::deposit_event(RawEvent::Transferred(sender, recipient, kitty_id));
        }

        pub fn ask(origin, kitty_id: T::KittyIndex, price: Option<BalanceOf<T>>) {
            let sender = ensure_signed(origin)?;

            ensure!(<OwnedKitties<T>>::exists(&(sender.clone(), Some(kitty_id))), "Only owner can set price for kitty");

            if let Some(price) = price {
                <KittyPrices<T>>::insert(kitty_id, price);
            } else {
                <KittyPrices<T>>::remove(kitty_id);
            }

            Self::deposit_event(RawEvent::Ask(sender, kitty_id, price));
        }

        pub fn buy(origin, kitty_id: T::KittyIndex, price: BalanceOf<T>) {
            let sender = ensure_signed(origin)?;

            let owner = Self::kitty_owner(kitty_id);
            ensure!(owner.is_some(), "Kitty does not exist");
            let owner = owner.unwrap();

            let kitty_price = Self::kitty_price(kitty_id);
            ensure!(kitty_price.is_some(), "Kitty not for sale");
            let kitty_price = kitty_price.unwrap();

            ensure!(price >= kitty_price, "Price is too low");

            T::Currency::transfer(&sender, &owner, kitty_price)?;

            <KittyPrices<T>>::remove(kitty_id);

            <OwnedKitties<T>>::remove(&owner, kitty_id);
            <OwnedKitties<T>>::append(&sender, kitty_id);
            <KittyOwners<T>>::insert(kitty_id, &sender);

            Self::deposit_event(RawEvent::Sold(owner, sender, kitty_id, kitty_price));
        }
    }
}

fn combine_dna(dna1: u8, dna2: u8, selector: u8) -> u8 {
    ((selector & dna1) | (!selector & dna2))
}

impl<T: Trait> OwnedKitties<T> {
    fn read_head(account: &T::AccountId) -> KittyLinkedItem<T> {
        Self::read(account, None)
    }

    fn write_head(account: &T::AccountId, item: KittyLinkedItem<T>) {
        Self::write(account, None, item)
    }

    fn read(account: &T::AccountId, key: Option<T::KittyIndex>) -> KittyLinkedItem<T> {
        <OwnedKitties<T>>::get(&(account.clone(), key)).unwrap_or_else(|| KittyLinkedItem {
            prev: None,
            next: None,
        })
    }

    fn write(account: &T::AccountId, key: Option<T::KittyIndex>, item: KittyLinkedItem<T>) {
        <OwnedKitties<T>>::insert(&(account.clone(), key), item);
    }

    fn append(account: &T::AccountId, kitty_id: T::KittyIndex) {
        let head = Self::read_head(account);
        let new_head = KittyLinkedItem {
            prev: Some(kitty_id),
            next: head.next,
        };
        Self::write_head(account, new_head);

        let prev = Self::read(account, head.prev);
        let new_prev = KittyLinkedItem {
            prev: prev.prev,
            next: Some(kitty_id),
        };
        Self::write(account, head.prev, new_prev);

        let item = KittyLinkedItem {
            prev: head.prev,
            next: None,
        };
        Self::write(account, Some(kitty_id), item);
    }

    fn remove(account: &T::AccountId, kitty_id: T::KittyIndex) {
        if let Some(item) = <OwnedKitties<T>>::take(&(account.clone(), Some(kitty_id))) {
            let prev = Self::read(account, item.prev);
            let new_prev = KittyLinkedItem {
                prev: prev.prev,
                next: item.next,
            };
            Self::write(account, item.prev, new_prev);

            let next = Self::read(account, item.next);
            let new_next = KittyLinkedItem {
                prev: item.prev,
                next: next.next,
            };
            Self::write(account, item.next, new_next);
        }
    }
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

    fn insert_owned_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex) {
        <OwnedKitties<T>>::append(owner, kitty_id);
    }

    fn insert_kitty(owner: &T::AccountId, kitty_id: T::KittyIndex, kitty: Kitty) {
        // Create and store kitty
        <Kitties<T>>::insert(kitty_id, kitty);
        <KittiesCount<T>>::put(kitty_id + One::one());
        <KittyOwners<T>>::insert(kitty_id, owner);

        Self::insert_owned_kitty(owner, kitty_id);
    }

    fn do_breed(
        sender: &T::AccountId,
        kitty_id_1: T::KittyIndex,
        kitty_id_2: T::KittyIndex,
    ) -> result::Result<T::KittyIndex, &'static str> {
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
        Ok(new_kitty_id)
    }

    fn do_transfer(
        sender: &T::AccountId,
        recipient: &T::AccountId,
        kitty_id: T::KittyIndex,
    ) -> Result {
        // Check if the kitty exsit
        let transfer_kitty = Self::kitty(kitty_id);
        ensure!(transfer_kitty.is_some(), "Invalid transfer kitty");

        // Check if the sender own this kitty
        ensure!(
            <OwnedKitties<T>>::exists(&(sender.clone(), Some(kitty_id))),
            "Only owner can transfer kitty"
        );

        <OwnedKitties<T>>::remove(&sender, kitty_id);
        <OwnedKitties<T>>::append(&recipient, kitty_id);
        <KittyOwners<T>>::insert(kitty_id, recipient);
        Ok(())
    }
}

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher, H256};
    use runtime_io::with_externalities;
    use runtime_primitives::{
        testing::{Digest, DigestItem, Header},
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_ok, impl_outer_origin};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq, Debug)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    impl Trait for Test {
        type KittyIndex = u32;
    }
    type KittyModule = Module<Test>;
    type OwnedKittiesTest = OwnedKitties<Test>;

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0
            .into()
    }

    #[test]
    fn owned_kitties_can_append_values() {
        with_externalities(&mut new_test_ext(), || {
            OwnedKittiesTest::append(&0, 1);

            assert_eq!(
                OwnedKittiesTest::get(&(0, None)),
                Some(KittyLinkedItem {
                    prev: Some(1),
                    next: Some(1),
                })
            );

            println!("{:?}", OwnedKittiesTest::get(&(0, None)));

            assert_eq!(
                OwnedKittiesTest::get(&(0, Some(1))),
                Some(KittyLinkedItem {
                    prev: None,
                    next: None,
                })
            );

            OwnedKittiesTest::append(&0, 2);

            assert_eq!(
                OwnedKittiesTest::get(&(0, None)),
                Some(KittyLinkedItem {
                    prev: Some(2),
                    next: Some(1),
                })
            );

            assert_eq!(
                OwnedKittiesTest::get(&(0, Some(1))),
                Some(KittyLinkedItem {
                    prev: None,
                    next: Some(2),
                })
            );

            assert_eq!(
                OwnedKittiesTest::get(&(0, Some(2))),
                Some(KittyLinkedItem {
                    prev: Some(1),
                    next: None,
                })
            );
        });
    }
}
