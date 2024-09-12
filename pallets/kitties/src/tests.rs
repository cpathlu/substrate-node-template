#![cfg(test)]

use super::*;

use crate::{
	mock::*, pallet::{Error}
};

use frame_support::{
	assert_ok,
	assert_noop,
	assert_err,
};

#[test]
fn should_build_genesis_kitties() {
	new_test_ext().execute_with(|| {
		assert_eq!(SubstrateKitties::kitty_cnt(), 2u32.into());

		let kitties_owned_by_1 = SubstrateKitties::kitties_owned(1);
		assert_eq!(kitties_owned_by_1.len(), 1);

		let kitties_owned_by_2 = SubstrateKitties::kitties_owned(2);
		assert_eq!(kitties_owned_by_2.len(), 1);

		let kid1 = kitties_owned_by_1[0];
		let kitty1 = SubstrateKitties::kitties(kid1)
			.expect("Could have this kitty ID owned by acct 1");
		assert_eq!(kitty1.owner, 1);

		let kid2 = kitties_owned_by_2[0];
		let kitty2 = SubstrateKitties::kitties(kid2)
			.expect("Could have this kitty ID owned by acct 2");
		assert_eq!(kitty2.owner, 2);
	});
}

#[test]
fn create_kitty_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(SubstrateKitties::create_kitty(Origin::signed(1)));

		assert_eq!(SubstrateKitties::kitty_cnt(), 3u32.into());
		assert_eq!(SubstrateKitties::kitties_owned(1).len(), 2);

		assert_eq!(SubstrateKitties::kitties_owned(5).len(), 0);

		let hash = SubstrateKitties::kitties_owned(1)[1];
		let kitty = SubstrateKitties::kitties(hash).expect("should found the kitty");
		assert_eq!(kitty.owner, 1);
		assert_eq!(kitty.price, None);
	});
}



#[test]
fn set_price_should_work() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_id, Some(1)));
		assert_eq!(SubstrateKitties::kitties(kitty_id).unwrap().price, Some(1));
	});
}

#[test]
fn set_price_not_own_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_noop!(
			SubstrateKitties::set_price(Origin::signed(2), kitty_id, Some(1)),
			Error::<Test>::NotKittyOwner
		);
	});
}


#[test]
fn buy_kitty_should_work() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_id, Some(1)));
		assert_ok!(SubstrateKitties::buy_kitty(Origin::signed(2), kitty_id, 1));
	});
}

#[test]
fn buy_kitty_is_owner_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_id, Some(1)));
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(1), kitty_id, 1),
			Error::<Test>::BuyerIsKittyOwner
		);
	});
}

#[test]
fn buy_kitty_low_price_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_id, Some(2)));
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_id, 1),
			Error::<Test>::KittyBidPriceTooLow
		);
	});
}



#[test]
fn buy_kitty_not_enough_balance_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_ok!(SubstrateKitties::set_price(Origin::signed(1), kitty_id, Some(2)));
		assert_noop!(
			SubstrateKitties::buy_kitty(Origin::signed(2), kitty_id, 200_000),
			Error::<Test>::NotEnoughBalance
		);
	});
}

#[test]
fn transfer_kitty_should_work() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];

		assert_ok!(SubstrateKitties::transfer(Origin::signed(1), 3, kitty_id));
		assert_eq!(SubstrateKitties::kitties_owned(1).len(), 0);
		assert_eq!(SubstrateKitties::kitties_owned(3).len(), 1);
		let new_id = SubstrateKitties::kitties_owned(3)[0];
		assert_eq!(kitty_id, new_id);
	});
}

#[test]
fn transfer_kitty_to_self_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id = SubstrateKitties::kitties_owned(1)[0];
		assert_noop!(
			SubstrateKitties::transfer(Origin::signed(1), 1, kitty_id),
			Error::<Test>::TransferToSelf
		);
	});
}

#[test]
fn transfer_non_owned_kitty_should_fail() {
	new_test_ext().execute_with(|| {
		let hash = SubstrateKitties::kitties_owned(1)[0];

		assert_noop!(
			SubstrateKitties::transfer(Origin::signed(9), 2, hash),
			Error::<Test>::NotKittyOwner
		);
	});
}

#[test]
fn breed_kitty_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(SubstrateKitties::create_kitty(Origin::signed(1)));
		let kitty_id1= SubstrateKitties::kitties_owned(1)[0];
		let kitty_id2= SubstrateKitties::kitties_owned(1)[1];

		assert_ok!(SubstrateKitties::breed_kitty(Origin::signed(1), kitty_id1, kitty_id2));
		assert_eq!(SubstrateKitties::kitties_owned(1).len(), 3);
	});
}

#[test]
fn breed_kitty_not_own_should_fail() {
	new_test_ext().execute_with(|| {
		let kitty_id1= 12345;
		let kitty_id2= 54321;

		assert_noop!(
			SubstrateKitties::breed_kitty(Origin::signed(1), kitty_id1, kitty_id2),
			Error::<Test>::KittyNotExist
		);
	});
}


#[test]
fn create_kitty_not_enough_balance_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			SubstrateKitties::create_kitty(Origin::signed(3)),
			Error::<Test>::InvalidReserveAmount
		);
	});
}