/*
 * Stems.asi -- plays the game's music and voice on DirectSound devices of their
 * own (test bench only), so a take records them as separate audio tracks.
 *
 * Wine's DirectSound mixes every buffer of one device into one output stream, so
 * the bench could only ever record the whole mix. This plugin opens one more
 * DirectSound device per category named in Stems.ini, each on an output the bench
 * made for it (a null sink of the session's), and moves every 2D sound of that
 * category onto it as the sound starts. What stays on the game's own device --
 * the 3D sound effects, and anything not named -- is the effects stem.
 * testbench/README.md, "Audio stems", has the why and the bench's side.
 *
 * WHAT IT USES OF Armada2.exe (GOG patch 1.1, addresses from armada2.map)
 * ----------------------------------------------------------------------
 *   Every AudioSound carries its volume category (eAudioVolume) as the int at
 *     +0xc: AudioSound::mSetVolume (0x45fa60) reads it there to pick the
 *     player's volume setting. 1 is music (AudioMusic2D, the jukebox), 2 voice
 *     (AudioQueue's unit speech, mission narration through AudioOverlay::Play),
 *     3 the 3D effects, 4 the interface (the button bar). Its name is the string
 *     at +4. The same order is the four volumes in ARMADA.PRF's third line.
 *   AudioSound2D::mInit(IDirectSoundBuffer *) (0x45fb90, thiscall) is where a 2D
 *     sound is handed its buffer, loaded whole and synchronously by
 *     SoundBuffer::RequestBuffer: it stores it at +0x10, sets the volume and
 *     plays it. Its one caller, AudioSound2D::sSoundBufferCallback, calls it at
 *     0x45fb7d; that call is pointed at stems_init(), which, for a category with a
 *     device of its own, copies the buffer (format, flags, every byte) onto that
 *     device, releases the game's copy, and passes the new one on. The sound owns
 *     the buffer it is given and releases it when it ends, as before.
 *   AudioDefault::s_sampleRate and s_bitsPerSample (0x6f5a40, 0x6f5a44) are the
 *     game's own primary-buffer format; each new device's primary is set to it.
 *
 * 3D buffers (DSBCAPS_CTRL3D) are never moved: their listener lives on the game's
 * device. Every site is checked against its bytes first, and a category whose
 * device cannot be found or opened stays on the game's: Stems.log says so.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef int                 INT;
typedef unsigned int        UINT;
typedef long                LONG;
typedef long                HRESULT;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef HANDLE              HWND;
typedef const char         *LPCSTR;
typedef char               *LPSTR;
typedef struct { DWORD d1; WORD d2, d3; BYTE d4[8]; } GUID;

#define NULLPTR ((void *)0)
#define TRUE  1

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
#define CREATE_ALWAYS          2
#define FILE_ATTRIBUTE_NORMAL  0x80
#define FILE_END               2
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_EXECUTE_READWRITE 0x40

__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) BOOL    __stdcall FlushInstructionCache(HANDLE, const void *, UINT);
__declspec(dllimport) HANDLE  __stdcall GetCurrentProcess(void);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   __stdcall SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) HMODULE __stdcall LoadLibraryA(LPCSTR);
__declspec(dllimport) void *  __stdcall GetProcAddress(HMODULE, LPCSTR);

int _fltused = 0;

/* clang may emit these for struct copies even freestanding */
void *memset(void *d, int c, UINT n) { BYTE *p = d; while (n--) *p++ = (BYTE)c; return d; }
void *memcpy(void *d, const void *s, UINT n)
{
    BYTE *p = d; const BYTE *q = s;
    while (n--) *p++ = *q++;
    return d;
}

/* ---- the engine --------------------------------------------------------- */

#define INIT_CALL      0x45fb7du   /* call AudioSound2D::mInit, in sSoundBufferCallback */
#define MINIT          0x45fb90u   /* AudioSound2D::mInit */
#define SETVOL_CAT     0x45fa74u   /* mov 0xc(%esi),%eax in AudioSound::mSetVolume */
#define SAMPLE_RATE    0x6f5a40u   /* AudioDefault::s_sampleRate */
#define BITS           0x6f5a44u   /* AudioDefault::s_bitsPerSample */
#define SND_NAME       0x4
#define SND_CATEGORY   0xc

static const struct { DWORD at; int len; BYTE sig[10]; } k_sigs[] = {
    { INIT_CALL,  5,  { 0xE8, 0x0E, 0x00, 0x00, 0x00 } },
    { MINIT,      10, { 0x55, 0x8B, 0xEC, 0x6A, 0xFF, 0x68, 0x78, 0xAD, 0x69, 0x00 } },
    { SETVOL_CAT, 3,  { 0x8B, 0x46, 0x0C } },
};

/* ---- DirectSound, declared by hand (vtable slots of IDirectSound / Buffer) */

typedef struct { WORD wFormatTag, nChannels; DWORD nSamplesPerSec, nAvgBytesPerSec;
                 WORD nBlockAlign, wBitsPerSample, cbSize; } WAVEFORMATEX;
typedef struct { DWORD dwSize, dwFlags, dwBufferBytes, dwReserved; WAVEFORMATEX *lpwfxFormat;
                 GUID guid3DAlgorithm; } DSBUFFERDESC;
typedef struct { DWORD dwSize, dwFlags, dwBufferBytes, dwUnlockTransferRate, dwPlayCpuOverhead; } DSBCAPS;

typedef struct { void **vt; } COM;
#define VT(o, i) ((o)->vt[i])

typedef HRESULT (__stdcall *ReleaseFn)(COM *);
typedef HRESULT (__stdcall *CreateBufFn)(COM *, const DSBUFFERDESC *, COM **, void *);
typedef HRESULT (__stdcall *CoopFn)(COM *, HWND, DWORD);
typedef HRESULT (__stdcall *GetCapsFn)(COM *, DSBCAPS *);
typedef HRESULT (__stdcall *GetFormatFn)(COM *, WAVEFORMATEX *, DWORD, DWORD *);
typedef HRESULT (__stdcall *GetFreqFn)(COM *, DWORD *);
typedef HRESULT (__stdcall *SetFreqFn)(COM *, DWORD);
typedef HRESULT (__stdcall *LockFn)(COM *, DWORD, DWORD, void **, DWORD *, void **, DWORD *, DWORD);
typedef HRESULT (__stdcall *UnlockFn)(COM *, void *, DWORD, void *, DWORD);
typedef HRESULT (__stdcall *SetFormatFn)(COM *, const WAVEFORMATEX *);
typedef HRESULT (__stdcall *DSCreateFn)(const GUID *, COM **, void *);
typedef BOOL    (__stdcall *EnumCb)(GUID *, LPCSTR, LPCSTR, void *);
typedef HRESULT (__stdcall *DSEnumFn)(EnumCb, void *);
typedef HWND    (__stdcall *DesktopFn)(void);

enum { DS_RELEASE = 2, DS_CREATEBUF = 3, DS_COOP = 6 };
enum { B_RELEASE = 2, B_GETCAPS = 3, B_GETFORMAT = 5, B_GETFREQ = 8, B_LOCK = 11,
       B_SETFORMAT = 14, B_SETFREQ = 17, B_UNLOCK = 19 };

#define DSBCAPS_PRIMARYBUFFER  0x00000001
#define DSBCAPS_STATIC         0x00000002
#define DSBCAPS_CTRL3D         0x00000010
#define DSBCAPS_CTRLFREQUENCY  0x00000020
#define DSBCAPS_CTRLPAN        0x00000040
#define DSBCAPS_CTRLVOLUME     0x00000080
#define DSBCAPS_CTRLPOSNOTIFY  0x00000100
#define DSBCAPS_GLOBALFOCUS    0x00008000
#define DSBCAPS_GETCURPOS2     0x00010000
#define DSBLOCK_ENTIREBUFFER   0x00000002
#define DSSCL_PRIORITY         2

/* ---- small string helpers, log ----------------------------------------- */

static char g_ini[320], g_logpath[320];

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }
static void s_cat(char *d, const char *s) { d += s_len(d); while ((*d++ = *s++)) ; }
static void s_num(char *d, long v)
{
    char t[16]; int i = 15;
    int neg = v < 0;
    if (neg) v = -v;
    t[i] = 0;
    do { t[--i] = (char)('0' + v % 10); v /= 10; } while (v && i > 1);
    if (neg) t[--i] = '-';
    s_cat(d, t + i);
}
static void s_hex(char *d, DWORD v)
{
    char t[11]; int i;
    t[0] = '0'; t[1] = 'x';
    for (i = 0; i < 8; i++) t[2 + i] = "0123456789abcdef"[(v >> (28 - 4 * i)) & 15];
    t[10] = 0;
    s_cat(d, t);
}
static int s_has(const char *hay, const char *needle)
{
    int n = s_len(needle), i, k;
    if (!n) return 0;
    for (i = 0; hay[i]; i++) {
        for (k = 0; k < n && hay[i + k] && hay[i + k] == needle[k]; k++) ;
        if (k == n) return 1;
    }
    return 0;
}

static void logline(const char *s)
{
    HANDLE h;
    DWORD  wrote;
    if (!g_logpath[0]) return;
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR, OPEN_ALWAYS,
                    FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    WriteFile(h, s, (DWORD)s_len(s), &wrote, NULLPTR);
    WriteFile(h, "\r\n", 2, &wrote, NULLPTR);
    CloseHandle(h);
}

/* ---- the devices -------------------------------------------------------- */

#define NCAT 5
static const char *const k_names[NCAT] = { "Other", "Music", "Voice", "Effects", "Interface" };

static struct {
    char  sink[128];      /* the output's name, as Stems.ini gives it; empty: not moved */
    int   state;          /* 0 not tried, 1 open, -1 failed */
    COM  *ds;
    long  moved, bytes, failed;
} g_cat[NCAT];

static DSCreateFn g_create;
static DSEnumFn   g_enum;
static HWND       g_hwnd;

struct find { const char *sink; GUID guid; int found; char desc[128]; };

static BOOL __stdcall enum_cb(GUID *guid, LPCSTR desc, LPCSTR module, void *ctx)
{
    struct find *f = ctx;
    (void)module;
    if (guid && desc && s_has(desc, f->sink)) {
        f->guid = *guid;
        f->found = 1;
        f->desc[0] = 0;
        if (s_len(desc) < 120) s_cat(f->desc, desc);
        return 0;
    }
    return 1;
}

static int open_device(int cat)
{
    struct find f;
    char b[300];
    COM *prim = NULLPTR;
    DSBUFFERDESC d;
    WAVEFORMATEX w;
    HRESULT hr;

    if (g_cat[cat].state) return g_cat[cat].state > 0;
    g_cat[cat].state = -1;
    if (!g_create) {
        HMODULE m = LoadLibraryA("dsound.dll"), u = LoadLibraryA("user32.dll");
        g_create = m ? (DSCreateFn)GetProcAddress(m, "DirectSoundCreate") : NULLPTR;
        g_enum = m ? (DSEnumFn)GetProcAddress(m, "DirectSoundEnumerateA") : NULLPTR;
        if (u) {
            DesktopFn df = (DesktopFn)GetProcAddress(u, "GetDesktopWindow");
            g_hwnd = df ? df() : NULLPTR;
        }
        if (!g_create || !g_enum) { logline("NOT MOVED: no DirectSoundCreate in dsound.dll"); return 0; }
    }
    memset(&f, 0, sizeof f);
    f.sink = g_cat[cat].sink;
    g_enum(enum_cb, &f);
    b[0] = 0;
    if (!f.found) {
        s_cat(b, k_names[cat]); s_cat(b, ": no output named "); s_cat(b, g_cat[cat].sink);
        s_cat(b, " among DirectSound's devices; it stays in the game's mix");
        logline(b);
        return 0;
    }
    hr = g_create(&f.guid, &g_cat[cat].ds, NULLPTR);
    if (hr < 0 || !g_cat[cat].ds) {
        s_cat(b, k_names[cat]); s_cat(b, ": DirectSoundCreate failed "); s_hex(b, (DWORD)hr);
        logline(b);
        return 0;
    }
    ((CoopFn)VT(g_cat[cat].ds, DS_COOP))(g_cat[cat].ds, g_hwnd, DSSCL_PRIORITY);
    /* the primary buffer in the game's own format, as AudioManager::Open sets its own */
    memset(&d, 0, sizeof d);
    d.dwSize = sizeof d;
    d.dwFlags = DSBCAPS_PRIMARYBUFFER;
    if (((CreateBufFn)VT(g_cat[cat].ds, DS_CREATEBUF))(g_cat[cat].ds, &d, &prim, NULLPTR) >= 0 && prim) {
        memset(&w, 0, sizeof w);
        w.wFormatTag = 1;
        w.nChannels = 2;
        w.nSamplesPerSec = (DWORD)*(const LONG *)SAMPLE_RATE;
        w.wBitsPerSample = (WORD)*(const LONG *)BITS;
        if (w.nSamplesPerSec < 8000 || w.nSamplesPerSec > 96000) w.nSamplesPerSec = 22050;
        if (w.wBitsPerSample != 8 && w.wBitsPerSample != 16) w.wBitsPerSample = 16;
        w.nBlockAlign = (WORD)(w.nChannels * w.wBitsPerSample / 8);
        w.nAvgBytesPerSec = w.nSamplesPerSec * w.nBlockAlign;
        ((SetFormatFn)VT(prim, B_SETFORMAT))(prim, &w);
        /* the primary is kept: releasing it lets the device fall back to its default */
    }
    g_cat[cat].state = 1;
    s_cat(b, k_names[cat]); s_cat(b, ": its own device, "); s_cat(b, f.desc);
    logline(b);
    return 1;
}

/* The game's buffer, copied onto the category's device: NULL if it cannot be. */
static COM *copy_buffer(int cat, COM *src)
{
    DSBCAPS caps;
    BYTE fmt[64];
    DWORD got = 0, freq = 0, n1, n2, m1, m2;
    void *p1, *p2, *q1, *q2;
    DSBUFFERDESC d;
    COM *dst = NULLPTR;
    COM *ds = g_cat[cat].ds;

    memset(&caps, 0, sizeof caps);
    caps.dwSize = sizeof caps;
    if (((GetCapsFn)VT(src, B_GETCAPS))(src, &caps) < 0) return NULLPTR;
    if (caps.dwFlags & (DSBCAPS_PRIMARYBUFFER | DSBCAPS_CTRL3D)) return NULLPTR;
    if (((GetFormatFn)VT(src, B_GETFORMAT))(src, (WAVEFORMATEX *)fmt, sizeof fmt, &got) < 0) return NULLPTR;
    memset(&d, 0, sizeof d);
    d.dwSize = sizeof d;
    d.dwFlags = (caps.dwFlags & (DSBCAPS_STATIC | DSBCAPS_CTRLPOSNOTIFY | DSBCAPS_GETCURPOS2))
              | DSBCAPS_CTRLFREQUENCY | DSBCAPS_CTRLPAN | DSBCAPS_CTRLVOLUME | DSBCAPS_GLOBALFOCUS;
    d.dwBufferBytes = caps.dwBufferBytes;
    d.lpwfxFormat = (WAVEFORMATEX *)fmt;
    if (((CreateBufFn)VT(ds, DS_CREATEBUF))(ds, &d, &dst, NULLPTR) < 0 || !dst) return NULLPTR;
    if (((LockFn)VT(src, B_LOCK))(src, 0, 0, &p1, &n1, &p2, &n2, DSBLOCK_ENTIREBUFFER) < 0) {
        ((ReleaseFn)VT(dst, B_RELEASE))(dst);
        return NULLPTR;
    }
    if (((LockFn)VT(dst, B_LOCK))(dst, 0, 0, &q1, &m1, &q2, &m2, DSBLOCK_ENTIREBUFFER) < 0) {
        ((UnlockFn)VT(src, B_UNLOCK))(src, p1, n1, p2, n2);
        ((ReleaseFn)VT(dst, B_RELEASE))(dst);
        return NULLPTR;
    }
    memcpy(q1, p1, n1 < m1 ? n1 : m1);
    ((UnlockFn)VT(dst, B_UNLOCK))(dst, q1, m1, q2, m2);
    ((UnlockFn)VT(src, B_UNLOCK))(src, p1, n1, p2, n2);
    if (((GetFreqFn)VT(src, B_GETFREQ))(src, &freq) >= 0 && freq)
        ((SetFreqFn)VT(dst, B_SETFREQ))(dst, freq);
    g_cat[cat].bytes += (long)caps.dwBufferBytes;
    return dst;
}

typedef void (__fastcall *MInitFn)(void *self, void *edx, COM *buf);

/* thiscall with one stack argument, as AudioSound2D::mInit: fastcall's ecx, with edx unused */
static void __fastcall stems_init(void *self, void *edx, COM *buf)
{
    int cat = self ? *(const int *)((const BYTE *)self + SND_CATEGORY) : -1;
    if (buf && cat > 0 && cat < NCAT && g_cat[cat].sink[0] && open_device(cat)) {
        COM *mine = copy_buffer(cat, buf);
        if (mine) {
            ((ReleaseFn)VT(buf, B_RELEASE))(buf);
            buf = mine;
            if (g_cat[cat].moved++ < 40) {     /* the first few, by name: what each stem holds */
                char b[300];
                const char *name = *(const char *const *)((const BYTE *)self + SND_NAME);
                b[0] = 0;
                s_cat(b, k_names[cat]); s_cat(b, ": ");
                if (name && s_len(name) < 200) s_cat(b, name);
                s_cat(b, ", "); s_num(b, g_cat[cat].bytes); s_cat(b, " bytes so far");
                logline(b);
            }
        } else if (g_cat[cat].failed++ < 10) {
            char b[120];
            b[0] = 0;
            s_cat(b, k_names[cat]); s_cat(b, ": a buffer could not be copied; it plays in the game's mix");
            logline(b);
        }
    }
    ((MInitFn)MINIT)(self, edx, buf);
}

/* ---- startup ------------------------------------------------------------ */

static void build_paths(void)
{
    char path[300];
    int n, i, cut = 0;
    n = (int)GetModuleFileNameA(NULLPTR, path, 280);
    if (n <= 0) return;
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;
    g_ini[0] = 0;     s_cat(g_ini, path);     s_cat(g_ini, "Stems.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "Stems.log");
}

static void startup(void)
{
    char b[300];
    int i, k, any = 0;
    DWORD old;
    BYTE *p;

    build_paths();
    logline("--- Stems");
    for (i = 1; i < NCAT; i++) {
        GetPrivateProfileStringA("Stems", k_names[i], "", g_cat[i].sink, sizeof g_cat[i].sink, g_ini);
        if (i == 3) g_cat[i].sink[0] = 0;   /* the effects are 3D: they stay, and are the rest */
        if (g_cat[i].sink[0]) {
            any = 1;
            b[0] = 0;
            s_cat(b, k_names[i]); s_cat(b, " -> "); s_cat(b, g_cat[i].sink);
            logline(b);
        }
    }
    if (!any) { logline("no [Stems] outputs named: nothing patched"); return; }
    for (i = 0; i < (int)(sizeof k_sigs / sizeof k_sigs[0]); i++)
        for (k = 0; k < k_sigs[i].len; k++)
            if (((const BYTE *)k_sigs[i].at)[k] != k_sigs[i].sig[k]) {
                b[0] = 0;
                s_cat(b, "NOT PATCHED: bytes at "); s_hex(b, k_sigs[i].at);
                s_cat(b, " differ -- not the Armada2.exe this was built for");
                logline(b);
                return;
            }
    p = (BYTE *)INIT_CALL;
    if (!VirtualProtect(p, 5, PAGE_EXECUTE_READWRITE, &old)) { logline("NOT PATCHED: VirtualProtect failed"); return; }
    *(DWORD *)(p + 1) = (DWORD)&stems_init - (INIT_CALL + 5);
    VirtualProtect(p, 5, old, &old);
    FlushInstructionCache(GetCurrentProcess(), p, 5);
    logline("AudioSound2D::mInit hooked");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
