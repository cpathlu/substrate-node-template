use frame_support::{assert_noop, assert_ok};

use crate::{Error, mock::*};

#[test]
fn create_kitty_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Kitties::create_kitty(Origin::signed(10)));

		// check that 3 kitties exists (together with the two from genesis)
		assert_eq!(Kitties::kitty_cnt(), 3);

		// check that account #10 owns 1 kitty
		assert_eq!(Kitties::kitties_owned(10).len(), 1);

		// check that some random account #5 does not own a kitty
		assert_eq!(Kitties::kitties_owned(5).len(), 0);

		// check that this kitty is specifically owned by account #10
		let hash = Kitties::kitties_owned(10)[0];
		let kitty = Kitties::kitties(hash).expect("should found the kitty");
		assert_eq!(kitty.owner, 10);
		assert_eq!(kitty.price, None);

	});
}

#[test]
fn get_kitty_works() {
	new_test_ext().execute_with(|| {

	});
}

#[test]
fn mutate_price_works() {
	new_test_ext().execute_with(|| {

	});
}

#[test]
fn transfer_works() {
	new_test_ext().execute_with(|| {

	});
}

#[test]
fn buy_kitty_works() {
	new_test_ext().execute_with(|| {

	});
}

#[test]
fn breed_kitty_works() {
	new_test_ext().execute_with(|| {

	});
}


#[test]
fn create_kitty_should_work() {
	new_test_ext().execute_with(|| {
		// create a kitty with account #10.
		assert_ok!(Kitties::create_kitty(Origin::signed(10)));

		// check that 3 kitties exists (together with the two from genesis)
		assert_eq!(Kitties::kitty_cnt(), 3);

		// check that account #10 owns 1 kitty
		assert_eq!(Kitties::kitties_owned(10).len(), 1);

		// check that some random account #5 does not own a kitty
		assert_eq!(Kitties::kitties_owned(5).len(), 0);

		// check that this kitty is specifically owned by account #10
		let hash = Kitties::kitties_owned(10)[0];
		let kitty = Kitties::kitties(hash).expect("should found the kitty");
		assert_eq!(kitty.owner, 10);
		assert_eq!(kitty.price, None);
	});
}

#[test]
fn transfer_kitty_should_work() {
	new_test_ext().execute_with(|| {
		// check that acct 10 own a kitty
		assert_ok!(Kitties::create_kitty(Origin::signed(10)));
		assert_eq!(Kitties::kitties_owned(10).len(), 1);
		let hash = Kitties::kitties_owned(10)[0];

		// acct 10 send kitty to acct 3
		assert_ok!(Kitties::transfer(Origin::signed(10), 3, hash));

		// acct 10 now has nothing
		assert_eq!(Kitties::kitties_owned(10).len(), 0);
		// but acct 3 does
		assert_eq!(Kitties::kitties_owned(3).len(), 1);
		let new_hash = Kitties::kitties_owned(3)[0];
		// and it has the same hash
		assert_eq!(hash, new_hash);
	});
}

#[test]
fn transfer_non_owned_kitty_should_fail() {
	new_test_ext().execute_with(|| {
		let hash = Kitties::kitties_owned(1)[0];

		// account 0 cannot transfer a kitty with this hash.
		assert_noop!(
			Kitties::transfer(Origin::signed(9), 2, hash),
			Error::<Test>::NotKittyOwner
		);
	});
}
