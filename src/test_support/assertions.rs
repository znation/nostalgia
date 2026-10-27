//! Serde-contract and id-list assertions shared by the crate's tests.

/// Asserts that `T` requires every field `payload` declares: for each key,
/// removing that one key must fail deserialization, because a payload missing
/// a required field must error rather than silently yield a half-populated
/// value the UI would render as blank data. The four model-type tests
/// (`Artist`, `Album`, `Song`, and `Playlist`) each hand in a full, valid
/// payload, so the walk-every-key loop lives here once. Probing every key
/// matters: a single named field per type leaves the others unpinned, and a
/// `#[serde(default)]` added to a field no probe omitted would clear the
/// suite while blanking that field.
pub(crate) fn assert_every_field_required<T>(payload: serde_json::Value)
where
    T: serde::de::DeserializeOwned,
{
    let object = payload
        .as_object()
        .expect("a full payload must be a JSON object");
    assert!(
        !object.is_empty(),
        "a full payload must declare at least one field"
    );
    for key in object.keys() {
        let mut without = object.clone();
        without.remove(key);
        assert!(
            serde_json::from_value::<T>(serde_json::Value::Object(without)).is_err(),
            "a payload missing required field {key:?} must be rejected"
        );
    }
}

/// Asserts that `value` serializes to exactly `expected` and then survives an
/// out-and-back trip through `serde_json` unchanged. The four model-type
/// tests (`Artist`, `Album`, `Song`, and `Playlist`) each pin the same two
/// halves — the exact serialized field names (a `#[serde(rename)]` would pass
/// a round-trip alone but fails this pin), then the round trip via
/// [`assert_round_trips`] — so the serialize-then-compare-then-round-trip
/// sequence lives here once and no caller can pin one half without the
/// other.
pub(crate) fn assert_serializes_as<T>(value: T, expected: serde_json::Value)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    assert_eq!(serde_json::to_value(&value).unwrap(), expected);
    assert_round_trips(value);
}

/// Asserts that `value` survives an out-and-back trip through `serde_json`
/// unchanged: serialize it, deserialize the result, and compare. The
/// round-trip half of [`assert_serializes_as`], kept separate so the chain has
/// one named implementation. Private to this module: [`assert_serializes_as`]
/// is the only entry point, so no caller can pin the round trip alone.
fn assert_round_trips<T>(value: T)
where
    T: PartialEq + std::fmt::Debug + serde::Serialize + serde::de::DeserializeOwned,
{
    let back: T = serde_json::from_value(serde_json::to_value(&value).unwrap()).unwrap();
    assert_eq!(back, value);
}

/// Asserts that `payload` deserializes as `T` to exactly `expected`, proving
/// serde's default tolerance of fields beyond `T`'s declared set: a real
/// Apple Music payload carries more than the model's fields, so an unknown
/// field must be ignored rather than failing the whole parse. The four
/// model-type tests (`Artist`, `Album`, `Song`, and `Playlist`) each probe
/// this contract, so the `from_value`-then-`assert_eq` chain lives here once.
pub(crate) fn assert_unknown_fields_tolerated<T>(payload: serde_json::Value, expected: T)
where
    T: PartialEq + std::fmt::Debug + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(payload).expect("unknown fields must be tolerated");
    assert_eq!(parsed, expected);
}

/// Asserts that `items` yield exactly the expected ids, in order.
///
/// The browse-query tests in the `apple_music`, `apple_music::rest` client,
/// and `ui` suites each fetch a list and pin its exact contents by mapping the
/// list to its ids and comparing; only the id projection and the expected ids
/// differ, so the map-to-ids-then-compare chain lives here once and each call
/// site only names its list and expectation. Comparing the ids *in order* is what
/// catches a dropped or reordered element that a bare non-empty assertion
/// would miss. The `sample_library` suite's `assert_id_label_pairs` is a
/// distinct helper — it pins each item's secondary label alongside its id,
/// not the id alone — so it stays local to that suite.
pub(crate) fn assert_ids<T>(items: &[T], id: impl Fn(&T) -> &str, expected: &[&str]) {
    let ids: Vec<&str> = items.iter().map(id).collect();
    assert_eq!(ids, expected);
}
