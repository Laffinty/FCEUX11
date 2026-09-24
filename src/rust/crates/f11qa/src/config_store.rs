// F11QA v1.8 — kgmqa-118 config_store
//
// Rust 端 `TypedConfig<T>` 双身，对应 C++ `tests/f11qa/config_store_test.cpp`
// 测的 `fceu11::qt::TypedConfig<T>` (Qt/ConfigStore.h)。
//
// C++ 原版绑定 QSettings (Qt INI 后端)；这里用 in-memory HashMap 保持纯 Rust。
// 行为契约（与 C++ 测试 1:1 对应）：
//   1. default value returned when key is absent
//   2. set/get round-trip is lossless for bool / int / String
//   3. is_set distinguishes "absent" from "set to default"
//   4. key accessor returns the literal that was passed in (本实现直接用 key 字符串，
//      通过 TypedConfigStore::new_with_seed(seed) 给每个测试实例分配独立命名空间，
//      模拟 C++ 端给每个 key 配独立 IniFormat 路径以避免污染)
//   5. Override-default at read time is honored
//
// 设计取舍：
//   - 用 `HashMap<String, ConfigValue>` 存储原始类型；get_* 接受 default 参数派生
//   - 不实现泛型 `TypedConfig<T>`（避免 type erasure / dyn 复杂度），改用具体类型方法
//   - 线程安全：单线程测试场景不需要 RwLock；保留单线程 `RefCell` 风格（不可变）
//     即 `TypedConfigStore { entries: HashMap<String, ConfigValue> }`

use std::collections::HashMap;

/// Rust 端的配置值类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

/// 纯 Rust `TypedConfig` 模拟层。
///
/// C++ 测试通过给每个 key 配独立 IniFormat 路径避免相互污染；本实现里
/// `TypedConfigStore` 实例本身就是隔离命名空间。
#[derive(Clone, Debug, Default)]
pub struct TypedConfigStore {
    entries: HashMap<String, ConfigValue>,
}

impl TypedConfigStore {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- bool ----

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        match self.entries.get(key) {
            Some(ConfigValue::Bool(b)) => *b,
            _ => default,
        }
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.entries
            .insert(key.to_string(), ConfigValue::Bool(value));
    }

    // ---- int ----

    pub fn get_int(&self, key: &str, default: i64) -> i64 {
        match self.entries.get(key) {
            Some(ConfigValue::Int(i)) => *i,
            _ => default,
        }
    }

    pub fn set_int(&mut self, key: &str, value: i64) {
        self.entries
            .insert(key.to_string(), ConfigValue::Int(value));
    }

    // ---- string ----

    pub fn get_str(&self, key: &str, default: &str) -> String {
        match self.entries.get(key) {
            Some(ConfigValue::Str(s)) => s.clone(),
            _ => default.to_string(),
        }
    }

    pub fn set_str(&mut self, key: &str, value: &str) {
        self.entries
            .insert(key.to_string(), ConfigValue::Str(value.to_string()));
    }

    // ---- meta ----

    pub fn is_set(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn remove(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}