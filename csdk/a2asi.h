/*
 * csdk/a2asi.h -- what every C plugin of this repository shares.  Header-only, no CRT.
 *
 * Used by qol/qol.c, qol/rules.c, online/online.c (and peer.c, which it includes) and
 * cutscenes/binkproxy/binkproxy.c; built by each layer's build.sh with the import
 * libraries of csdk/def/ (one set of .def files, the union of what the plugins call:
 * an import library only costs a thunk for what a plugin really references).
 *
 *   - Win32 types, constants and the kernel32 calls all four use;
 *   - string helpers s_len / s_cpy / s_cat / s_num / s_hex / s_hexw / s_eq / s_copyn
 *     writing into a caller's buffer, and f_round (the FPU's own, since a C cast
 *     would call the CRT's __ftol2);
 *   - the log: a2_append_line() opens the file, appends one line and closes it (a
 *     plugin that keeps its log open has its own logline() over the s_* helpers),
 *     and a2_exe_dir() gives the directory of Armada2.exe, where every ini sits;
 *     a2_log_path() puts a log in %LOCALAPPDATA%\armada2-refit\logs (the game directory
 *     if that is unset or not writable) and a2_log_where() writes the first line, which
 *     says where;
 *   - patching: a2_sig_is() checks a site's bytes against this build before a plugin
 *     patches, a2_poke() writes through VirtualProtect and flushes the instruction
 *     cache, a2_write_call() / a2_write_jmp() write a rel32 call or jmp, NOP-padded.
 *
 * Every function is `static inline`, so one that a plugin does not use costs nothing and
 * -Wunused-function stays quiet.  A plugin that uses floating point defines A2_FLTUSED
 * before the include (the MSVC ABI wants the `_fltused` marker the CRT would give).
 * Anything that is one plugin's own (its log's format, its tables) stays in its file.
 */
#ifndef A2ASI_H
#define A2ASI_H

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned short      WCHAR;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef long                HRESULT;
typedef unsigned long       ULONG;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

#define WINAPI   __stdcall
#define NULLPTR ((void *)0)
#define TRUE  1
#define FALSE 0

#ifdef A2_FLTUSED
/* The MSVC ABI references this marker from any object that uses floating point; the
 * CRT would define it, and there is no CRT here. */
int _fltused = 0;
#endif

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define CREATE_ALWAYS          2
#define OPEN_ALWAYS            4
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_READWRITE         0x04
#define PAGE_EXECUTE_READWRITE 0x40

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) void *  __stdcall GetProcAddress(HMODULE, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetEnvironmentVariableA(LPCSTR, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall CreateDirectoryA(LPCSTR, void *);

/* ---- strings: the caller's buffer, no CRT ------------------------------ */

static inline int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }
static inline void s_cpy(char *d, const char *s) { while ((*d++ = *s++)) {} }

static inline void s_cat(char *d, const char *s)
{
    int n = s_len(d);
    while (*s) d[n++] = *s++;
    d[n] = 0;
}

static inline void s_num(char *d, long v)
{
    char t[16];
    int  n = 0, neg = 0, k = s_len(d);
    if (v < 0) { neg = 1; v = -v; }
    if (!v) t[n++] = '0';
    while (v) { t[n++] = (char)('0' + (v % 10)); v /= 10; }
    if (neg) d[k++] = '-';
    while (n) d[k++] = t[--n];
    d[k] = 0;
}

/* `digits` lower-case hex digits of v, appended */
static inline void s_hexw(char *d, DWORD v, int digits)
{
    static const char x[] = "0123456789abcdef";
    int i, k = s_len(d);
    for (i = (digits - 1) * 4; i >= 0; i -= 4) d[k++] = x[(v >> i) & 15];
    d[k] = 0;
}

/* "0x" and eight digits, appended */
static inline void s_hex(char *d, DWORD v) { s_cat(d, "0x"); s_hexw(d, v, 8); }

static inline int s_eq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

/* at most max-1 characters of s (s may be NULL), always terminated */
static inline void s_copyn(char *d, const char *s, int max)
{
    int i = 0;
    if (s) for (; s[i] && i < max - 1; i++) d[i] = s[i];
    d[i] = 0;
}

/* Round to long with the FPU itself: a C cast would call the CRT's __ftol2. */
static inline long f_round(float f)
{
    long r;
    __asm__ __volatile__("flds %1\n\tfistpl %0" : "=m"(r) : "m"(f));
    return r;
}

/* ---- the files beside Armada2.exe ---------------------------------------- */

/* The directory of the exe, with its trailing slash, in path (cap bytes at most).
 * Returns 0, with path empty, when the name cannot be read. */
static inline int a2_exe_dir(char *path, int cap)
{
    int n = (int)GetModuleFileNameA(NULLPTR, path, (DWORD)cap);
    if (n <= 0) { path[0] = 0; return 0; }
    while (n > 0 && path[n - 1] != '\\' && path[n - 1] != '/') n--;
    path[n] = 0;
    return 1;
}

/* One line, then CR LF, appended to the file at path (created on the first line);
 * the file is closed again, so another process may read it between lines. */
static inline void a2_append_line(const char *path, const char *s)
{
    HANDLE h;
    DWORD  wrote;

    h = CreateFileA(path, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    WriteFile(h, s, (DWORD)s_len(s), &wrote, NULLPTR);
    WriteFile(h, "\r\n", 2, &wrote, NULLPTR);
    CloseHandle(h);
}

/* Where the log called name goes: %LOCALAPPDATA%\armada2-refit\logs\name, the folders
 * made if missing, else beside Armada2.exe (dir, with its trailing slash, from
 * a2_exe_dir).  The folder is checked by opening the file for append, which does not
 * truncate it.  Returns 1 for the first place, 0 for the fallback. */
static inline int a2_log_path(char *out, int cap, const char *dir, const char *name)
{
    char    d[300];
    HANDLE  h;
    DWORD   n = GetEnvironmentVariableA("LOCALAPPDATA", d, sizeof d);

    if (n > 0 && n < sizeof d - 40) {
        s_cat(d, "\\armada2-refit");
        CreateDirectoryA(d, NULLPTR);
        s_cat(d, "\\logs");
        CreateDirectoryA(d, NULLPTR);
        s_cat(d, "\\");
        s_cat(d, name);
        if (s_len(d) < cap) {
            h = CreateFileA(d, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                            OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
            if (h != INVALID_HANDLE_VALUE) {
                CloseHandle(h);
                s_copyn(out, d, cap);
                return 1;
            }
        }
    }
    out[0] = 0;
    s_copyn(out, dir, cap);
    if (s_len(out) + s_len(name) < cap) s_cat(out, name);
    return 0;
}

/* The first line of a log: where it is. */
static inline void a2_log_where(const char *path, int appdata)
{
    char b[400];
    b[0] = 0;
    s_cat(b, "log: ");
    s_cat(b, path);
    if (!appdata) s_cat(b, " (LOCALAPPDATA unset or not writable)");
    a2_append_line(path, b);
}

/* ---- patching the exe in memory ------------------------------------------ */

/* Do the n bytes at addr match sig?  Every site is checked against this build of
 * Armada2.exe before it is patched; a different exe leaves that change out. */
static inline int a2_sig_is(DWORD addr, const BYTE *sig, int n)
{
    int k;
    for (k = 0; k < n; k++)
        if (((const BYTE *)addr)[k] != sig[k]) return 0;
    return 1;
}

/* Write n bytes at addr through VirtualProtect and flush the instruction cache.
 * 0 when the page could not be opened. */
static inline int a2_poke(DWORD addr, const BYTE *bytes, int n)
{
    DWORD old;
    int   k;
    if (!VirtualProtect((void *)addr, (UINT)n, PAGE_EXECUTE_READWRITE, &old)) return 0;
    for (k = 0; k < n; k++) ((BYTE *)addr)[k] = bytes[k];
    VirtualProtect((void *)addr, (UINT)n, old, &old);
    FlushInstructionCache(GetCurrentProcess(), (void *)addr, (UINT)n);
    return 1;
}

/* A rel32 call (0xe8) or jmp (0xe9) to `to` at `at`, NOP-padded to n >= 5 bytes (n <= 16). */
static inline int a2_write_rel(BYTE op, DWORD at, const void *to, int n)
{
    BYTE  b[16];
    DWORD rel = (DWORD)to - (at + 5);
    int   i;
    b[0] = op;
    for (i = 0; i < 4; i++) b[1 + i] = (BYTE)(rel >> (8 * i));
    for (i = 5; i < n; i++) b[i] = 0x90;
    return a2_poke(at, b, n);
}
static inline int a2_write_call(DWORD at, const void *to, int n) { return a2_write_rel(0xe8, at, to, n); }
static inline int a2_write_jmp(DWORD at, const void *to, int n)  { return a2_write_rel(0xe9, at, to, n); }

#endif /* A2ASI_H */
