// Dynarmic cores share the monitor and RAM but own JIT state. Fastmem is used until GPU mappings
// need write observation; then the core runs through callbacks so dirty pages and exclusives agree.
#include <dynarmic/interface/A64/a64.h>
#include <dynarmic/interface/exclusive_monitor.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cstdint>
#include <cstring>
#include <limits>
#include <memory>
#include <utility>

namespace {
constexpr std::uint64_t tick_hz = 19'200'000;
using WriteCallback = void (*)(void*, std::uint64_t, std::size_t);

struct Context {
    std::uint64_t x[31];
    std::uint64_t v[64];
    std::uint64_t sp;
    std::uint64_t pc;
    std::uint64_t tls;
    std::uint64_t tpidr;
    std::uint64_t daif;
    std::uint32_t pstate;
    std::uint32_t fpcr;
    std::uint32_t fpsr;
};

struct Monitor { Dynarmic::ExclusiveMonitor value{8}; };

struct Callbacks final : Dynarmic::A64::UserCallbacks {
    std::uint8_t* memory{};
    std::uint64_t size{};
    WriteCallback write_callback{};
    void* write_data{};
    Dynarmic::A64::Jit* jit{};
    std::uint64_t tls{};
    std::uint64_t tpidr{};
    std::uint64_t daif{};
    std::uint64_t ticks{};
    std::uint64_t deadline{std::numeric_limits<std::uint64_t>::max()};
    std::uint64_t tick_scale{192};
    std::uint32_t svc{std::numeric_limits<std::uint32_t>::max()};
    std::uint32_t core{};
    bool virtual_time{};
    bool fault{};
    bool fallback_resume{};

    std::uint64_t now() const {
        if (virtual_time) return ticks;
        const auto ns = std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now().time_since_epoch()).count();
        return static_cast<std::uint64_t>(ns) * tick_hz / 1'000'000'000;
    }
    bool valid(std::uint64_t address, std::size_t length) {
        if (address <= size && length <= size - address) return true;
        fault = true;
        if (jit) jit->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
        return false;
    }
    template<typename T> T read(std::uint64_t address) {
        T value{};
        if (valid(address, sizeof(value))) std::memcpy(&value, memory + address, sizeof(value));
        return value;
    }
    template<typename T> void write(std::uint64_t address, T value) {
        if (!valid(address, sizeof(value))) return;
        std::memcpy(memory + address, &value, sizeof(value));
        if (write_callback) write_callback(write_data, address, sizeof(value));
    }
    template<typename T> bool exclusive(std::uint64_t address, T value, T expected) {
        if (!valid(address, sizeof(value))) return false;
        T current;
        std::memcpy(&current, memory + address, sizeof(current));
        if (std::memcmp(&current, &expected, sizeof(T)) != 0) return false;
        write(address, value);
        return true;
    }

    std::optional<std::uint32_t> MemoryReadCode(std::uint64_t a) override {
        if (!valid(a, 4)) return std::nullopt;
        return read<std::uint32_t>(a);
    }
    std::uint8_t MemoryRead8(std::uint64_t a) override { return read<std::uint8_t>(a); }
    std::uint16_t MemoryRead16(std::uint64_t a) override { return read<std::uint16_t>(a); }
    std::uint32_t MemoryRead32(std::uint64_t a) override { return read<std::uint32_t>(a); }
    std::uint64_t MemoryRead64(std::uint64_t a) override { return read<std::uint64_t>(a); }
    Dynarmic::A64::Vector MemoryRead128(std::uint64_t a) override { return {read<std::uint64_t>(a), read<std::uint64_t>(a + 8)}; }
    void MemoryWrite8(std::uint64_t a, std::uint8_t v) override { write(a, v); }
    void MemoryWrite16(std::uint64_t a, std::uint16_t v) override { write(a, v); }
    void MemoryWrite32(std::uint64_t a, std::uint32_t v) override { write(a, v); }
    void MemoryWrite64(std::uint64_t a, std::uint64_t v) override { write(a, v); }
    void MemoryWrite128(std::uint64_t a, Dynarmic::A64::Vector v) override { write(a, v[0]); write(a + 8, v[1]); }
    bool MemoryWriteExclusive8(std::uint64_t a, std::uint8_t v, std::uint8_t e) override { return exclusive(a, v, e); }
    bool MemoryWriteExclusive16(std::uint64_t a, std::uint16_t v, std::uint16_t e) override { return exclusive(a, v, e); }
    bool MemoryWriteExclusive32(std::uint64_t a, std::uint32_t v, std::uint32_t e) override { return exclusive(a, v, e); }
    bool MemoryWriteExclusive64(std::uint64_t a, std::uint64_t v, std::uint64_t e) override { return exclusive(a, v, e); }
    bool MemoryWriteExclusive128(std::uint64_t a, Dynarmic::A64::Vector v, Dynarmic::A64::Vector e) override { return exclusive(a, v, e); }
    void InterpreterFallback(std::uint64_t pc, std::size_t count) override {
        if (count != 1 || !valid(pc, 4)) { fault = true; jit->HaltExecution(); return; }
        const auto instruction = read<std::uint32_t>(pc);
        const auto reg = instruction & 31;
        switch (instruction & ~31U) {
        case 0xd51b4220: daif = jit->GetRegister(reg); break;
        case 0xd53b4220: jit->SetRegister(reg, daif); break;
        case 0xd5384240: jit->SetRegister(reg, 4); break;
        case 0xd5384200: jit->SetRegister(reg, 1); break;
        case 0xd53800a0: jit->SetRegister(reg, 0x8000'0000 | core); break;
        default: fault = true; jit->HaltExecution(); return;
        }
        jit->SetPC(pc + 4);
        fallback_resume = true;
        jit->HaltExecution(Dynarmic::HaltReason::UserDefined2);
    }
    void CallSVC(std::uint32_t value) override { svc = value; jit->HaltExecution(); }
    void ExceptionRaised(std::uint64_t, Dynarmic::A64::Exception) override { fault = true; jit->HaltExecution(); }
    void InstructionCacheOperationRaised(Dynarmic::A64::InstructionCacheOperation, std::uint64_t) override { jit->ClearCache(); }
    void AddTicks(std::uint64_t value) override { if (virtual_time) ticks += value * tick_scale; }
    std::uint64_t GetTicksRemaining() override { return deadline - std::min(deadline, now()); }
    std::uint64_t GetCNTPCT() override { return now(); }
};

struct Cpu {
    Cpu(Monitor* monitor_, std::size_t id_, std::uint8_t* memory_, std::uint64_t size_, WriteCallback callback_, void* data_)
        : monitor(monitor_), id(id_) {
        callbacks.memory = memory_;
        callbacks.size = size_;
        callbacks.write_callback = callback_;
        callbacks.write_data = data_;
        rebuild(true);
    }
    Dynarmic::A64::UserConfig config(bool fastmem) {
        Dynarmic::A64::UserConfig config{&callbacks};
        config.processor_id = id;
        config.global_monitor = &monitor->value;
        config.cntfrq_el0 = tick_hz;
        config.tpidrro_el0 = &callbacks.tls;
        config.tpidr_el0 = &callbacks.tpidr;
        config.fastmem_exclusive_access = false;
        config.silently_mirror_fastmem = false;
        config.fastmem_address_space_bits = 34;
        if (fastmem) config.fastmem_pointer = reinterpret_cast<std::uintptr_t>(callbacks.memory);
        return config;
    }
    void save(Context& out) const {
        const auto x = jit->GetRegisters();
        const auto v = jit->GetVectors();
        std::copy(x.begin(), x.end(), out.x);
        for (std::size_t i = 0; i < v.size(); ++i) { out.v[i * 2] = v[i][0]; out.v[i * 2 + 1] = v[i][1]; }
        out.sp = jit->GetSP(); out.pc = jit->GetPC(); out.tls = callbacks.tls; out.tpidr = callbacks.tpidr; out.daif = callbacks.daif;
        out.pstate = jit->GetPstate(); out.fpcr = jit->GetFpcr(); out.fpsr = jit->GetFpsr();
    }
    void load(const Context& in) {
        std::array<std::uint64_t, 31> x; std::copy(std::begin(in.x), std::end(in.x), x.begin());
        std::array<Dynarmic::A64::Vector, 32> v;
        for (std::size_t i = 0; i < v.size(); ++i) v[i] = {in.v[i * 2], in.v[i * 2 + 1]};
        jit->SetRegisters(x); jit->SetVectors(v); jit->SetSP(in.sp); jit->SetPC(in.pc); callbacks.tls = in.tls; callbacks.tpidr = in.tpidr; callbacks.daif = in.daif;
        jit->SetPstate(in.pstate); jit->SetFpcr(in.fpcr); jit->SetFpsr(in.fpsr);
    }
    void rebuild(bool fastmem) {
        Context state{};
        if (jit) save(state);
        jit.reset();
        jit = std::make_unique<Dynarmic::A64::Jit>(config(fastmem));
        callbacks.jit = jit.get();
        if (initialized) load(state);
        initialized = true;
        using_fastmem = fastmem;
    }
    Monitor* monitor;
    std::size_t id;
    Callbacks callbacks;
    std::unique_ptr<Dynarmic::A64::Jit> jit;
    bool initialized{};
    bool using_fastmem{};
};
}

extern "C" {
Monitor* oboromi_dynarmic_monitor_new() { return new Monitor; }
void oboromi_dynarmic_monitor_free(Monitor* monitor) { delete monitor; }
Cpu* oboromi_dynarmic_cpu_new(Monitor* monitor, std::size_t id, std::uint8_t* memory, std::uint64_t size, WriteCallback callback, void* data) {
    try { return new Cpu(monitor, id, memory, size, callback, data); } catch (...) { return nullptr; }
}
void oboromi_dynarmic_cpu_free(Cpu* cpu) { delete cpu; }
void oboromi_dynarmic_set_watched(Cpu* cpu, bool watched) { if (cpu->using_fastmem == watched) cpu->rebuild(!watched); }
void oboromi_dynarmic_set_virtual_time(Cpu* cpu, bool enabled) { cpu->callbacks.virtual_time = enabled; }
void oboromi_dynarmic_set_tick_scale(Cpu* cpu, std::uint64_t scale) { cpu->callbacks.tick_scale = scale; }
std::uint64_t oboromi_dynarmic_ticks(const Cpu* cpu) { return cpu->callbacks.now(); }
bool oboromi_dynarmic_idle_until(Cpu* cpu, std::uint64_t deadline) {
    if (cpu->callbacks.virtual_time) cpu->callbacks.ticks = std::max(cpu->callbacks.ticks, deadline);
    return cpu->callbacks.virtual_time;
}
bool oboromi_dynarmic_run(Cpu* cpu, std::uint64_t deadline, std::uint32_t core, bool* sliced) {
    cpu->callbacks.deadline = deadline; cpu->callbacks.svc = std::numeric_limits<std::uint32_t>::max(); cpu->callbacks.fault = false;
    cpu->callbacks.core = core;
    cpu->jit->ClearHalt();
    Dynarmic::HaltReason reason{};
    for (;;) {
        reason = cpu->jit->Run();
        if (Dynarmic::Has(reason, Dynarmic::HaltReason::CacheInvalidation)) continue;
        if (cpu->callbacks.fallback_resume) {
            cpu->callbacks.fallback_resume = false;
            // Keep a fallback system-register instruction and its following store or SVC in one slice.
            cpu->callbacks.deadline = std::max(cpu->callbacks.deadline, cpu->callbacks.now() + 4 * cpu->callbacks.tick_scale);
            continue;
        }
        break;
    }
    *sliced = !cpu->callbacks.fault && cpu->callbacks.svc == std::numeric_limits<std::uint32_t>::max() && !reason;
    cpu->callbacks.deadline = std::numeric_limits<std::uint64_t>::max();
    return !cpu->callbacks.fault;
}
bool oboromi_dynarmic_step(Cpu* cpu) { cpu->callbacks.fault = false; cpu->jit->ClearHalt(); cpu->jit->Step(); return !cpu->callbacks.fault; }
void oboromi_dynarmic_halt(Cpu* cpu) { cpu->jit->HaltExecution(); }
std::uint64_t oboromi_dynarmic_get_x(const Cpu* cpu, std::size_t index) { return index < 31 ? cpu->jit->GetRegister(index) : 0; }
void oboromi_dynarmic_set_x(Cpu* cpu, std::size_t index, std::uint64_t value) { if (index < 31) cpu->jit->SetRegister(index, value); }
std::uint64_t oboromi_dynarmic_get_sp(const Cpu* cpu) { return cpu->jit->GetSP(); }
void oboromi_dynarmic_set_sp(Cpu* cpu, std::uint64_t value) { cpu->jit->SetSP(value); }
std::uint64_t oboromi_dynarmic_get_pc(const Cpu* cpu) { return cpu->jit->GetPC(); }
void oboromi_dynarmic_set_pc(Cpu* cpu, std::uint64_t value) { cpu->jit->SetPC(value); }
void oboromi_dynarmic_set_tls(Cpu* cpu, std::uint64_t value) { cpu->callbacks.tls = value; }
std::uint32_t oboromi_dynarmic_take_svc(Cpu* cpu) { return std::exchange(cpu->callbacks.svc, std::numeric_limits<std::uint32_t>::max()); }
void oboromi_dynarmic_clear_exclusive(Cpu* cpu) { cpu->jit->ClearExclusiveState(); cpu->monitor->value.ClearProcessor(cpu->id); }
void oboromi_dynarmic_save(const Cpu* cpu, Context* context) { cpu->save(*context); }
void oboromi_dynarmic_load(Cpu* cpu, const Context* context) { cpu->load(*context); }
void oboromi_dynarmic_invalidate(Cpu* cpu, std::uint64_t address, std::size_t size) { cpu->jit->InvalidateCacheRange(address, size); }
}
