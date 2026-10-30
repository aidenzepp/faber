//! Runner pins for map keys and set elements beyond the scalars: `none`, and
//! `class` values (faber#3), which key by their fields like a union variant.

use super::lib_test::evaluate_collection_call;
use crate::Value;
use crate::value::MapKey;
use handroll::{FxHashMap, FxHashSet};
use radix_lexer::{Span, Symbol};
use radix_mir::{MirCollectionOp, MirIntrinsic, MirOperand, MirRuntimeCall, MirType, MirValueId};
use radix_types::{DefId, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

fn call(op: MirCollectionOp, args: usize) -> MirRuntimeCall {
    MirRuntimeCall {
        intrinsic: MirIntrinsic::Collection(op),
        args: (0..args)
            .map(|index| MirOperand::Value(MirValueId(u32::try_from(index).expect("fits"))))
            .collect(),
        return_ty: MirType::semantic(TypeId(0)),
    }
}

fn run(op: MirCollectionOp, inputs: Vec<Value>) -> Value {
    let args = inputs.len();
    let bound = inputs
        .into_iter()
        .enumerate()
        .map(|(index, value)| (MirValueId(u32::try_from(index).expect("fits")), value))
        .collect();
    evaluate_collection_call(&call(op, args), bound).expect("collection call")
}

#[test]
fn none_is_a_set_element() {
    let set = Rc::new(RefCell::new(FxHashSet::default()));
    let receiver = || Value::Set(set.clone());
    run(MirCollectionOp::Append, vec![receiver(), Value::Int(5)]);
    run(MirCollectionOp::Append, vec![receiver(), Value::Nil]);
    run(MirCollectionOp::Append, vec![receiver(), Value::Int(5)]);
    run(MirCollectionOp::Append, vec![receiver(), Value::Nil]);
    assert_eq!(
        run(MirCollectionOp::Length, vec![receiver()]),
        Value::Int(2)
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), Value::Nil]),
        Value::Bool(true)
    );
    // The none element is stored as the none key and reads back as none.
    assert!(set.borrow().contains(&MapKey::Unit));
    assert_eq!(MapKey::Unit.to_value(), Value::Nil);
    assert_eq!(
        run(MirCollectionOp::Delete, vec![receiver(), Value::Nil]),
        Value::Bool(true)
    );
    assert_eq!(
        run(MirCollectionOp::Length, vec![receiver()]),
        Value::Int(1)
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), Value::Nil]),
        Value::Bool(false)
    );
}

#[test]
fn none_is_a_map_key() {
    let map = Rc::new(RefCell::new(FxHashMap::default()));
    map.borrow_mut()
        .insert(MapKey::Unit, Value::Str("none".to_owned()));
    map.borrow_mut()
        .insert(MapKey::UInt(5), Value::Str("five".to_owned()));
    let receiver = || Value::Map(map.clone());
    assert_eq!(
        run(MirCollectionOp::Length, vec![receiver()]),
        Value::Int(2)
    );
    assert_eq!(
        run(MirCollectionOp::Delete, vec![receiver(), Value::Nil]),
        Value::Bool(true)
    );
    assert_eq!(
        run(MirCollectionOp::Length, vec![receiver()]),
        Value::Int(1)
    );
}

const KEY: DefId = DefId(40);
const OTHER: DefId = DefId(41);
const NAME: Symbol = Symbol(7);
const COUNT: Symbol = Symbol(3);

/// A fresh `class` value, so equal keys never share a field cell.
fn class(def: DefId, fields: &[(Symbol, Value)]) -> Value {
    Value::Struct(def, Rc::new(RefCell::new(fields.iter().cloned().collect())))
}

fn named(name: &str) -> Value {
    class(KEY, &[(NAME, Value::Str(name.to_owned()))])
}

fn new_map() -> Rc<RefCell<FxHashMap<MapKey, Value>>> {
    Rc::new(RefCell::new(FxHashMap::default()))
}

/// `put` then `get` and `has` through the runner, each with a freshly built
/// equal key.
fn put_then_get(build: impl Fn() -> Value) {
    let map = new_map();
    let receiver = || Value::Map(map.clone());
    run(
        MirCollectionOp::Put,
        vec![receiver(), build(), Value::Int(1)],
    );
    assert_eq!(
        run(MirCollectionOp::Index, vec![receiver(), build()]),
        Value::Int(1)
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), build()]),
        Value::Bool(true)
    );
    // The stored key reads back (map iteration) as a value that finds the
    // same entry; an `int` part reads back through its canonical carrier.
    let span = Span::default();
    let keys: Vec<MapKey> = map
        .borrow()
        .keys()
        .map(|key| key.to_value().to_key(span).expect("read-back key"))
        .collect();
    assert_eq!(keys, vec![build().to_key(span).expect("key")]);
}

#[test]
fn string_key_control() {
    put_then_get(|| Value::Str("a".to_owned()));
}

#[test]
fn class_with_a_string_field_is_a_map_key() {
    put_then_get(|| named("a"));
    let key = named("a").to_key(Span::default()).expect("class key");
    assert_eq!(key.to_value(), named("a"));
}

#[test]
fn class_with_a_string_field_is_a_set_element() {
    let set = Rc::new(RefCell::new(FxHashSet::default()));
    let receiver = || Value::Set(set.clone());
    run(MirCollectionOp::Append, vec![receiver(), named("a")]);
    run(MirCollectionOp::Append, vec![receiver(), named("a")]);
    assert_eq!(
        run(MirCollectionOp::Length, vec![receiver()]),
        Value::Int(1)
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), named("a")]),
        Value::Bool(true)
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), named("b")]),
        Value::Bool(false)
    );
}

#[test]
fn class_with_two_fields_keys_by_both() {
    let pair = |n: i64| {
        class(
            KEY,
            &[(NAME, Value::Str("a".to_owned())), (COUNT, Value::Int(n))],
        )
    };
    put_then_get(|| pair(2));
    let map = new_map();
    let receiver = || Value::Map(map.clone());
    run(
        MirCollectionOp::Put,
        vec![receiver(), pair(2), Value::Int(1)],
    );
    assert_eq!(
        run(MirCollectionOp::Contains, vec![receiver(), pair(3)]),
        Value::Bool(false)
    );
}

#[test]
fn nested_class_is_a_map_key() {
    put_then_get(|| class(OTHER, &[(NAME, named("a"))]));
}

#[test]
fn class_inside_a_tuple_is_a_map_key() {
    put_then_get(|| Value::Tuple(vec![named("a"), Value::Int(2)]));
}

#[test]
fn classes_with_equal_fields_are_distinct_keys() {
    let span = Span::default();
    let key = named("a").to_key(span).expect("class key");
    let other = class(OTHER, &[(NAME, Value::Str("a".to_owned()))])
        .to_key(span)
        .expect("class key");
    assert_ne!(key, other);
}

#[test]
fn class_with_an_unhashable_field_is_not_a_key() {
    assert!(
        class(KEY, &[(NAME, Value::Float(1.5))])
            .to_key(Span::default())
            .is_err()
    );
}
