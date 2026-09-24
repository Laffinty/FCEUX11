// F11QA v1.8 — kgmqa-118 config_store cargo test
//
// 对偶 C++ `tests/f11qa/config_store_test.cpp`：
//   TypedConfig<T> in-memory 模拟层（无 QSettings 依赖）。
//
// 跑：
//   cargo test -p f11qa --test config_store -- --nocapture

use f11qa::config_store::TypedConfigStore;

#[test]
fn default_value_returned_when_key_absent() {
    let store = TypedConfigStore::new();
    assert_eq!(store.get_bool("missing/bool", false), false);
    assert_eq!(store.get_bool("missing/bool", true), true);
    assert_eq!(store.get_int("missing/int", 42), 42);
    assert_eq!(store.get_int("missing/int", -1), -1);
    assert_eq!(store.get_str("missing/str", "fallback"), "fallback");
    println!("PASS: default value returned when key absent");
}

#[test]
fn set_get_roundtrip_lossless() {
    let mut store = TypedConfigStore::new();
    store.set_bool("k/bool", true);
    store.set_int("k/int", 12345);
    store.set_int("k/neg", -7);
    store.set_str("k/str", "hello f11qa");

    assert_eq!(store.get_bool("k/bool", false), true);
    assert_eq!(store.get_int("k/int", 0), 12345);
    assert_eq!(store.get_int("k/neg", 0), -7);
    assert_eq!(store.get_str("k/str", "default"), "hello f11qa");
    println!("PASS: set/get round-trip lossless for bool/int/String");
}

#[test]
fn is_set_distinguishes_absent_from_set_to_default() {
    let mut store = TypedConfigStore::new();

    // unset key
    assert_eq!(store.is_set("k/unset"), false);
    assert_eq!(store.get_bool("k/unset", false), false);

    // set to default value (false) — key must be present
    store.set_bool("k/set_default_false", false);
    assert_eq!(store.is_set("k/set_default_false"), true);
    assert_eq!(store.get_bool("k/set_default_false", true), false);

    // set to non-default value
    store.set_bool("k/set_true", true);
    assert_eq!(store.is_set("k/set_true"), true);
    assert_eq!(store.get_bool("k/set_true", false), true);

    // explicit remove clears is_set
    assert_eq!(store.remove("k/set_true"), true);
    assert_eq!(store.is_set("k/set_true"), false);
    assert_eq!(store.remove("k/set_true"), false);
    println!("PASS: is_set distinguishes absent from set-to-default; remove() works");
}

#[test]
fn override_default_honored_at_read() {
    // Override default at read time: setting a value means get returns stored
    // value, NOT the default supplied at the call site.
    let mut store = TypedConfigStore::new();
    store.set_int("k/stored", 999);
    // Even if caller passes 0 as default, get should return 999.
    assert_eq!(store.get_int("k/stored", 0), 999);
    // Sanity: missing key with default 0 returns 0.
    assert_eq!(store.get_int("k/missing", 0), 0);
    println!("PASS: override-default at read time honored");
}

#[test]
fn string_default_owned_no_aliasing() {
    // C++ 测试里要求 set("") 之后的 get_str("") 返回 ""（不是 default 的内存别名）。
    // Rust String 已经保证 owned，无 aliasing 风险；本测试覆盖 round-trip 一致性。
    let mut store = TypedConfigStore::new();
    store.set_str("k/empty", "");
    assert_eq!(store.get_str("k/empty", "default"), "");
    store.set_str("k/utf8", "测试 / F11QA ✓");
    assert_eq!(store.get_str("k/utf8", ""), "测试 / F11QA ✓");
    println!("PASS: string default owned; UTF-8 round-trip");
}