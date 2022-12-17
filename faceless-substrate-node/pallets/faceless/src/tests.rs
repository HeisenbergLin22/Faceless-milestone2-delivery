use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};


#[test]
fn test_register() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		assert_ok!(FacelessModule::register(RuntimeOrigin::signed(1), Vec::from("ONKpKGedbVyPXAznId3okhveVOfbZH/NcHuA9M1NfSUsbkYJoV6y+aTGpUmNyZXxzjkzFMe0y1p/LJaZyW/YAbqSdVTfWbbKmLrkjTPHfsWTqOSzyJaPmw4pJcYFTAcQmNI+EhSkuuSk/gEIo8SCyoPoBQ7nIDIyxNKNlQSW4hxKzI0a6sKTIy0br+qQGDnYEaFYwr5PbhkQYM9t3tWtJ71HcKczxmQQ9z725XaeCNJ3vfXLqrmrIYSNAejUWu8uKXUqcAQb3vnY5i+PFf981OmYx2ksbjzoat6xHIulyhSNLsxaGWWxLiv64PoSgjprRBqHgE9wVv5W6nhhyPhTF07DTiZ1P4oQysm2tbDk8XotkpPiSwtUIqqz+qt245sklgg+sRAcc1oqf3d6GcsQDpra7yn6oda/vZv7qmmVwC6MJB226CMxxn1+XaPkwmtBai+xESUg+GeINP3KA23SCufhIywNaCi6eH4jWMiIE5D4jx1klJIwevPnsj9gOeQQ")));
		// // Read pallet storage and assert an expected result.
		// assert_eq!(FacelessModule::accounts(), Some(42));
	});
}
