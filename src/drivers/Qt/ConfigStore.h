// ConfigStore.h
//
// v0.3.15.x PHASE-4: TypedConfig<T> typed wrapper around QSettings.
//
// Iron rules:
//   * TypedConfig instances are constructed lazily on first use and
//     cached as `static const` locals at every call site; do NOT
//     construct one per-call. The wrapper itself is cheap, but the
//     QSettings lookup it triggers is not — caching the static
//     keeps the .key() string and the default value computed once.
//   * Keys are static string literals (const char*). Storing a
//     QString or std::string in the key slot would defeat the
//     compile-time intent and would not be detectable at runtime.
//
// The template defers the actual T <-> QVariant conversion to Qt's
// QVariant::value<T>() machinery; for the types we use (bool, int,
// QString) the conversion is exact and lossless. For custom types
// (enums, etc.) register QMetaType via Q_DECLARE_METATYPE before
// instantiating TypedConfig<T>.

#pragma once

#include <QSettings>
#include <QVariant>
#include <type_traits>

namespace fceu11::qt {

template <typename T>
class TypedConfig {
public:
	/// Construct a typed accessor for the given QSettings key with
	/// a default value used when the key is missing.
	constexpr TypedConfig(const char* key, T defaultValue)
		: m_key(key), m_default(defaultValue) {}

	/// Read the value; returns the default if the key is missing
	/// or the QVariant cannot be coerced to T.
	T get() const {
		QSettings settings;
		return settings.value(m_key, m_default).value<T>();
	}

	/// Write the value back to QSettings. The change is flushed
	/// lazily by QSettings' own sync() policy; callers that need
	/// the value on disk immediately can call QSettings().sync().
	void set(const T& v) const {
		QSettings settings;
		settings.setValue(m_key, v);
	}

	/// Returns true iff the underlying QVariant for this key is
	/// not QVariant::Invalid (i.e. the key was previously written).
	/// Useful for distinguishing "absent" from "explicitly set to
	/// the default value".
	bool isSet() const {
		QSettings settings;
		return settings.value(m_key).isValid();
	}

	/// The QSettings key. Exposed for diagnostics (e.g. an error
	/// message that wants to print which key failed to parse).
	const char* key() const { return m_key; }

	/// The default value. Exposed for read-only default-override
	/// logic (e.g. "if (!config.isSet()) config.set(default);").
	T defaultValue() const { return m_default; }

private:
	const char* m_key;
	T           m_default;
};

/// Convenience factory: builds a TypedConfig<T> with the type
/// deduced from the default value. Equivalent to
/// `TypedConfig<T>(key, default)`.
template <typename T>
constexpr TypedConfig<T> makeConfig(const char* key, T defaultValue)
{
	return TypedConfig<T>(key, defaultValue);
}

} // namespace fceu11::qt
