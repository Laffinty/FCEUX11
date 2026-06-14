// FCEUX11 v0.3.13 — XInput backend implementation.

#include "xinput_backend.h"

#include <cstdio>

namespace fceu11 {
namespace input {

// Button id -> XINPUT_GAMEPAD_* mask mapping.
// Order mirrors common NES gamepad conventions; exact mapping to the
// emulator's logical buttons is performed by GamePadConf / input.cpp.
static constexpr WORD kXInputButtonMasks[] = {
    XINPUT_GAMEPAD_A,
    XINPUT_GAMEPAD_B,
    XINPUT_GAMEPAD_X,
    XINPUT_GAMEPAD_Y,
    XINPUT_GAMEPAD_LEFT_SHOULDER,
    XINPUT_GAMEPAD_RIGHT_SHOULDER,
    XINPUT_GAMEPAD_BACK,
    XINPUT_GAMEPAD_START,
    XINPUT_GAMEPAD_LEFT_THUMB,
    XINPUT_GAMEPAD_RIGHT_THUMB,
    XINPUT_GAMEPAD_DPAD_UP,
    XINPUT_GAMEPAD_DPAD_DOWN,
    XINPUT_GAMEPAD_DPAD_LEFT,
    XINPUT_GAMEPAD_DPAD_RIGHT,
};
static constexpr int kXInputButtonCount =
    static_cast<int>(sizeof(kXInputButtonMasks) / sizeof(kXInputButtonMasks[0]));

class XInputDevice : public InputDevice {
public:
    explicit XInputDevice(int userIndex, XInputBackend* owner)
        : userIndex_(userIndex), owner_(owner) {}

    const char* name() const override {
        if (!connected_) return "XInput (disconnected)";
        return "XInput Controller";
    }

    bool connected() const override { return connected_; }

    bool button(int buttonId) const override {
        if (!connected_ || buttonId < 0 || buttonId >= kXInputButtonCount) {
            return false;
        }
        return (state_.Gamepad.wButtons & kXInputButtonMasks[buttonId]) != 0;
    }

    void poll() override {
        connected_ = false;
        if (!owner_) return;

        XINPUT_STATE state{};
        if (owner_->getState(userIndex_, state) == ERROR_SUCCESS) {
            state_ = state;
            connected_ = true;
        }
    }

    void rumble(float lowFrequency, float highFrequency,
                uint32_t durationMs) override {
        if (!owner_) return;
        owner_->setVibration(
            userIndex_,
            static_cast<WORD>(lowFrequency * 65535.0f),
            static_cast<WORD>(highFrequency * 65535.0f));
        // XInput vibration is one-shot; duration is handled by the caller
        // if continuous rumble is required.  For our simple API we just fire
        // and forget.
        (void)durationMs;
    }

private:
    int userIndex_;
    XInputBackend* owner_;
    XINPUT_STATE state_{};
    bool connected_ = false;
};

XInputBackend::XInputBackend() = default;

XInputBackend::~XInputBackend() {
    shutdown();
}

bool XInputBackend::initialize() {
    if (dll_) return true;

    dll_ = LoadLibraryW(L"xinput1_4.dll");
    if (!dll_) {
        std::printf("XInputBackend: LoadLibrary(xinput1_4.dll) failed (0x%08X).\n",
                    static_cast<unsigned int>(GetLastError()));
        return false;
    }

    getState_ = reinterpret_cast<XInputGetStateProc>(
        GetProcAddress(dll_, "XInputGetState"));
    setState_ = reinterpret_cast<XInputSetStateProc>(
        GetProcAddress(dll_, "XInputSetState"));

    if (!getState_ || !setState_) {
        std::printf("XInputBackend: GetProcAddress failed.\n");
        shutdown();
        return false;
    }

    for (int i = 0; i < MAX_DEVICES; ++i) {
        devices_[i] = std::make_unique<XInputDevice>(i, this);
    }

    return true;
}

void XInputBackend::shutdown() {
    for (int i = 0; i < MAX_DEVICES; ++i) {
        devices_[i].reset();
    }

    if (dll_) {
        FreeLibrary(dll_);
        dll_ = nullptr;
    }
    getState_ = nullptr;
    setState_ = nullptr;
}

void XInputBackend::enumerate() {
    // XInput has fixed 4 slots; connection status is discovered during poll().
}

InputDevice* XInputBackend::device(std::size_t index) {
    if (index >= static_cast<std::size_t>(MAX_DEVICES)) return nullptr;
    return devices_[index].get();
}

void XInputBackend::pollAll() {
    for (int i = 0; i < MAX_DEVICES; ++i) {
        if (devices_[i]) {
            devices_[i]->poll();
        }
    }
}

bool XInputBackend::getState(int index, XINPUT_STATE& state) const {
    if (!getState_ || index < 0 || index >= MAX_DEVICES) return false;
    return getState_(static_cast<DWORD>(index), &state) == ERROR_SUCCESS;
}

void XInputBackend::setVibration(int index, WORD leftMotorSpeed,
                                 WORD rightMotorSpeed) {
    if (!setState_ || index < 0 || index >= MAX_DEVICES) return;
    XINPUT_VIBRATION vib{};
    vib.wLeftMotorSpeed = leftMotorSpeed;
    vib.wRightMotorSpeed = rightMotorSpeed;
    setState_(static_cast<DWORD>(index), &vib);
}

} // namespace input
} // namespace fceu11
