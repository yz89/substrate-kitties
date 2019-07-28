use support::{StorageMap, Parameter};
use runtime_primitives::traits::Member;
use parity_codec::{Encode, Decode, Input, Output};

#[cfg_attr(feature = "std", derive(Debug, PartialEq, Eq))]
pub struct LinkedItem<Item> {
	pub prev: Option<Item>,
	pub next: Option<Item>,
}

impl<Item> Encode for LinkedItem<Item> where Item: Encode + Default + Copy {
    fn encode_to<T: Output>(&self, output: &mut T) {
       self.prev.unwrap_or_default().encode_to(output);
	   self.next.unwrap_or_default().encode_to(output);
    }
}

impl<Item> Decode for LinkedItem<Item> where Item: Decode {
    fn decode<I: Input>(input: &mut I) -> Option<Self> {
		Some(
			LinkedItem{
				prev: Decode::decode(input)?,
				next: Decode::decode(input)?,
			}
		)
    }
}

pub struct LinkedList<Storage, Key, Item>(rstd::marker::PhantomData<(Storage, Key, Item)>);

impl<Storage, Key, Value> LinkedList<Storage, Key, Value> where
  Value: Parameter + Member + Copy + Default,
  Key: Parameter,
  Storage: StorageMap<(Key, Option<Value>), LinkedItem<Value>, Query = Option<LinkedItem<Value>>>,
{
	fn read_head(key: &Key) -> LinkedItem<Value> {
		Self::read(key, None)
	}

	fn write_head(account: &Key, item: LinkedItem<Value>) {
		Self::write(account, None, item);
	}

	fn read(key: &Key, value: Option<Value>) -> LinkedItem<Value> {
		Storage::get(&(key.clone(), value)).unwrap_or_else(|| LinkedItem {
			prev: None,
			next: None,
		})
	}

	fn write(key: &Key, value: Option<Value>, item: LinkedItem<Value>) {
		Storage::insert(&(key.clone(), value), item);
	}

	pub fn append(key: &Key, value: Value) {
		let head = Self::read_head(key);
		let new_head = LinkedItem {
			prev: Some(value),
			next: head.next,
		};

		Self::write_head(key, new_head);

		let prev = Self::read(key, head.prev);
		let new_prev = LinkedItem {
			prev: prev.prev,
			next: Some(value),
		};
		Self::write(key, head.prev, new_prev);

		let item = LinkedItem {
			prev: head.prev,
			next: None,
		};
		Self::write(key, Some(value), item);
	}

	pub fn remove(key: &Key, value: Value) {
		if let Some(item) = Storage::take(&(key.clone(), Some(value))) {
			let prev = Self::read(key, item.prev);
			let new_prev = LinkedItem {
				prev: prev.prev,
				next: item.next,
			};

			Self::write(key, item.prev, new_prev);

			let next = Self::read(key, item.next);
			let new_next = LinkedItem {
				prev: item.prev,
				next: next.next,
			};

			Self::write(key, item.next, new_next);
		}
	}
}