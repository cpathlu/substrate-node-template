#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

mod mock;
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{ensure, pallet_prelude::*};
	use frame_support::{
		sp_runtime::traits::{AtLeast32BitUnsigned, Bounded, CheckedAdd,One},
		traits::{Currency, Randomness, tokens::ExistenceRequirement,ReservableCurrency},
		transactional,
	};
	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The Currency handler for the Kitties pallet.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		//type KittyIndex: u32;
		//type KittyIndex: Parameter + Member + AtLeast32Bit + Bounded + Default + Copy;
		type KittyIndex: Parameter + AtLeast32BitUnsigned + Default + Copy + Bounded + CheckedAdd + Sized;


		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;

		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type ReserveForCreateKitty: Get<BalanceOf<Self>>;
	}

	// Errors.
	#[pallet::error]
	pub enum Error<T> {
		KittyNotExist,
		KittyCntOverflow,
		ExceedMaxKittyOwned,
		BuyerIsKittyOwner,
		KittyBidPriceTooLow,
		NotKittyOwner,
		TransferToSelf,
		NotEnoughBalance,
		KittyNotForSale,
		BreedSameParent,
		InvalidReserveAmount,
	}

	// Events.
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created(T::AccountId, T::KittyIndex),
		PriceSet(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::KittyIndex),
		Bought(T::AccountId, T::AccountId, T::KittyIndex, BalanceOf<T>),
	}

	// #[pallet::storage]
	// #[pallet::getter(fn kitty_cnt)]
	// pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, T::KittyIndex>;

	/// Keeps track of the number of Kitties in existence.
	#[pallet::storage]
	#[pallet::getter(fn kitty_cnt)]
	pub(super) type KittyCnt<T: Config> = StorageValue<_, T::KittyIndex>;

	/// Stores a Kitty's unique traits, owner and price.
	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub(super) type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Kitty<T>>;

	/// Keeps track of what accounts own what Kitties.
	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<T::KittyIndex, T::MaxKittyOwned>,
		ValueQuery,
	>;


	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub kitties: Vec<(T::AccountId, [u8; 16], Gender)>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { kitties: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (acct, dna, gender) in &self.kitties {
				let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(gender.clone()));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			T::Currency::reserve(&sender, T::ReserveForCreateKitty::get()).map_err(|_| Error::<T>::InvalidReserveAmount)?;

			let kitty_id = Self::mint(&sender, None, None)?;

			Self::deposit_event(Event::Created(sender, kitty_id));

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id , &sender)?,<Error<T>>::NotKittyOwner);

			let mut kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			kitty.price = new_price.clone();

			<Kitties<T>>::insert(&kitty_id, kitty);

			Self::deposit_event(Event::PriceSet(sender, kitty_id, new_price));

			Ok(())
		}

		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: T::AccountId,
			kitty_id: T::KittyIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::is_kitty_owner(&kitty_id,&sender)?, <Error<T>>::NotKittyOwner);

			ensure!(sender != dest ,  <Error<T>>::TransferToSelf);

			let dest_owned = <KittiesOwned<T>>::get(&dest);

			ensure!((dest_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			Self::transfer_kitty_to(&kitty_id, &dest)?;

			Self::deposit_event(Event::Transferred(sender, dest, kitty_id));

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: T::KittyIndex,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			let kitty = Self::kitties(&kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			ensure!(kitty.owner != buyer, <Error<T>>::BuyerIsKittyOwner);

			if let Some(ask_price) = kitty.price {
				ensure!(ask_price <= bid_price , <Error<T>>::KittyBidPriceTooLow)
			} else {
				Err(<Error<T>>::KittyNotForSale)?;
			}

			ensure!(T::Currency::free_balance(&buyer) >= bid_price , <Error<T>>::NotEnoughBalance);

			let to_owned = <KittiesOwned<T>>::get(&buyer);

			ensure!((to_owned.len() as u32) < T::MaxKittyOwned::get(), <Error<T>>::ExceedMaxKittyOwned);

			let seller = kitty.owner.clone();

			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			Self::transfer_kitty_to(&kitty_id, &buyer)?;

			Self::deposit_event(Event::Bought(buyer, seller, kitty_id, bid_price));
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100)]
		pub fn breed_kitty(
			origin: OriginFor<T>,
			parent1: T::KittyIndex,
			parent2: T::KittyIndex,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_kitty_owner(&parent1 , &sender)?, <Error<T>>::NotKittyOwner);
			ensure!(Self::is_kitty_owner(&parent2 , &sender)?, <Error<T>>::NotKittyOwner);

			let new_dna = Self::breed_dna(&parent1, &parent2)?;

			Self::mint(&sender, Some(new_dna), None)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		pub fn gen_dna() -> [u8; 16] {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		pub fn breed_dna(parent1: &T::KittyIndex, parent2: &T::KittyIndex) -> Result<[u8; 16], Error<T>> {
			let dna1 = Self::kitties(parent1).ok_or(<Error<T>>::KittyNotExist)?.dna;
			let dna2 = Self::kitties(parent2).ok_or(<Error<T>>::KittyNotExist)?.dna;

			let mut new_dna = Self::gen_dna();
			for i in 0..new_dna.len() {
				new_dna[i] = (new_dna[i] & dna1[i]) | (!new_dna[i] & dna2[i]);
			}
			Ok(new_dna)
		}

		pub fn is_kitty_owner(kitty_id: &T::KittyIndex, acct: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::kitties(kitty_id) {
				Some(kitty) => Ok(kitty.owner == *acct),
				None => Err(<Error<T>>::KittyNotExist),
			}
		}

		pub fn mint(
			owner: &T::AccountId,
			dna: Option<[u8; 16]>,
			gender: Option<Gender>,
		) -> Result<T::KittyIndex, Error<T>> {
			let kitty = Kitty::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				gender: gender.unwrap_or_else(Self::gen_gender),
				owner: owner.clone(),
			};


			// let new_cnt = Self::kitty_cnt().checked_add(1).ok_or(<Error<T>>::KittyCntOverflow)?;

			let new_cnt = match Self::kitty_cnt() {
				Some(cnt) => {
					cnt.checked_add(&One::one()).ok_or(<Error<T>>::KittyCntOverflow)?
				}
				None => One::one(),
			};

			// let kitty_id = match Self::kitty_cnt() {
			// 	Some(id) => {
			// 		ensure!(id != T::KittyIndex::max_value(),<Error<T>>::KittyCntOverflow);
			// 		id
			// 	}
			// 	None => {
			// 		1u32.into()
			// 	}
			// };
			// let new_cnt = Self::kitty_cnt().unwrap().checked_add(&One::one()).ok_or(<Error<T>>::KittyCntOverflow)?;

			let kitty_id = new_cnt;

			<KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| kitty_vec.try_push(kitty_id))
				.map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			<Kitties<T>>::insert(kitty_id, kitty);

			<KittyCnt<T>>::put(kitty_id);

			Ok(kitty_id)
		}

		#[transactional]
		pub fn transfer_kitty_to(
			kitty_id: &T::KittyIndex,
			dest: &T::AccountId,
		) -> Result<(), Error<T>> {
			let mut kitty = Self::kitties(kitty_id).ok_or(<Error<T>>::KittyNotExist)?;

			let prev_owner = kitty.owner.clone();

			<KittiesOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *kitty_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			}).map_err(|_| <Error<T>>::KittyNotExist)?;

			kitty.owner = dest.clone();

			<Kitties<T>>::insert(kitty_id, kitty);

			<KittiesOwned<T>>::try_mutate(dest, |vec| {
				vec.try_push(*kitty_id)
			}).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

			Ok(())
		}
	}
}
