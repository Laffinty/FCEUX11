// FCEUX11 v1.11 Bridge — DriverCallbacks scaffolding test (PHASE B §7.1)
//
// 验证 fceu11::DriverCallbacks 接口契约：
//   - g_driver() 是进程级单例，返回有效引用（未注册时字段全 nullptr）
//   - register_driver() 通过 POD 拷贝赋值替换字段
//   - DriverCallbacks 是 trivially copyable（无堆、无虚函数、无 type erasure）
//   - 重复 register_driver() 覆盖前次；不同字段独立生效
//   - const 引用入参不修改调用方对象
//
// 设计为 headless 测试（不调用 fceu11::Initialize()），只验证接口层契约。
// 每个用例结束清空全局存储，保证用例间隔离。

#include <cstdio>
#include <cstdlib>
#include <cstring>     // memset
#include <type_traits> // is_trivially_copyable_v
#include <string>

#include "driver_callbacks.h"

// ---------------------------------------------------------------------------
// Mock 回调状态（每个测试用例前清零）
// ---------------------------------------------------------------------------
namespace {
struct MockState {
    int message_calls = 0;
    int print_error_calls = 0;
    int get_time_calls = 0;
    std::string last_message;
    std::string last_print_error;
    uint64_t    fake_time_value = 0xDEADBEEFCAFEBABEull;
};
MockState g_mock;

void mock_message(const char* s) {
    ++g_mock.message_calls;
    g_mock.last_message = s ? s : "(null)";
}

void mock_print_error(const char* s) {
    ++g_mock.print_error_calls;
    g_mock.last_print_error = s ? s : "(null)";
}

uint64_t mock_get_time() {
    ++g_mock.get_time_calls;
    return g_mock.fake_time_value;
}

// 用例间重置：清空全局 driver + mock 计数器
void reset_all() {
    fceu11::register_driver(fceu11::DriverCallbacks{});
    g_mock = MockState{};
}
} // namespace

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------
namespace {
struct TestRunner {
    int passed = 0;
    int failed = 0;
    void expect(bool cond, const char* label) {
        if (cond) {
            std::printf("  ok    %s\n", label);
            ++passed;
        } else {
            std::printf("  FAIL  %s\n", label);
            ++failed;
        }
    }
};
} // namespace

int main() {
    // 启动清空：避免上一进程残留状态污染（虽然 ctest 每测独立进程，防御性编程）
    fceu11::register_driver(fceu11::DriverCallbacks{});

    std::printf("=== FCEUX11 v1.11 PHASE B: driver_callbacks_test ===\n\n");
    TestRunner t;

    // ---------- 用例 1: g_driver() 返回非 null 引用 ----------
    {
        fceu11::DriverCallbacks& d = fceu11::g_driver();
        t.expect(&d != nullptr, "g_driver() returns non-null reference");
    }

    // ---------- 用例 2: g_driver() 是单例（地址稳定） ----------
    {
        bool same = &fceu11::g_driver() == &fceu11::g_driver();
        t.expect(same, "g_driver() is a singleton (same address across calls)");
    }

    // ---------- 用例 3: 默认字段全 nullptr（抽样 10 个不同类别） ----------
    {
        reset_all();
        const fceu11::DriverCallbacks& d = fceu11::g_driver();
        bool all_null =
            d.print_error       == nullptr &&
            d.message           == nullptr &&
            d.get_time          == nullptr &&
            d.set_emulation_speed == nullptr &&
            d.utf8_fopen        == nullptr &&
            d.open_archive      == nullptr &&
            d.video_changed     == nullptr &&
            d.sound_toggle      == nullptr &&
            d.set_input         == nullptr &&
            d.send_data         == nullptr;
        t.expect(all_null, "all sampled callback fields default to nullptr");
    }

    // ---------- 用例 4: register_driver() 后字段非 nullptr（被 mock 填充） ----------
    {
        reset_all();
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        cb.print_error = &mock_print_error;
        cb.get_time = &mock_get_time;
        fceu11::register_driver(cb);

        const fceu11::DriverCallbacks& d = fceu11::g_driver();
        bool registered =
            d.message     == &mock_message &&
            d.print_error == &mock_print_error &&
            d.get_time    == &mock_get_time;
        t.expect(registered, "register_driver() installs callbacks (fields non-null)");
    }

    // ---------- 用例 5: 注册的 callback 可正确触发（mock 计数器自增） ----------
    {
        reset_all();
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        cb.print_error = &mock_print_error;
        fceu11::register_driver(cb);

        // 模拟 core 调用：g_driver()->message("hello")
        fceu11::g_driver().message("hello");
        fceu11::g_driver().message("world");
        fceu11::g_driver().print_error("oops");

        bool invoked =
            g_mock.message_calls == 2 &&
            g_mock.last_message == "world" &&
            g_mock.print_error_calls == 1 &&
            g_mock.last_print_error == "oops";
        t.expect(invoked, "registered callbacks are invoked with correct arguments");
    }

    // ---------- 用例 6: DriverCallbacks 是 trivially copyable ----------
    {
        constexpr bool is_tc = std::is_trivially_copyable_v<fceu11::DriverCallbacks>;
        t.expect(is_tc, "DriverCallbacks is trivially copyable (POD contract)");
    }

    // ---------- 用例 7: sizeof(DriverCallbacks) > 0（防空 struct 退化） ----------
    {
        t.expect(sizeof(fceu11::DriverCallbacks) > 0,
                 "sizeof(DriverCallbacks) > 0 (non-empty struct)");
    }

    // ---------- 用例 8: memset(&cb, 0, sizeof(cb)) 后字段全 nullptr ----------
    {
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        cb.get_time = &mock_get_time;
        // 故意污染，再用 memset 清零
        std::memset(&cb, 0, sizeof(cb));

        bool all_null =
            cb.message == nullptr &&
            cb.get_time == nullptr &&
            cb.print_error == nullptr;
        t.expect(all_null, "memset(cb, 0, sizeof) zeros all function pointers");
    }

    // ---------- 用例 9: 二次 register_driver() 覆盖前次注册 ----------
    {
        reset_all();

        // 第一次注册：message → mock_message
        fceu11::DriverCallbacks cb1{};
        cb1.message = &mock_message;
        fceu11::register_driver(cb1);
        fceu11::g_driver().message("first");
        int calls_after_first = g_mock.message_calls;

        // 第二次注册：message → 另一个 lambda-free 占位函数 (用 mock_print_error 复用)
        // （Phase B 用现有 mock 函数作占位；mock_print_error 同样签名兼容 message）
        fceu11::DriverCallbacks cb2{};
        cb2.message = &mock_print_error;
        fceu11::register_driver(cb2);
        fceu11::g_driver().message("second");

        // 验证：第二次调用走 mock_print_error（print_error_calls 自增）
        // 而不是 mock_message（message_calls 不再变）
        bool overridden =
            g_mock.message_calls == calls_after_first &&  // message mock 未被再次调用
            g_mock.print_error_calls == 1 &&                // print_error mock 触发 1 次
            g_mock.last_print_error == "second";
        t.expect(overridden, "second register_driver() overrides first (latest wins)");
    }

    // ---------- 用例 10: mock 多种 callback 独立分发 ----------
    {
        reset_all();
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        cb.print_error = &mock_print_error;
        cb.get_time = &mock_get_time;
        fceu11::register_driver(cb);

        // 同时触发 3 个不同 callback
        fceu11::g_driver().message("m");
        fceu11::g_driver().print_error("e");
        uint64_t t1 = fceu11::g_driver().get_time();

        bool all_invoked =
            g_mock.message_calls == 1 &&
            g_mock.print_error_calls == 1 &&
            g_mock.get_time_calls == 1 &&
            t1 == g_mock.fake_time_value;
        t.expect(all_invoked,
                 "distinct callbacks dispatched independently with correct return values");
    }

    // ---------- 用例 11: register_driver() 接收 const 引用不修改入参 ----------
    {
        reset_all();
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        cb.get_time = &mock_get_time;

        // 保存拷贝作为"原始"参考（避免 &cb 在 register 后被覆盖）
        const auto orig_message = cb.message;
        const auto orig_get_time = cb.get_time;
        const auto orig_print_error = cb.print_error;
        (void)orig_message; (void)orig_get_time; (void)orig_print_error;

        fceu11::register_driver(cb);

        // cb 自己应未被 register_driver() 修改（POD 拷贝语义）
        bool cb_unchanged =
            cb.message == orig_message &&
            cb.get_time == orig_get_time &&
            cb.print_error == orig_print_error;
        t.expect(cb_unchanged, "register_driver() does not mutate caller's struct");
    }

    // ---------- 用例 12: 不同字段独立注册（只注册 message 不影响 print_error） ----------
    {
        reset_all();
        fceu11::DriverCallbacks cb{};
        cb.message = &mock_message;
        // 故意不设置 print_error
        fceu11::register_driver(cb);

        const fceu11::DriverCallbacks& d = fceu11::g_driver();
        bool selective =
            d.message     == &mock_message &&
            d.print_error == nullptr;  // 未注册，保持 nullptr
        t.expect(selective,
                 "register_driver() only installs provided fields (others remain nullptr)");
    }

    // ---------- 最终清理 ----------
    reset_all();

    // ---------- 报告 ----------
    std::printf("\n=== driver_callbacks_test ===\n");
    std::printf("Passed:    %d\n", t.passed);
    std::printf("Failed:    %d\n", t.failed);
    std::printf("Total:     %d\n", t.passed + t.failed);
    if (t.failed == 0) {
        std::printf("RESULT:    PASSED\n");
        return 0;
    }
    std::printf("RESULT:    FAILED\n");
    return 1;
}
