use std::cell::RefCell;
use std::rc::Rc;

use haproxy_api::Txn;
use mlua::{FromLua, LightUserData, Lua, Table, Value};

fn txn_class(lua: &Lua) -> Table {
    let class = lua.create_table().unwrap();
    class.set("c", lua.create_table().unwrap()).unwrap();
    class.set("f", lua.create_table().unwrap()).unwrap();
    class
}

#[test]
fn opaque_transaction_slot_round_trips_and_transfers_ownership() {
    let lua = Lua::new();
    let class = txn_class(&lua);
    let mut first = 0_u8;
    let mut second = 0_u8;
    let first = LightUserData((&mut first as *mut u8).cast());
    let second = LightUserData((&mut second as *mut u8).cast());
    let slot = Rc::new(RefCell::new(None));

    class.set("__txn_slot_supported", true).unwrap();
    class.set("__txn_slot", first).unwrap();
    class
        .set(
            "set_txn_slot",
            lua.create_function({
                let slot = Rc::clone(&slot);
                move |_, (_txn, value, destroy): (Table, LightUserData, LightUserData)| {
                    assert!(!value.0.is_null());
                    assert!(!destroy.0.is_null());
                    *slot.borrow_mut() = Some(value);
                    Ok(())
                }
            })
            .unwrap(),
        )
        .unwrap();
    class
        .set(
            "take_txn_slot",
            lua.create_function({
                let slot = Rc::clone(&slot);
                move |_, ()| Ok(slot.borrow_mut().take())
            })
            .unwrap(),
        )
        .unwrap();

    let txn = Txn::from_lua(Value::Table(class), &lua).unwrap();
    assert_eq!(txn.get_txn_slot().unwrap().unwrap().0, first.0);

    txn.set_txn_slot(second, first).unwrap();
    assert_eq!(txn.take_txn_slot().unwrap().unwrap().0, second.0);
    assert!(txn.take_txn_slot().unwrap().is_none());
}
