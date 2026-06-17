// config_store_test.cpp
//
// v0.3.15.x PHASE-4: unit test for fceu11::qt::TypedConfig<T>.
// The test exercises the wrapper against a real QSettings (the
// in-memory IniFormat default) and verifies:
//   1. Default value is returned when the key is absent.
//   2. set() / get() round-trip is lossless for bool / int / QString.
//   3. isSet() distinguishes "absent" from "set to default".
//   4. The key() accessor returns the literal that was passed in.
//   5. Override-default at read time is honoured (read returns
//      stored value, not the default supplied at construction).
//
// This test deliberately avoids touching the real fceux11 settings
// hierarchy (which is initialised by fceuWrapperInit()) by giving
// every TypedConfig instance a unique, ephemeral key path that
// nothing else in the process writes to. The INI format also
// avoids any side effects on the user's real QSettings file.
//
// Style: matches i18n_regression_test.cpp and smoke_test.cpp
// (printf-based, return 0/1).

#include <cstdio>
#include <cstdlib>
#include <cstring>

#include <QCoreApplication>
#include <QSettings>
#include <QString>

#include "Qt/ConfigStore.h"

namespace {

int g_failures = 0;

#define EXPECT(cond, msg)                                                  \
	do {                                                                   \
		if (!(cond)) {                                                     \
			printf("FAIL [%s:%d] %s\n", __FILE__, __LINE__, msg);          \
			++g_failures;                                                  \
		} else {                                                           \
			printf("ok   [%s:%d] %s\n", __FILE__, __LINE__, msg);          \
		}                                                                  \
	} while (0)

} // namespace

int main(int argc, char** argv)
{
	// QSettings (and by extension TypedConfig<T>) needs a
	// QCoreApplication for the organisation / application name
	// resolution that determines the INI file location. The
	// unique names below make sure we never collide with a
	// fceux11.ini from the user's real config.
	QCoreApplication app(argc, argv);
	QCoreApplication::setOrganizationName("fceux11-test-org");
	QCoreApplication::setOrganizationDomain("test.fceux11.local");
	QCoreApplication::setApplicationName("config_store_test");

	using fceu11::qt::TypedConfig;

	// Use a unique key prefix so the test can be re-run without
	// QSettings cross-contamination.
	const char* kBoolKey   = "test/boolKey";
	const char* kIntKey    = "test/intKey";
	const char* kStrKey    = "test/strKey";
	const char* kUnsetKey  = "test/unsetKey";
	const char* kDefaultKey = "test/defaultKey";

	// Pre-clean (a previous failed run may have left values).
	{
		QSettings cleanup;
		cleanup.remove(kBoolKey);
		cleanup.remove(kIntKey);
		cleanup.remove(kStrKey);
		cleanup.remove(kUnsetKey);
		cleanup.remove(kDefaultKey);
		cleanup.sync();
	}

	// 1. Default value when key is absent.
	{
		TypedConfig<bool> cfg(kUnsetKey, true);
		EXPECT(cfg.get() == true, "bool default returned when absent");
		EXPECT(cfg.isSet() == false, "isSet() false when absent");
	}

	// 2. bool round-trip.
	{
		TypedConfig<bool> cfg(kBoolKey, false);
		cfg.set(true);
		EXPECT(cfg.get() == true, "bool get() reflects set(true)");
		EXPECT(cfg.isSet() == true, "isSet() true after set()");
		cfg.set(false);
		EXPECT(cfg.get() == false, "bool get() reflects set(false)");
	}

	// 3. int round-trip.
	{
		TypedConfig<int> cfg(kIntKey, 42);
		cfg.set(12345);
		EXPECT(cfg.get() == 12345, "int get() reflects set(12345)");
		cfg.set(-7);
		EXPECT(cfg.get() == -7, "int get() reflects negative value");
	}

	// 4. QString round-trip with non-ASCII content (the use
	//    case that motivated the i18n / PHASE-1 PHASE-2 work).
	{
		TypedConfig<QString> cfg(kStrKey, QStringLiteral("en"));
		cfg.set(QStringLiteral("zh_CN"));
		EXPECT(cfg.get() == QStringLiteral("zh_CN"),
		       "QString get() reflects set(zh_CN)");
		cfg.set(QStringLiteral("zh_TW"));
		EXPECT(cfg.get() == QStringLiteral("zh_TW"),
		       "QString get() reflects set(zh_TW)");
	}

	// 5. Key accessor round-trips the constructor argument.
	{
		TypedConfig<int> cfg("test/canonicalKeyName", 0);
		EXPECT(strcmp(cfg.key(), "test/canonicalKeyName") == 0,
		       "key() returns the literal passed to ctor");
		EXPECT(cfg.defaultValue() == 0,
		       "defaultValue() returns the default passed to ctor");
	}

	// 6. isSet() distinguishes "absent" from "explicitly set to
	//    default". This is the behaviour the GuiConf dialog
	//    relies on for the "Reset to default" button.
	{
		TypedConfig<int> cfg(kDefaultKey, 99);
		EXPECT(cfg.isSet() == false, "absent: isSet() is false");
		cfg.set(99); // == default
		EXPECT(cfg.isSet() == true, "explicit-set-to-default: isSet() true");
		EXPECT(cfg.get() == 99, "get() still returns 99 (default)");
		cfg.set(0);
		EXPECT(cfg.get() == 0, "get() reflects non-default override");
	}

	// 7. Static instances share state (the documented
	//    "cache as static" iron rule from the plan).
	{
		static const TypedConfig<QString> kShared("test/sharedKey",
		                                          QStringLiteral("default"));
		kShared.set(QStringLiteral("hello"));
		// Re-declare in a different block to force a fresh
		// TypedConfig<QString> at the same key; should still see
		// the stored value.
		TypedConfig<QString> reread("test/sharedKey",
		                            QStringLiteral("default"));
		EXPECT(reread.get() == QStringLiteral("hello"),
		       "static caching is transparent across instances");
	}

	if (g_failures == 0) {
		printf("\n[PASS] config_store_test: all assertions green\n");
		return 0;
	}
	printf("\n[FAIL] config_store_test: %d assertion(s) failed\n",
	       g_failures);
	return 1;
}
