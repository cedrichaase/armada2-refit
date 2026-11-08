/* STA64 M1 presentation spike, the 32-bit side: M1Spike.asi.
 *
 * - makes a named shared mapping (m1.h) and starts the 64-bit host (M1Host.exe) with
 *   its name, on request (the flag files below), timing the start;
 * - counts the engine's frames: it registers with Display8 (display8v1_register), whose
 *   claims callback is asked at every BeginScene on the game's thread;
 * - blocks the game's thread for 3 s at the next BeginScene on F11 or the flag file
 *   M1Stall.flag, to stand for a load's still;
 * - counts the messages the game's thread takes off its queue (a WH_GETMESSAGE hook on
 *   that thread: a window-message hook of this process, no code patched).
 *
 * Flag files in the game directory, polled every 50 ms and deleted when taken:
 *   M1Spawn.flag  its text is the host's arguments; the running host is ended first
 *   M1Kill.flag   end the host
 *   M1Stall.flag  stall the game thread (its text, if any, is the time in ms)
 *   M1Hide.flag / M1Show.flag   ask the host to hide / show its window
 *
 * Throwaway: nothing here ships. */
#include "../m1.h"

typedef void *HANDLE;
typedef void *HWND;
typedef void *HMODULE;
typedef int BOOL;
typedef unsigned long DWORD;
typedef long LONG;
typedef unsigned int UINT;
typedef unsigned int WPARAM;
typedef long LPARAM;
typedef long LRESULT;
typedef struct { LONG left, top, right, bottom; } RECT;
typedef struct { LONG x, y; } POINT;
typedef struct { HWND hwnd; UINT message; WPARAM wParam; LPARAM lParam; DWORD time; POINT pt; } MSG;
typedef struct { long long QuadPart; } LARGE_INTEGER;
typedef struct {
    DWORD cb; char *lpReserved, *lpDesktop, *lpTitle;
    DWORD dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
    unsigned short wShowWindow, cbReserved2; void *lpReserved2; HANDLE hStdInput, hStdOutput, hStdError;
} STARTUPINFOA;
typedef struct { HANDLE hProcess, hThread; DWORD dwProcessId, dwThreadId; } PROCESS_INFORMATION;
typedef LRESULT(__stdcall *HOOKPROC)(int, WPARAM, LPARAM);
typedef BOOL(__stdcall *WNDENUMPROC)(HWND, LPARAM);

#define WINAPI __stdcall
#define IMPORT __declspec(dllimport)
IMPORT HANDLE WINAPI CreateFileMappingA(HANDLE, void *, DWORD, DWORD, DWORD, const char *);
IMPORT void *WINAPI MapViewOfFile(HANDLE, DWORD, DWORD, DWORD, unsigned);
IMPORT BOOL WINAPI CreateProcessA(const char *, char *, void *, void *, BOOL, DWORD, void *, const char *, STARTUPINFOA *, PROCESS_INFORMATION *);
IMPORT BOOL WINAPI TerminateProcess(HANDLE, UINT);
IMPORT DWORD WINAPI WaitForSingleObject(HANDLE, DWORD);
IMPORT HANDLE WINAPI CreateThread(void *, unsigned, DWORD(WINAPI *)(void *), void *, DWORD, DWORD *);
IMPORT void WINAPI Sleep(DWORD);
IMPORT HMODULE WINAPI GetModuleHandleA(const char *);
IMPORT void *WINAPI GetProcAddress(HMODULE, const char *);
IMPORT DWORD WINAPI GetModuleFileNameA(HMODULE, char *, DWORD);
IMPORT BOOL WINAPI QueryPerformanceCounter(LARGE_INTEGER *);
IMPORT BOOL WINAPI QueryPerformanceFrequency(LARGE_INTEGER *);
IMPORT DWORD WINAPI GetCurrentProcessId(void);
IMPORT HANDLE WINAPI CreateFileA(const char *, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
IMPORT BOOL WINAPI WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
IMPORT BOOL WINAPI ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
IMPORT BOOL WINAPI CloseHandle(HANDLE);
IMPORT DWORD WINAPI GetFileAttributesA(const char *);
IMPORT BOOL WINAPI DeleteFileA(const char *);
IMPORT BOOL WINAPI DisableThreadLibraryCalls(HMODULE);
IMPORT BOOL WINAPI EnumWindows(WNDENUMPROC, LPARAM);
IMPORT DWORD WINAPI GetWindowThreadProcessId(HWND, DWORD *);
IMPORT BOOL WINAPI IsWindowVisible(HWND);
IMPORT BOOL WINAPI GetWindowRect(HWND, RECT *);
IMPORT HANDLE WINAPI SetWindowsHookExA(int, HOOKPROC, HMODULE, DWORD);
IMPORT LRESULT WINAPI CallNextHookEx(HANDLE, int, WPARAM, LPARAM);
IMPORT HWND WINAPI GetForegroundWindow(void);
IMPORT HWND WINAPI GetActiveWindow(void);
IMPORT int __cdecl wsprintfA(char *, const char *, ...);

#define INVALID_HANDLE ((HANDLE)(long)-1)
#define PAGE_READWRITE 4
#define FILE_MAP_ALL_ACCESS 0xf001f

void *memset(void *d, int c, unsigned n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
int _fltused;

static HMODULE g_self;
static volatile M1Header *H;
static char g_dir[260], g_logpath[300], g_name[64];
static HANDLE g_log, g_hook, g_host;
static DWORD g_hook_tid;

static u64 qpc(void) { LARGE_INTEGER t; QueryPerformanceCounter(&t); return (u64)t.QuadPart; }
static u32 us_since(u64 t0, u64 t1) { return (u32)((double)(long long)(t1 - t0) * 1e6 / (double)(long long)H->qpf); }

static void logs(const char *s)
{
    DWORD n = 0, w;
    while (s[n]) n++;
    if (g_log) WriteFile(g_log, s, n, &w, 0);
}
#define LOGF(...) do { char b_[512]; int n_ = wsprintfA(b_, __VA_ARGS__); b_[n_] = 0; logs(b_); } while (0)

static void path(char *out, const char *file) { wsprintfA(out, "%s\\%s", g_dir, file); }
static BOOL take_flag(const char *file, char *text, int cap)
{
    char p[300];
    path(p, file);
    if (GetFileAttributesA(p) == 0xffffffff) return 0;
    if (text) {
        DWORD r = 0;
        HANDLE f = CreateFileA(p, 0x80000000, 7, 0, 3, 0, 0);
        text[0] = 0;
        if (f != INVALID_HANDLE) { ReadFile(f, text, cap - 1, &r, 0); CloseHandle(f); }
        text[r] = 0;
        while (r && (text[r - 1] == '\n' || text[r - 1] == '\r' || text[r - 1] == ' ')) text[--r] = 0;
    }
    DeleteFileA(p);
    return 1;
}

/* The engine's frame: Display8 asks this at every BeginScene, on the game's thread. */
static u32 claims_cb(void)
{
    u64 now = qpc();
    u64 prev = H->heartbeat_qpc;
    H->counter++;
    H->heartbeat_qpc = now;
    H->game_focus_hwnd = (u32)(unsigned long)GetForegroundWindow();
    H->game_active = (u32)(unsigned long)GetActiveWindow();
    if (prev && us_since(prev, now) > 100000)
        LOGF("frame gap %u us before BeginScene %u\r\n", us_since(prev, now), H->counter);
    if (H->stall_ms) {
        u32 ms = H->stall_ms;
        H->stall_ms = 0;
        H->stalled = 1;
        H->stall_begin_qpc = qpc();
        LOGF("stall: game thread sleeps %u ms at BeginScene %u\r\n", ms, H->counter);
        Sleep(ms);
        H->stall_end_qpc = qpc();
        H->stalled = 0;
        H->stall_count++;
        LOGF("stall: done after %u us; host presented up to frame %u\r\n",
             us_since(H->stall_begin_qpc, H->stall_end_qpc), H->host_frames);
    }
    return 0;
}

typedef struct { u32 size; void *caller; u32 (*claims)(void); } Plugin8;

static LRESULT WINAPI getmsg(int code, WPARAM wp, LPARAM lp)
{
    if (code >= 0 && wp == 1 /* PM_REMOVE */) {
        MSG *m = (MSG *)lp;
        int k;
        switch (m->message) {
        case 0x200: k = MK_MOVE; break;
        case 0x201: k = MK_LDOWN; break;
        case 0x202: k = MK_LUP; break;
        case 0x204: k = MK_RDOWN; break;
        case 0x100: case 0x104: k = MK_KEYDOWN; break;
        case 0x102: k = MK_CHAR; break;
        case 0x06: case 0x1c: case 0x07: case 0x08: k = MK_ACTIVATE; break;
        default: k = MK_OTHER;
        }
        H->game_msgs[k]++;
        if (k == MK_LDOWN || k == MK_RDOWN || k == MK_KEYDOWN)
            LOGF("game msg 0x%x hwnd %x wp %x lp %x\r\n", m->message, (u32)(unsigned long)m->hwnd, m->wParam, (u32)m->lParam);
        if (k == MK_ACTIVATE)
            LOGF("game msg 0x%x (activation) hwnd %x wp %x\r\n", m->message, (u32)(unsigned long)m->hwnd, m->wParam);
        if (m->message == 0x100 && m->wParam == 0x7a /* F11 */) H->stall_ms = 3000;
    }
    return CallNextHookEx(g_hook, code, wp, lp);
}

static HWND g_best; static long g_best_area;
static BOOL WINAPI enum_cb(HWND w, LPARAM lp)
{
    DWORD pid = 0;
    RECT r;
    (void)lp;
    GetWindowThreadProcessId(w, &pid);
    if (pid != GetCurrentProcessId() || !IsWindowVisible(w) || !GetWindowRect(w, &r)) return 1;
    if ((r.right - r.left) * (r.bottom - r.top) > g_best_area) {
        g_best_area = (r.right - r.left) * (r.bottom - r.top);
        g_best = w;
    }
    return 1;
}

static void kill_host(void)
{
    if (!g_host) return;
    TerminateProcess(g_host, 0);
    WaitForSingleObject(g_host, 2000);
    CloseHandle(g_host);
    g_host = 0;
    LOGF("host ended\r\n");
}

static void spawn_host(const char *args)
{
    char cmd[600], exe[300];
    STARTUPINFOA si;
    PROCESS_INFORMATION pi;
    kill_host();
    path(exe, "M1Host.exe");
    wsprintfA(cmd, "\"%s\" %s %s", exe, g_name, args);
    memset(&si, 0, sizeof si);
    si.cb = sizeof si;
    H->host_open_qpc = H->host_window_qpc = H->host_first_present_qpc = 0;
    H->host_frames = 0;
    H->host_hide = 0;
    H->spawn_qpc = qpc();
    if (!CreateProcessA(exe, cmd, 0, 0, 0, 0, 0, g_dir, &si, &pi)) {
        LOGF("CreateProcess failed: %s\r\n", cmd);
        return;
    }
    H->spawned_qpc = qpc();
    g_host = pi.hProcess;
    CloseHandle(pi.hThread);
    LOGF("host started: pid %u, CreateProcess took %u us: %s\r\n", pi.dwProcessId,
         us_since(H->spawn_qpc, H->spawned_qpc), cmd);
}

static DWORD WINAPI watcher(void *unused)
{
    char text[400];
    u32 registered = 0, reported = 0, ticks = 0, last_counter = 0;
    (void)unused;
    for (;; Sleep(50), ticks++) {
        if (!registered) {
            HMODULE d = GetModuleHandleA("d3d8.dll");
            u32 (*reg)(const Plugin8 *) = d ? (u32 (*)(const Plugin8 *))GetProcAddress(d, "display8v1_register") : 0;
            if (reg) {
                Plugin8 p = { sizeof(Plugin8), (void *)claims_cb, claims_cb };
                registered = reg(&p);
                LOGF("registered with Display8: id %u\r\n", registered);
            }
        }
        g_best = 0; g_best_area = 0;
        EnumWindows(enum_cb, 0);
        if (g_best && (u32)(unsigned long)g_best != H->game_hwnd) {
            DWORD tid = GetWindowThreadProcessId(g_best, 0);
            RECT r;
            GetWindowRect(g_best, &r);
            H->game_hwnd = (u32)(unsigned long)g_best;
            H->game_tid = tid;
            LOGF("game window %x (thread %u) %d,%d %dx%d\r\n", H->game_hwnd, tid, r.left, r.top, r.right - r.left, r.bottom - r.top);
            if (tid != g_hook_tid) {
                g_hook = SetWindowsHookExA(3 /* WH_GETMESSAGE */, getmsg, g_self, tid);
                g_hook_tid = tid;
                LOGF("WH_GETMESSAGE hook on thread %u: %s\r\n", tid, g_hook ? "ok" : "failed");
            }
        }
        if (take_flag("M1Spawn.flag", text, sizeof text)) spawn_host(text);
        if (take_flag("M1Kill.flag", 0, 0)) kill_host();
        if (take_flag("M1Hide.flag", 0, 0)) H->host_hide = 1;
        if (take_flag("M1Show.flag", 0, 0)) H->host_hide = 0;
        if (take_flag("M1Stall.flag", text, sizeof text)) {
            u32 ms = 0; char *c = text;
            while (*c >= '0' && *c <= '9') ms = ms * 10 + (u32)(*c++ - '0');
            H->stall_ms = ms ? ms : 3000;
            LOGF("stall requested: %u ms\r\n", H->stall_ms);
        }
        if (H->host_first_present_qpc && !reported) {
            reported = 1;
            LOGF("host start: CreateProcess %u us, mapping open at %u us, window at %u us, first present at %u us\r\n",
                 us_since(H->spawn_qpc, H->spawned_qpc), us_since(H->spawn_qpc, H->host_open_qpc),
                 us_since(H->spawn_qpc, H->host_window_qpc), us_since(H->spawn_qpc, H->host_first_present_qpc));
        }
        if (!H->host_first_present_qpc) reported = 0;
        if (ticks % 20 == 0) {
            LOGF("t %u: frames %u (+%u), host frames %u mode %c hwnd %x; fg %x active %x; game msgs mv %u ld %u lu %u rd %u kd %u ch %u act %u oth %u; host msgs mv %u ld %u lu %u rd %u kd %u ch %u act %u oth %u\r\n",
                 ticks / 20, H->counter, H->counter - last_counter, H->host_frames, H->host_mode ? H->host_mode : '-', H->host_hwnd,
                 H->game_focus_hwnd, H->game_active,
                 H->game_msgs[0], H->game_msgs[1], H->game_msgs[2], H->game_msgs[3], H->game_msgs[4], H->game_msgs[5], H->game_msgs[6], H->game_msgs[7],
                 H->host_msgs[0], H->host_msgs[1], H->host_msgs[2], H->host_msgs[3], H->host_msgs[4], H->host_msgs[5], H->host_msgs[6], H->host_msgs[7]);
            last_counter = H->counter;
        }
    }
    return 0;
}

BOOL WINAPI DllMain(HMODULE self, DWORD reason, void *reserved)
{
    (void)reserved;
    if (reason != 1) return 1;
    g_self = self;
    DisableThreadLibraryCalls(self);
    {
        int i, last = 0;
        GetModuleFileNameA(self, g_dir, sizeof g_dir);
        for (i = 0; g_dir[i]; i++) if (g_dir[i] == '\\') last = i;
        g_dir[last] = 0;
    }
    path(g_logpath, "M1Spike.log");
    g_log = CreateFileA(g_logpath, 0x40000000, 1, 0, 2 /* CREATE_ALWAYS */, 0, 0);
    if (g_log == INVALID_HANDLE) g_log = 0;
    wsprintfA(g_name, "Local\\A2M1Spike-%u", GetCurrentProcessId());
    {
        HANDLE m = CreateFileMappingA(INVALID_HANDLE, 0, PAGE_READWRITE, 0, 65536, g_name);
        LARGE_INTEGER f;
        if (!m) { logs("CreateFileMapping failed\r\n"); return 1; }
        H = (volatile M1Header *)MapViewOfFile(m, FILE_MAP_ALL_ACCESS, 0, 0, 0);
        if (!H) { logs("MapViewOfFile failed\r\n"); return 1; }
        QueryPerformanceFrequency(&f);
        H->qpf = (u64)f.QuadPart;
        H->magic = M1_MAGIC; H->version = M1_VERSION; H->size = sizeof(M1Header);
        H->game_pid = GetCurrentProcessId();
    }
    LOGF("M1Spike: mapping %s (%u bytes header), qpf %u\r\n", g_name, (u32)sizeof(M1Header), (u32)H->qpf);
    CreateThread(0, 0, watcher, 0, 0, 0);
    return 1;
}
