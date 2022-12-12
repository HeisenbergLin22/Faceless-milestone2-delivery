#![cfg_attr(not(feature = "std"), no_std)]

/// This file defines a substrate pallet for the Faceless protocol.
/// It includes the cryptographic verification logic for the following 
/// relevant zero-knowledge proofs:
/// 1. Verification of burn proof
/// 2. Verification of transfer proof

extern crate alloc;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use frame_support::traits::{Currency, ExistenceRequirement};
	use frame_support::PalletId;
	use frame_support::sp_runtime::traits::{AccountIdConversion, Zero};
    use sp_std::vec::Vec;
    use aibe::zk::burn::{BurnStatement, BurnProof, BurnVerifier};
    use aibe::zk::transfer::{TransferStatement, TransferProof, TransferVerifier};
	use aibe::bf_ibe::{BFIbe, CipherText, PlainData, MasterSecretKey, MasterPublicKey, IdSecretKey, G1, G2, Gt, pairing, Group};
	use aibe::utils::{u64_to_scalar};
    use borsh::de::BorshDeserialize;
	use borsh::ser::BorshSerialize;
	use rand_chacha::ChaCha20Rng;


	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type Currency: Currency<Self::AccountId>;
	}

	type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
    // 'without_storage_info' is needed for storing variable-length Vec<u8> data in the StorageMap (Proofs below).
    #[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
    pub(super) type Proofs<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, (T::AccountId, T::BlockNumber)>; 

	// #[pallet::storage]
	// #[pallet::getter(fn get_balance)]
    // pub(super) type Accounts<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_balance)]
	pub(super) type Accounts<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<u8>>;

	#[pallet::storage]
	// This getter `get_faceless_account` is only for use inside the Substrate node. Externally, like in Polkadot-JS,
	// the function `facelessAccount` is automatically generated for usage.
	#[pallet::getter(fn get_faceless_account)] 
	pub type FacelessAccount<T: Config> = StorageValue<_, T::AccountId>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        BurnVerificationSuccess(T::AccountId, Vec<u8>),
        TransferVerificationSuccess(T::AccountId, Vec<u8>),
		RegisterSuccess(T::AccountId, Vec<u8>),
		DepositSuccess(T::AccountId, u32),
		WithdrawSuccess(T::AccountId, u32),
		TransferSuccess(T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
        BurnVerificationFailure,
        TransferVerificationFailure,
		AccountNotRegistered,
	}

	pub const PALLET_ID: PalletId = PalletId(*b"faceless");

	#[pallet::genesis_config]
	pub struct GenesisConfig {
		// pub faceless_account: T::AccountId,
	}

	// Give it a default value.
	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				// faceless_account: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			let faceless_account = PALLET_ID.into_account_truncating();
			// Need to first create a Currency account for the faceless account, otherwise the transfer will fail.
			T::Currency::deposit_creating(&faceless_account, T::Currency::minimum_balance());
			// Store the faceless account into a storage item
			FacelessAccount::<T>::put::<T::AccountId>(faceless_account);
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(1_000)]
		pub fn register(origin: OriginFor<T>, pk_id: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			
			// Create encryption of 0
			let ct_0 = G1::one();
			let ct_1 = Gt::try_from_slice(base64::decode(pk_id.as_slice()).unwrap().as_slice()).unwrap();
			let zero_ct = base64::encode((ct_0, ct_1).try_to_vec().unwrap()).into_bytes();

			Accounts::<T>::insert::<Vec<u8>, Vec<u8>>(pk_id.clone(), zero_ct);

			Self::deposit_event(Event::RegisterSuccess(sender, pk_id));
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn deposit(origin: OriginFor<T>, pk_id: Vec<u8>, amount: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Current encrypted balance
			let balance = Self::get_balance(&pk_id).ok_or(Error::<T>::AccountNotRegistered)?;
			let mut balance = CipherText::try_from_slice(base64::decode(balance.as_slice()).unwrap().as_slice()).unwrap();

			// Add the encryption of amount to current encrypted balance
			let addend = pairing(G1::one(), G2::one()).pow(u64_to_scalar(amount as u64));
			balance.1 = balance.1 * addend;
			let balance = base64::encode(balance.try_to_vec().unwrap()).into_bytes();
			Accounts::<T>::insert(pk_id, balance);

			// Transfer sender's native tokens to pallet's account
			let value: BalanceOf<T> = From::<u32>::from(amount);
			T::Currency::transfer(&sender, &Self::get_faceless_account().unwrap(), value, ExistenceRequirement::AllowDeath)?;

			Self::deposit_event(Event::DepositSuccess(sender, amount));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn withdraw(origin: OriginFor<T>, pk_id: Vec<u8>, destination: T::AccountId, amount: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Current encrypted balance
			let balance = Self::get_balance(&pk_id).ok_or(Error::<T>::AccountNotRegistered)?;
			let mut balance = CipherText::try_from_slice(base64::decode(balance.as_slice()).unwrap().as_slice()).unwrap();

			// Substract the encryption of amount from current encrypted balance
			let deduction = pairing(G1::one(), G2::one()).pow(-u64_to_scalar(amount as u64));
			balance.1 = balance.1 * deduction;
			let balance = base64::encode(balance.try_to_vec().unwrap()).into_bytes();
			Accounts::<T>::insert(pk_id, balance);

			// Transfer tokens from pallet's account to sender
			let value: BalanceOf<T> = From::<u32>::from(amount);			
			T::Currency::transfer(&Self::get_faceless_account().unwrap(), &destination, value, ExistenceRequirement::AllowDeath)?;

			Self::deposit_event(Event::WithdrawSuccess(sender, amount));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn transfer(origin: OriginFor<T>, pk_id1: Vec<u8>, pk_id2: Vec<u8>, enc_amount1: Vec<u8>, enc_amount2: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let enc_amount1 = CipherText::try_from_slice(base64::decode(enc_amount1.as_slice()).unwrap().as_slice()).unwrap();
			let enc_amount2 = CipherText::try_from_slice(base64::decode(enc_amount2.as_slice()).unwrap().as_slice()).unwrap();

			let balance1 = Self::get_balance(&pk_id1).ok_or(Error::<T>::AccountNotRegistered)?;
			let mut balance1 = CipherText::try_from_slice(base64::decode(balance1.as_slice()).unwrap().as_slice()).unwrap();
			balance1 = BFIbe::<ChaCha20Rng>::add_ciphers(&balance1, &enc_amount1);
			let balance1 = base64::encode(balance1.try_to_vec().unwrap()).into_bytes();

			let balance2 = Self::get_balance(&pk_id2).ok_or(Error::<T>::AccountNotRegistered)?;
			let mut balance2 = CipherText::try_from_slice(base64::decode(balance2.as_slice()).unwrap().as_slice()).unwrap();
			balance2 = BFIbe::<ChaCha20Rng>::add_ciphers(&balance2, &enc_amount2);
			let balance2 = base64::encode(balance2.try_to_vec().unwrap()).into_bytes();

			Accounts::<T>::insert(pk_id1, balance1);
			Accounts::<T>::insert(pk_id2, balance2);
			
			Self::deposit_event(Event::TransferSuccess(sender));

			Ok(())
		}

        /// A dispatchable that takes a burn statement and a burn proof as inputs, verifies the proof, and 
        /// emits an event that denotes the verification status.
		#[pallet::weight(1_000)]
		pub fn verify_burn(origin: OriginFor<T>, statement: Vec<u8>, proof: Vec<u8>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

            let bs = BurnStatement::try_from_slice(base64::decode(statement.as_slice()).unwrap().as_slice()).unwrap();
            let bp = BurnProof::try_from_slice(base64::decode(proof.as_slice()).unwrap().as_slice()).unwrap();

            let result = BurnVerifier::verify_proof(bs, bp);

            match result {
                Ok(()) => {
                    Self::deposit_event(Event::BurnVerificationSuccess(sender, proof));
                    Ok(())
                },
                Err(_) => {
                    Err(Error::<T>::BurnVerificationFailure.into())
                }
            }
		}

        /// A dispatchable that takes a transfer statement and a transfer proof as inputs, verifies the proof, and 
        /// emits an event that denotes the verification status.
		#[pallet::weight(10_000)]
		pub fn verify_transfer(origin: OriginFor<T>, statement: Vec<u8>, proof: Vec<u8>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/v3/runtime/origins
			let sender = ensure_signed(origin)?;

            let bs = TransferStatement::try_from_slice(base64::decode(statement.as_slice()).unwrap().as_slice()).unwrap();
            let bp = TransferProof::try_from_slice(base64::decode(proof.as_slice()).unwrap().as_slice()).unwrap();

            let result = TransferVerifier::verify_proof(bs, bp);

            match result {
                Ok(()) => {
                    Self::deposit_event(Event::TransferVerificationSuccess(sender, proof));
                    Ok(())
                },
                Err(_) => {
                    Err(Error::<T>::TransferVerificationFailure.into())
                }
            }
		}
	}
}
