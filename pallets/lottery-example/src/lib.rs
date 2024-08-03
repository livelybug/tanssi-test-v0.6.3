// All pallets must be configured for `no_std`.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::{*, ValueQuery, OptionQuery};
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::vec::Vec;

    use frame_support::
    {
        sp_runtime::traits::AccountIdConversion,
        traits:: {
            Currency, ExistenceRequirement, Randomness
        },
        PalletId,
    };

    type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {

        // Event definition
        type RuntimeEvent: From<Event<Self>>
        + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // Currency
        type Currency: Currency<Self::AccountId>;

        // Randomness
        type MyRandomness: Randomness<Self::Hash, BlockNumberFor<Self>>;

        // Ticket cost
        #[pallet::constant]
        type TicketCost: Get<BalanceOf<Self>>;

        // Maximum number of participants
        #[pallet::constant]
        type MaxParticipants: Get<u32>;

        // Module Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event emitted when a ticket is bought
        TicketBought { who: T::AccountId },
        /// Event emitted when the prize is awarded
        PrizeAwarded { winner: T::AccountId },
        /// Event emitted when there are no participants
        ThereAreNoParticipants,
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        NotEnoughCurrency,
        AccountAlreadyParticipating,
        CanNotAddParticipant,
    }

    #[pallet::storage]
    #[pallet::getter(fn get_participants)]
    pub(super) type Participants<T: Config> = StorageValue<
        _,
        BoundedVec<T::AccountId, T::MaxParticipants>,
        OptionQuery
    >;

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::call_index(0)]
        #[pallet::weight(0)]
        pub fn buy_ticket(origin: OriginFor<T>) -> DispatchResult {

            // 1. Validates the origin signature
            let buyer = ensure_signed(origin)?;

            // 2. Checks that the user has enough balance to afford the ticket price
            ensure!(
            T::Currency::free_balance(&buyer) >= T::TicketCost::get(),
            Error::<T>::NotEnoughCurrency
        );

            // 3. Checks that the user is not already participating
            if let Some(participants) = Self::get_participants() {
                ensure!(
                !participants.contains(&buyer),
                Error::<T>::AccountAlreadyParticipating
            );
            }

            // 4. Adds the user as a new participant for the prize
            match Self::get_participants() {
                Some(mut participants) => {
                    ensure!(
                    participants.try_push(buyer.clone()).is_ok(),
                    Error::<T>::CanNotAddParticipant
                );
                    Participants::<T>::set(Some(participants));
                },
                None => {
                    let mut participants = BoundedVec::new();
                    ensure!(
                    participants.try_push(buyer.clone()).is_ok(),
                    Error::<T>::CanNotAddParticipant
                );
                    Participants::<T>::set(Some(participants));
                }
            };

            // 5. Transfers the ticket cost to the module's account
            // to be hold until transferred to the winner
            T::Currency::transfer(
                &buyer,
                &Self::get_pallet_account(),
                T::TicketCost::get(),
                ExistenceRequirement::KeepAlive)?;

            // 6. Notify the event
            Self::deposit_event(Event::TicketBought { who: buyer });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(0)]
        pub fn award_prize(origin: OriginFor<T>) -> DispatchResult {

            // 1. Validates the origin signature
            let _who = ensure_root(origin)?;

            match Self::get_participants() {
                Some(participants) => {

                    // 2. Gets a random number from the randomness module
                    let nonce = Self::get_and_increment_nonce();
                    let (random_seed, _) = T::MyRandomness::random(&nonce);
                    let random_number = <u32>::decode(&mut random_seed.as_ref())
                        .expect("secure hashes should always be bigger than u32; qed");

                    // 3. Selects the winner from the participants lit
                    let winner_index = random_number as usize % participants.len();
                    let winner = participants.as_slice().get(winner_index).unwrap();

                    // 4. Transfers the total prize to the winner's account
                    let prize = T::Currency::free_balance(&Self::get_pallet_account());
                    T::Currency::transfer(
                        &Self::get_pallet_account(),
                        &winner,
                        prize,
                        ExistenceRequirement::AllowDeath)?;

                    // 5. Resets the participants list, and gets ready for another lottery round
                    Participants::<T>::kill();

                    // 6. Notify the event
                    Self::deposit_event(Event::PrizeAwarded { winner: winner.clone() } );
                },
                None => {
                    // Notify the event (No participants)
                    Self::deposit_event(Event::ThereAreNoParticipants);
                }
            };

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {

        fn get_pallet_account() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        fn get_and_increment_nonce() -> Vec<u8> {
            // let nonce = Nonce::<T>::get();
            // Nonce::<T>::put(nonce.wrapping_add(1));
            // nonce.encode()
            vec![]
        }
    }
}
