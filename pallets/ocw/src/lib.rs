#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::sp_runtime::{
	offchain as rt_offchain,
	offchain::{
		http,
		storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
	},
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};
use frame_support::traits::Get;
use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
};
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use sp_std::vec::Vec;

#[cfg(test)]
mod tests;

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"dot!");

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrappers.
/// We can use from supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// the types with this pallet-specific identifier.
pub mod crypto {
	use super::KEY_TYPE;
	use frame_support::sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	use sp_core::sr25519::Signature as Sr25519Signature;
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	//use core::{convert::TryInto, fmt};
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;

		// Configuration parameters

		/// A grace period after we send transaction.
		///
		/// To avoid sending too many transactions, we only attempt to send one
		/// every `GRACE_PERIOD` blocks. We use Local Storage to coordinate
		/// sending between distinct runs of this offchain worker.
		#[pallet::constant]
		type GracePeriod: Get<Self::BlockNumber>;

		/// Number of blocks of cooldown after unsigned transaction is included.
		///
		/// This ensures that we only accept unsigned transactions once, every `UnsignedInterval`
		/// blocks.
		#[pallet::constant]
		type UnsignedInterval: Get<Self::BlockNumber>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}
	use serde::{Deserialize, Deserializer};
	use sp_arithmetic::per_things::Permill;
	//use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};
	use sp_std::{prelude::*, str};

	// const HTTP_REMOTE_REQUEST: &str = "https://api.coincap.io/v2/assets/polkadot";
	// const HTTP_HEADER_USER_AGENT: &str = "cc";

	// const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds
	// const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
	// const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

	pub type DotPrice = (u64, Permill);

	#[derive(Deserialize, Encode, Decode, Default)]
    #[allow(non_snake_case)]
	struct DotPriceInfo {
		#[serde(deserialize_with = "string_to_bytes")]
		id: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		rank: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		name: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		marketCapUsd: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		volumeUsd24Hr: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		priceUsd: Vec<u8>,
		#[serde(deserialize_with = "string_to_bytes")]
		changePercent24Hr: Vec<u8>,
	}

	#[derive(Debug, Deserialize, Encode, Decode, Default)]
	struct IndexingData(Vec<u8>, u64);

	pub fn string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	pub fn ç<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s: &str = Deserialize::deserialize(de)?;
		Ok(s.as_bytes().to_vec())
	}

	// impl fmt::Debug for DotPriceInfo {
	// 	// `fmt` converts the vector of bytes inside the struct back to string for
	// 	//   more friendly display.
	// 	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	// 		write!(
	// 			f,
	// 			"{{ login: {}, blog: {}, public_repos: {} }}",
	// 			str::from_utf8(&self.rank).map_err(|_| fmt::Error)?,
	// 			str::from_utf8(&self.priceUsd).map_err(|_| fmt::Error)?,
	// 			&self.priceUsd
	// 		)
	// 	}
	// }

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Error returned when not sure which ocw function to executed
		UnknownOffchainMux,

		// Error returned when making signed transactions in off-chain worker
		NoLocalAcctForSigning,
		OffchainSignedTxError,

		// Error returned when making unsigned transactions in off-chain worker
		OffchainUnsignedTxError,

		// Error returned when making unsigned transactions with signed payloads in off-chain worker
		OffchainUnsignedTxSignedPayloadError,

		// Error returned when fetching github info
		HttpFetchingError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// succesfuly imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		/// You can use `Local Storage` API to coordinate runs of the worker.
		fn offchain_worker(block_number: T::BlockNumber) {
			// Note that having logs compiled to WASM may cause the size of the blob to increase
			// significantly. You can use `RuntimeDebug` custom derive to hide details of the types
			// in WASM. The `sp-api` crate also provides a feature `disable-logging` to disable
			// all logging and thus, remove any logging from the WASM.
			log::info!("Hello World from offchain workers!");

			// Since off-chain workers are just part of the runtime code, they have direct access
			// to the storage and other included pallets.
			//
			// We can easily import `frame_system` and retrieve a block hash of the parent block.
			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);

			// It's a good practice to keep `fn offchain_worker()` function minimal, and move most
			// of the code to separate `impl` block.
			// Here we call a helper function to calculate current average price.
			// This function reads storage entries of the current state.
			let average: Option<u32> = Self::average_price();
			log::debug!("Current price: {:?}", average);

			// For this example we are going to send both signed and unsigned transactions
			// depending on the block number.
			// Usually it's enough to choose one or the other.
			let should_send = Self::choose_transaction_type(block_number);
			let res = match should_send {
				TransactionType::Signed => Self::fetch_price_and_send_signed(),
				TransactionType::UnsignedForAny => {
					Self::fetch_price_and_send_unsigned_for_any_account(block_number)
				}
				TransactionType::UnsignedForAll => {
					Self::fetch_price_and_send_unsigned_for_all_accounts(block_number)
				}
				TransactionType::Raw => Self::fetch_price_and_send_raw_unsigned(block_number),
				TransactionType::None => Ok(()),
			};
			if let Err(e) = res {
				log::error!("Error: {}", e);
			}
		}
	}

	/// A public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit new price to the list.
		///
		/// This method is a public function of the module and can be called from within
		/// a transaction. It appends given `price` to current list of prices.
		/// In our example the `offchain worker` will create, sign & submit a transaction that
		/// calls this function passing the price.
		///
		/// The transaction needs to be signed (see `ensure_signed`) check, so that the caller
		/// pays a fee to execute it.
		/// This makes sure that it's not easy (or rather cheap) to attack the chain by submitting
		/// excesive transactions, but note that it doesn't ensure the price oracle is actually
		/// working and receives (and provides) meaningful data.
		/// This example is not focused on correctness of the oracle itself, but rather its
		/// purpose is to showcase offchain worker capabilities.
		#[pallet::weight(0)]
		pub fn submit_price(origin: OriginFor<T>, price: u32) -> DispatchResultWithPostInfo {
			// Retrieve sender of the transaction.
			let who = ensure_signed(origin)?;
			// Add the price to the on-chain list.
			Self::add_price(who, price);
			Ok(().into())
		}

		/// Submit new price to the list via unsigned transaction.
		///
		/// Works exactly like the `submit_price` function, but since we allow sending the
		/// transaction without a signature, and hence without paying any fees,
		/// we need a way to make sure that only some transactions are accepted.
		/// This function can be called only once every `T::UnsignedInterval` blocks.
		/// Transactions that call that function are de-duplicated on the pool level
		/// via `validate_unsigned` implementation and also are rendered invalid if
		/// the function has already been called in current "session".
		///
		/// It's important to specify `weight` for unsigned calls as well, because even though
		/// they don't charge fees, we still don't want a single block to contain unlimited
		/// number of such transactions.
		///
		/// This example is not focused on correctness of the oracle itself, but rather its
		/// purpose is to showcase offchain worker capabilities.
		#[pallet::weight(0)]
		pub fn submit_price_unsigned(
			origin: OriginFor<T>,
			_block_number: T::BlockNumber,
			price: u32,
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// Add the price to the on-chain list, but mark it as coming from an empty address.
			Self::add_price(Default::default(), price);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, T::BlockNumber>,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			// This ensures that the function can only be called via unsigned transaction.
			ensure_none(origin)?;
			// Add the price to the on-chain list, but mark it as coming from an empty address.
			Self::add_price(Default::default(), price_payload.price);
			// now increment the block number at which we expect next unsigned transaction.
			let current_block = <system::Pallet<T>>::block_number();
			<NextUnsignedAt<T>>::put(current_block + T::UnsignedInterval::get());
			Ok(().into())
		}
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event generated when new price is accepted to contribute to the average.
		NewPrice { price: u32, who: T::AccountId },
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// Validate unsigned call to this module.
		///
		/// By default unsigned transactions are disallowed, but implementing the validator
		/// here we make sure that some particular calls (the ones produced by offchain worker)
		/// are being whitelisted and marked as valid.
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Firstly let's check that we call the right function.
			if let Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				ref signature,
			} = call
			{
				let signature_valid =
					SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());
				if !signature_valid {
					return InvalidTransaction::BadProof.into();
				}
				Self::validate_transaction_parameters(&payload.block_number, &payload.price)
			} else if let Call::submit_price_unsigned { block_number, price: new_price } = call {
				Self::validate_transaction_parameters(block_number, new_price)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	/// A vector of recently submitted prices.
	///
	/// This is used to calculate average price, should have bounded size.
	#[pallet::storage]
	#[pallet::getter(fn prices)]
	pub(super) type Prices<T: Config> = StorageValue<_, Vec<u32>, ValueQuery>;
	//pub type Prices<T> = StorageValue<_, VecDeque<(u64, Permill)>, ValueQuery>;

	/// Defines the block when next unsigned transaction will be accepted.
	///
	/// To prevent spam of unsigned (and unpayed!) transactions on the network,
	/// we only allow one transaction every `T::UnsignedInterval` blocks.
	/// This storage entry defines when new transaction is going to be accepted.
	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;
}

/// Payload used by this example crate to hold price
/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
pub struct PricePayload<Public, BlockNumber> {
	block_number: BlockNumber,
	price: u32,
	public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PricePayload<T::Public, T::BlockNumber> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

enum TransactionType {
	Signed,
	UnsignedForAny,
	UnsignedForAll,
	Raw,
	None,
}


const HTTP_REMOTE_REQUEST: &str = "https://api.coincap.io/v2/assets/polkadot";
const HTTP_HEADER_USER_AGENT: &str = "cc";

const FETCH_TIMEOUT_PERIOD: u64 = 30000; // in milli-seconds
//const LOCK_TIMEOUT_EXPIRATION: u64 = FETCH_TIMEOUT_PERIOD + 1000; // in milli-seconds
//const LOCK_BLOCK_EXPIRATION: u32 = 3; // in block number

impl<T: Config> Pallet<T> {
	/// Chooses which transaction type to send.
	///
	/// This function serves mostly to showcase `StorageValue` helper
	/// and local storage usage.
	///
	/// Returns a type of transaction that should be produced in current run.
	fn choose_transaction_type(block_number: T::BlockNumber) -> TransactionType {
		/// A friendlier name for the error that is going to be returned in case we are in the grace
		/// period.
		const RECENTLY_SENT: () = ();

		// Start off by creating a reference to Local Storage value.
		// Since the local storage is common for all offchain workers, it's a good practice
		// to prepend your entry with the module name.
		let val = StorageValueRef::persistent(b"example_ocw::last_send");
		// The Local Storage is persisted and shared between runs of the offchain workers,
		// and offchain workers may run concurrently. We can use the `mutate` function, to
		// write a storage entry in an atomic fashion. Under the hood it uses `compare_and_set`
		// low-level method of local storage API, which means that only one worker
		// will be able to "acquire a lock" and send a transaction if multiple workers
		// happen to be executed concurrently.
		let res = val.mutate(|last_send: Result<Option<T::BlockNumber>, StorageRetrievalError>| {
			match last_send {
				// If we already have a value in storage and the block number is recent enough
				// we avoid sending another transaction at this time.
				Ok(Some(block)) if block_number < block + T::GracePeriod::get() => {
					Err(RECENTLY_SENT)
				}
				// In every other case we attempt to acquire the lock and send a transaction.
				_ => Ok(block_number),
			}
		});

		// The result of `mutate` call will give us a nested `Result` type.
		// The first one matches the return of the closure passed to `mutate`, i.e.
		// if we return `Err` from the closure, we get an `Err` here.
		// In case we return `Ok`, here we will have another (inner) `Result` that indicates
		// if the value has been set to the storage correctly - i.e. if it wasn't
		// written to in the meantime.
		match res {
			// The value has been set correctly, which means we can safely send a transaction now.
			Ok(block_number) => {
				// Depending if the block is even or odd we will send a `Signed` or `Unsigned`
				// transaction.
				// Note that this logic doesn't really guarantee that the transactions will be sent
				// in an alternating fashion (i.e. fairly distributed). Depending on the execution
				// order and lock acquisition, we may end up for instance sending two `Signed`
				// transactions in a row. If a strict order is desired, it's better to use
				// the storage entry for that. (for instance store both block number and a flag
				// indicating the type of next transaction to send).
				let transaction_type = block_number % 3u32.into();
				if transaction_type == Zero::zero() {
					TransactionType::Signed
				} else if transaction_type == T::BlockNumber::from(1u32) {
					TransactionType::UnsignedForAny
				} else if transaction_type == T::BlockNumber::from(2u32) {
					TransactionType::UnsignedForAll
				} else {
					TransactionType::Raw
				}
			}
			// We are in the grace period, we should not send a transaction this time.
			Err(MutateStorageError::ValueFunctionFailed(RECENTLY_SENT)) => TransactionType::None,
			// We wanted to send a transaction, but failed to write the block number (acquire a
			// lock). This indicates that another offchain worker that was running concurrently
			// most likely executed the same logic and succeeded at writing to storage.
			// Thus we don't really want to send the transaction, knowing that the other run
			// already did.
			Err(MutateStorageError::ConcurrentModification(_)) => TransactionType::None,
		}
	}

	/// A helper function to fetch the price and send signed transaction.
	fn fetch_price_and_send_signed() -> Result<(), &'static str> {
		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(
				"No local accounts available. Consider adding one via `author_insertKey` RPC.",
			)?;
		}
		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;
		println!("pricepriceprice{}", price);

		// Using `send_signed_transaction` associated type we create and submit a transaction
		// representing the call, we've just created.
		// Submit signed will return a vector of results for all accounts that were found in the
		// local keystore with expected `KEY_TYPE`.
		let results = signer.send_signed_transaction(|_account| {
			// Received price is wrapped into a call to `submit_price` public function of this
			// pallet. This means that the transaction, when executed, will simply call that
			// function passing `price` as an argument.
			Call::submit_price { price }
		});

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, price),
				Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// A helper function to fetch the price and send a raw unsigned transaction.
	fn fetch_price_and_send_raw_unsigned(block_number: T::BlockNumber) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// Received price is wrapped into a call to `submit_price_unsigned` public function of this
		// pallet. This means that the transaction, when executed, will simply call that function
		// passing `price` as an argument.
		let call = Call::submit_price_unsigned { block_number, price };

		// Now let's create a transaction out of this call and submit it to the pool.
		// Here we showcase two ways to send an unsigned transaction / unsigned payload (raw)
		//
		// By default unsigned transactions are disallowed, so we need to whitelist this case
		// by writing `UnsignedValidator`. Note that it's EXTREMELY important to carefuly
		// implement unsigned validation logic, as any mistakes can lead to opening DoS or spam
		// attack vectors. See validation logic docs for more details.
		//
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|()| "Unable to submit unsigned transaction.")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_any_account(
		block_number: T::BlockNumber,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using any account
		let (_, result) = Signer::<T, T::AuthorityId>::any_account()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| Call::submit_price_unsigned_with_signed_payload {
					price_payload: payload,
					signature,
				},
			)
			.ok_or("No local accounts accounts available.")?;
		result.map_err(|()| "Unable to submit transaction")?;

		Ok(())
	}

	/// A helper function to fetch the price, sign payload and send an unsigned transaction
	fn fetch_price_and_send_unsigned_for_all_accounts(
		block_number: T::BlockNumber,
	) -> Result<(), &'static str> {
		// Make sure we don't fetch the price if unsigned transaction is going to be rejected
		// anyway.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if next_unsigned_at > block_number {
			return Err("Too early to send unsigned transaction");
		}

		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let price = Self::fetch_price().map_err(|_| "Failed to fetch price")?;

		// -- Sign using all accounts
		let transaction_results = Signer::<T, T::AuthorityId>::all_accounts()
			.send_unsigned_transaction(
				|account| PricePayload { price, block_number, public: account.public.clone() },
				|payload, signature| Call::submit_price_unsigned_with_signed_payload {
					price_payload: payload,
					signature,
				},
			);
		for (_account_id, result) in transaction_results.into_iter() {
			if result.is_err() {
				return Err("Unable to submit transaction");
			}
		}

		Ok(())
	}

	/// Fetch current price and return the result in cents.
	fn fetch_price() -> Result<u32, http::Error> {
		println!("{}", 1);
		// We want to keep the offchain worker execution time reasonable, so we set a hard-coded
		// deadline to 2s to complete the external call.
		// You can also wait idefinitely for the response, however you may still get a timeout
		// coming from the host machine.
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(1000_000));
		// Initiate an external HTTP GET request.
		// This is using high-level wrappers from `sp_runtime`, for the low-level calls that
		// you can find in `sp_io`. The API is trying to be similar to `reqwest`, but
		// since we are running in a custom WASM execution environment we can't simply
		// import the library here.
		let request = http::Request::get("https://api.coincap.io/v2/assets/polkadot");
		// We set the deadline for sending of the request, note that awaiting response can
		// have a separate deadline. Next we send the request, before that it's also possible
		// to alter request headers or stream body content in case of non-GET requests.
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

		// The request is already being processed by the host, we are free to do anything
		// else in the worker (we can send multiple concurrent requests too).
		// At some point however we probably want to check the response though,
		// so we can block current thread and wait for it to finish.
		// Note that since the request is being driven by the host, we don't have to wait
		// for the request to have it complete, we will just not read the response.
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

		// Let's check the status code before we proceed to reading the response.
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown);
		}

		// Next we want to fully read the response body and collect it to a vector of bytes.
		// Note that the return object allows you to read the body in chunks as well
		// with a way to control the deadline.
		let body = response.body().collect::<Vec<u8>>();
		println!("response{:?}", body);

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		println!("body_strbody_strbody_str{}", body_str);

		let price = match Self::parse_price(body_str) {
			Some(price) => Ok(price),
			None => {
				log::warn!("Unable to extract price from the response: {:?}", body_str);
				Err(http::Error::Unknown)
			}
		}?;

		log::warn!("Got price: {} cents", price);
		println!("Got price: {} cents", price);

		Ok(price)
	}


	// This function uses the `offchain::http` API to query the remote github information,
	///   and returns the JSON response as vector of bytes.
	pub fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
		println!("{}",1);
		log::info!("sending request to: {}", HTTP_REMOTE_REQUEST);

		// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
		let request = rt_offchain::http::Request::get(HTTP_REMOTE_REQUEST);
		println!("{}",2);

		// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
		let timeout = sp_io::offchain::timestamp()
			.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));
			println!("{}",3);

		// For github API request, we also need to specify `user-agent` in http request header.
		//   See: https://developer.github.com/v3/#user-agent-required
		let pending = request
			.add_header("User-Agent", HTTP_HEADER_USER_AGENT)
			.deadline(timeout) // Setting the timeout time
			.send() // Sending the request out by the host
			.map_err(|_| <Error<T>>::HttpFetchingError)?;
			println!("{}",5);

		// By default, the http request is async from the runtime perspective. So we are asking the
		//   runtime to wait here.
		// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
		//   ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
		let response = pending
			.try_wait(timeout)
			.map_err(|_| <Error<T>>::HttpFetchingError)?
			.map_err(|_| <Error<T>>::HttpFetchingError)?;

		println!("responseresponseresponse{:?}",response);

		println!("{}",6);

		if response.code != 200 {
			log::error!("Unexpected http request status code: {}", response.code);
			return Err(<Error<T>>::HttpFetchingError);
		}

		// Next we fully read the response body and collect it to a vector of bytes.
		Ok(response.body().collect::<Vec<u8>>())
	}
	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `None` when parsing failed or `Some(price in cents)` when parsing is successful.
	fn parse_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			}
			_ => return None,
		};

		let exp = price.fraction_length.checked_sub(2).unwrap_or(0);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

	// fn fetch_price_info() -> Result<(), Error<T>> {
	// 	// Create a reference to Local Storage value.
	// 	// Since the local storage is common for all offchain workers, it's a good practice
	// 	// to prepend our entry with the pallet name.
	// 	let s_info = StorageValueRef::persistent(b"offchain-demo::dot-price-info");

	// 	// Local storage is persisted and shared between runs of the offchain workers,
	// 	// offchain workers may run concurrently. We can use the `mutate` function to
	// 	// write a storage entry in an atomic fashion.
	// 	//
	// 	// With a similar API as `StorageValue` with the variables `get`, `set`, `mutate`.
	// 	// We will likely want to use `mutate` to access
	// 	// the storage comprehensively.
	// 	//
	// 	if let Ok(Some(gh_info)) = s_info.get::<DotPriceInfo>() {
	// 		// gh-info has already been fetched. Return early.
	// 		log::info!("cached gh-info: {:?}", gh_info);
	// 		return Ok(());
	// 	}

	// 	// Since off-chain storage can be accessed by off-chain workers from multiple runs, it is important to lock
	// 	//   it before doing heavy computations or write operations.
	// 	//
	// 	// There are four ways of defining a lock:
	// 	//   1) `new` - lock with default time and block exipration
	// 	//   2) `with_deadline` - lock with default block but custom time expiration
	// 	//   3) `with_block_deadline` - lock with default time but custom block expiration
	// 	//   4) `with_block_and_time_deadline` - lock with custom time and block expiration
	// 	// Here we choose the most custom one for demonstration purpose.
	// 	let mut lock = StorageLock::<BlockAndTime<Self>>::with_block_and_time_deadline(
	// 		b"offchain-demo::lock",
	// 		LOCK_BLOCK_EXPIRATION,
	// 		rt_offchain::Duration::from_millis(LOCK_TIMEOUT_EXPIRATION),
	// 	);

	// 	// We try to acquire the lock here. If failed, we know the `fetch_n_parse` part inside is being
	// 	//   executed by previous run of ocw, so the function just returns.
	// 	if let Ok(_guard) = lock.try_lock() {
	// 		match Self::fetch_n_parse() {
	// 			Ok(gh_info) => {
	// 				s_info.set(&gh_info);
	// 			}
	// 			Err(err) => {
	// 				return Err(err);
	// 			}
	// 		}
	// 	}
	// 	Ok(())
	// }

	/// Fetch from remote and deserialize the JSON to a struct
	// fn fetch_n_parse() -> Result<DotPriceInfo, Error<T>> {
	// 	let resp_bytes = Self::fetch_from_remote().map_err(|e| {
	// 		log::error!("fetch_from_remote error: {:?}", e);
	// 		<Error<T>>::HttpFetchingError
	// 	})?;

	// 	let resp_str =
	// 		str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
	// 	// Print out our fetched JSON string
	// 	log::info!("{}", resp_str);

	// 	// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
	// 	let gh_info: DotPriceInfo =
	// 		serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
	// 	Ok(gh_info)
	// }

	// fn fetch_n_parse() -> Result<DotPriceInfo, Error<T>> {
	// 	let resp_bytes = Self::fetch_from_remote().map_err(|e| {
	// 		log::error!("fetch_from_remote error: {:?}", e);
	// 		<Error<T>>::HttpFetchingError
	// 	})?;

	// 	let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
	// 	// Print out our fetched JSON string
	// 	log::info!("{}", resp_str);

	// 	// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
	// 	let dot_price_info: DotPriceInfo =
	// 		serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
	// 	Ok(dot_price_info)
	// }

	// fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
	// 	log::info!("sending request to: {}", HTTP_REMOTE_REQUEST);

	// 	// Initiate an external HTTP GET request. This is using high-level wrappers from `sp_runtime`.
	// 	let request = offchain::http::Request::get(HTTP_REMOTE_REQUEST);

	// 	// Keeping the offchain worker execution time reasonable, so limiting the call to be within 3s.
	// 	let timeout = sp_io::offchain::timestamp()
	// 		.add(offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));

	// 	// For github API request, we also need to specify `user-agent` in http request header.
	// 	//   See: https://developer.github.com/v3/#user-agent-required
	// 	let pending = request
	// 		.add_header("User-Agent", HTTP_HEADER_USER_AGENT)
	// 		.deadline(timeout) // Setting the timeout time
	// 		.send() // Sending the request out by the host
	// 		.map_err(|_| <Error<T>>::HttpFetchingError)?;

	// 	// By default, the http request is async from the runtime perspective. So we are asking the
	// 	//   runtime to wait here.
	// 	// The returning value here is a `Result` of `Result`, so we are unwrapping it twice by two `?`
	// 	//   ref: https://substrate.dev/rustdocs/v2.0.0/sp_runtime/offchain/http/struct.PendingRequest.html#method.try_wait
	// 	let response = pending
	// 		.try_wait(timeout)
	// 		.map_err(|_| <Error<T>>::HttpFetchingError)?
	// 		.map_err(|_| <Error<T>>::HttpFetchingError)?;

	// 	if response.code != 200 {
	// 		log::error!("Unexpected http request status code: {}", response.code);
	// 		return Err(<Error<T>>::HttpFetchingError);
	// 	}

	// 	// Next we fully read the response body and collect it to a vector of bytes.
	// 	Ok(response.body().collect::<Vec<u8>>())
	// }

	/// Add new price to the list.
	fn add_price(who: T::AccountId, price: u32) {
		log::info!("Adding to the average: {}", price);
		<Prices<T>>::mutate(|prices| {
			const MAX_LEN: usize = 64;

			if prices.len() < MAX_LEN {
				prices.push(price);
			} else {
				prices[price as usize % MAX_LEN] = price;
			}
		});

		let average = Self::average_price()
			.expect("The average is not empty, because it was just mutated; qed");
		log::info!("Current average price is: {}", average);
		// here we are raising the NewPrice event
		Self::deposit_event(Event::NewPrice { price, who });
	}

	/// Calculate current average price.
	fn average_price() -> Option<u32> {
		let prices = <Prices<T>>::get();
		if prices.is_empty() {
			None
		} else {
			Some(prices.iter().fold(0_u32, |a, b| a.saturating_add(*b)) / prices.len() as u32)
		}
	}

	fn validate_transaction_parameters(
		block_number: &T::BlockNumber,
		new_price: &u32,
	) -> TransactionValidity {
		// Now let's check if the transaction has any chance to succeed.
		let next_unsigned_at = <NextUnsignedAt<T>>::get();
		if &next_unsigned_at > block_number {
			return InvalidTransaction::Stale.into();
		}
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into();
		}

		// We prioritize transactions that are more far away from current average.
		//
		// Note this doesn't make much sense when building an actual oracle, but this example
		// is here mostly to show off offchain workers capabilities, not about building an
		// oracle.
		let avg_price = Self::average_price()
			.map(|price| if &price > new_price { price - new_price } else { new_price - price })
			.unwrap_or(0);

		ValidTransaction::with_tag_prefix("ExampleOffchainWorker")
			// We set base priority to 2**20 and hope it's included before any other
			// transactions in the pool. Next we tweak the priority depending on how much
			// it differs from the current average. (the more it differs the more priority it
			// has).
			.priority(T::UnsignedPriority::get().saturating_add(avg_price as _))
			// This transaction does not require anything else to go before into the pool.
			// In theory we could require `previous_unsigned_at` transaction to go first,
			// but it's not necessary in our case.
			//.and_requires()
			// We set the `provides` tag to be the same as `next_unsigned_at`. This makes
			// sure only one transaction produced after `next_unsigned_at` will ever
			// get to the transaction pool and will end up in the block.
			// We can still have multiple transactions compete for the same "spot",
			// and the one with higher priority will replace other one in the pool.
			.and_provides(next_unsigned_at)
			// The transaction is only valid for next 5 blocks. After that it's
			// going to be revalidated by the pool.
			.longevity(5)
			// It's fine to propagate that transaction to other peers, which means it can be
			// created even by nodes that don't produce blocks.
			// Note that sometimes it's better to keep it for yourself (if you are the block
			// producer), since for instance in some schemes others may copy your solution and
			// claim a reward.
			.propagate(true)
			.build()
	}

	// pub fn de_string_to_tuple<'de, D>(de: D) -> Result<DotPrice, D::Error>
	// where
	// 	D: Deserializer<'de>,
	// {
	// 	let s: &str = Deserialize::deserialize(de)?;
	// 	let price_usd: Vec<&str> = s.split(".").collect();
	// 	let price_usd_num: u64 = price_usd[0].parse().unwrap();
	// 	let price_usd_permill: Permill = Permill::from_parts(price_usd[1][..6].parse::<u32>().unwrap());
	// 	Ok((price_usd_num, price_usd_permill))
	// }

	// fn fetch_n_parse<U>() -> Result<U, Error<T>> {
	// 		let resp_bytes = Self::fetch_from_remote().map_err(|e| {
	// 			log::error!("fetch_from_remote error: {:?}", e);
	// 			<Error<T>>::HttpFetchingError
	// 		})?;

	// 		let resp_str =
	// 			str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;
	// 		// Print out our fetched JSON string
	// 		log::info!("response: {}", resp_str);

	// 		// Deserializing JSON to struct, thanks to `serde` and `serde_derive`
	// 		let info: U = serde_json::from_str(resp_str).map_err(|_| <Error<T>>::DecodeFailed)?;
	// 		Ok(info)
	// 	}
}
