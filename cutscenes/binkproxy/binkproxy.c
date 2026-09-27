/*
 * binkw32.dll replacement for Star Trek: Armada II.
 *
 * The game imports ten functions from binkw32.dll.  This DLL exports the same
 * ten, under the same decorated names, and does two things with them:
 *
 *   1. REPLACEMENT MOVIES.  When BinkOpen is asked for "<name>.bik" and a
 *      "<name>.mp4" sits beside it, the movie is decoded with Media
 *      Foundation instead (which Proton backs with GStreamer), and the BINK
 *      struct the game reads is faked.  Anything else is passed straight to
 *      the real DLL, renamed binkw32_orig.dll.
 *
 *   2. FULL-SCREEN LAUNCH REELS.  Program::PlayIntroMovie switches to a
 *      hard-coded 640x480 display mode, and NextBinkFrame copies each frame
 *      1:1 into the locked back buffer at (0,0).  This DLL raises that mode
 *      to the desktop size at load, and when a copy comes from that call site
 *      it scales the frame to fit the back buffer, centred, bars black.  That
 *      applies to real Bink reels too: they are decoded into a scratch buffer
 *      first and scaled from there.
 *
 * What the game reads from a BINK, established from Armada2.exe:
 *   +0x00 Width, +0x04 Height, +0x08 Frames, +0x0c FrameNum (1-based).
 * NextBinkFrame stops a movie when FrameNum == Frames-1 after a copy, so at
 * end of stream Frames is pulled in to FrameNum+1.
 *
 * AudioManager::Open passes the ADDRESS of BinkOpenDirectSound (read out of
 * the import table, not called) to BinkSetSoundSystem, so that address is
 * ours and is swapped for the real one on the way through.
 *
 * No CRT, same toolchain as msaa/: see build.sh.
 */

typedef unsigned char       BYTE;
typedef unsigned short      WORD;
typedef unsigned long       DWORD;
typedef unsigned int        UINT;
typedef int                 BOOL;
typedef long                LONG;
typedef long                HRESULT;
typedef long long           LONGLONG;
typedef unsigned long long  ULONGLONG;
typedef void               *HANDLE;
typedef void               *HMODULE;
typedef unsigned short      WCHAR;
typedef unsigned long       ULONG_PTR;

#define WINAPI   __stdcall
#define NULLPTR  ((void *)0)
#define EXPORT   __declspec(dllexport)

typedef struct { DWORD Data1; WORD Data2; WORD Data3; BYTE Data4[8]; } GUID;
typedef struct { LONGLONG QuadPart; } LARGE_INTEGER;

/* ---- kernel32 / user32 / winmm (static imports, see *.def) -------------- */
__declspec(dllimport) HMODULE WINAPI LoadLibraryA(const char *);
__declspec(dllimport) void *  WINAPI GetProcAddress(HMODULE, const char *);
__declspec(dllimport) HMODULE WINAPI GetModuleHandleA(const char *);
__declspec(dllimport) DWORD   WINAPI GetModuleFileNameA(HMODULE, char *, DWORD);
__declspec(dllimport) BOOL    WINAPI VirtualProtect(void *, DWORD, DWORD, DWORD *);
__declspec(dllimport) BOOL    WINAPI FlushInstructionCache(HANDLE, const void *, DWORD);
__declspec(dllimport) HANDLE  WINAPI GetCurrentProcess(void);
__declspec(dllimport) HANDLE  WINAPI CreateFileA(const char *, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    WINAPI WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) BOOL    WINAPI ReadFile(HANDLE, void *, DWORD, DWORD *, void *);
__declspec(dllimport) DWORD   WINAPI GetFileSize(HANDLE, DWORD *);
__declspec(dllimport) DWORD   WINAPI SetFilePointer(HANDLE, LONG, LONG *, DWORD);
__declspec(dllimport) BOOL    WINAPI CloseHandle(HANDLE);
__declspec(dllimport) DWORD   WINAPI GetFileAttributesA(const char *);
__declspec(dllimport) DWORD   WINAPI GetFullPathNameA(const char *, DWORD, char *, char **);
__declspec(dllimport) BOOL    WINAPI QueryPerformanceCounter(LARGE_INTEGER *);
__declspec(dllimport) BOOL    WINAPI QueryPerformanceFrequency(LARGE_INTEGER *);
__declspec(dllimport) void *  WINAPI VirtualAlloc(void *, DWORD, DWORD, DWORD);
__declspec(dllimport) BOOL    WINAPI VirtualFree(void *, DWORD, DWORD);
__declspec(dllimport) UINT    WINAPI GetPrivateProfileIntA(const char *, const char *, int, const char *);
__declspec(dllimport) int     WINAPI MultiByteToWideChar(UINT, DWORD, const char *, int, WCHAR *, int);
__declspec(dllimport) void    WINAPI Sleep(DWORD);
__declspec(dllimport) BOOL    WINAPI IsBadWritePtr(void *, UINT);
__declspec(dllimport) int     WINAPI GetSystemMetrics(int);

typedef struct { WORD wFormatTag, nChannels; DWORD nSamplesPerSec, nAvgBytesPerSec;
                 WORD nBlockAlign, wBitsPerSample, cbSize; } WAVEFORMATEX;
typedef struct { char *lpData; DWORD dwBufferLength, dwBytesRecorded; ULONG_PTR dwUser;
                 DWORD dwFlags, dwLoops; void *lpNext; ULONG_PTR reserved; } WAVEHDR;
__declspec(dllimport) UINT WINAPI waveOutOpen(HANDLE *, UINT, const WAVEFORMATEX *, ULONG_PTR, ULONG_PTR, DWORD);
__declspec(dllimport) UINT WINAPI waveOutPrepareHeader(HANDLE, WAVEHDR *, UINT);
__declspec(dllimport) UINT WINAPI waveOutUnprepareHeader(HANDLE, WAVEHDR *, UINT);
__declspec(dllimport) UINT WINAPI waveOutWrite(HANDLE, WAVEHDR *, UINT);
__declspec(dllimport) UINT WINAPI waveOutReset(HANDLE);
__declspec(dllimport) UINT WINAPI waveOutClose(HANDLE);
__declspec(dllimport) UINT WINAPI waveOutSetVolume(HANDLE, DWORD);

#define SM_CXSCREEN 0
#define SM_CYSCREEN 1
#define MEM_COMMIT_RESERVE 0x3000
#define MEM_RELEASE 0x8000
#define PAGE_READWRITE 0x04
#define PAGE_EXECUTE_READWRITE 0x40
#define INVALID_FILE_ATTRIBUTES 0xFFFFFFFF
#define FILE_ATTRIBUTE_DIRECTORY 0x10
#define WAVE_MAPPER 0xFFFFFFFF

int _fltused = 0;

void *memset(void *d, int c, unsigned n)
{
    BYTE *p = d;
    while (n--) *p++ = (BYTE)c;
    return d;
}

void *memcpy(void *d, const void *s, unsigned n)
{
    BYTE *p = d; const BYTE *q = s;
    while (n--) *p++ = *q++;
    return d;
}

/* ---- small string helpers ------------------------------------------------ */
static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }
static void s_cpy(char *d, const char *s) { while ((*d++ = *s++)) ; }
static void s_cat(char *d, const char *s) { s_cpy(d + s_len(d), s); }
static void s_num(char *d, long v)
{
    char b[16]; int i = 0; unsigned long u;
    if (v < 0) { s_cat(d, "-"); u = (unsigned long)(-v); } else u = (unsigned long)v;
    do { b[i++] = (char)('0' + u % 10); u /= 10; } while (u);
    d += s_len(d);
    while (i) *d++ = b[--i];
    *d = 0;
}
static void s_hex(char *d, unsigned long v)
{
    static const char hx[] = "0123456789abcdef";
    int i; d += s_len(d);
    *d++ = '0'; *d++ = 'x';
    for (i = 7; i >= 0; i--) *d++ = hx[(v >> (i * 4)) & 15];
    *d = 0;
}

/* ---- config and log, both beside Armada2.exe ------------------------------ */
static char g_dir[260];
static char g_ini[280];
static int  g_log = 1, g_replace = 1, g_raise = 1, g_fit = 1;

static void logline(const char *msg)
{
    char path[300];
    HANDLE h;
    DWORD w;
    if (!g_log) return;
    s_cpy(path, g_dir); s_cat(path, "BinkProxy.log");
    h = CreateFileA(path, 0x40000000 /*GENERIC_WRITE*/, 1, NULLPTR, 4 /*OPEN_ALWAYS*/, 0x80, NULLPTR);
    if (h == (HANDLE)-1) return;
    SetFilePointer(h, 0, NULLPTR, 2 /*FILE_END*/);
    WriteFile(h, msg, (DWORD)s_len(msg), &w, NULLPTR);
    WriteFile(h, "\r\n", 2, &w, NULLPTR);
    CloseHandle(h);
}

/* ---- the real DLL ---------------------------------------------------------- */
typedef void *(WINAPI *pOpen)(const char *, DWORD);
typedef int   (WINAPI *pOne)(void *);
typedef void  (WINAPI *pVoidOne)(void *);
typedef int   (WINAPI *pCopy)(void *, void *, DWORD, DWORD, DWORD, DWORD, DWORD);
typedef void  (WINAPI *pGoto)(void *, DWORD, DWORD);
typedef void  (WINAPI *pSetVol)(void *, int);
typedef int   (WINAPI *pSetSS)(void *, void *);
typedef void *(WINAPI *pOpenDS)(ULONG_PTR);

static struct {
    HMODULE  dll;
    pOpen    Open;
    pOne     DoFrame, Wait;
    pVoidOne NextFrame, Close;
    pCopy    CopyToBuffer;
    pGoto    Goto;
    pSetVol  SetVolume;
    pSetSS   SetSoundSystem;
    pOpenDS  OpenDirectSound;
} R;

static int real_load(void)
{
    char path[300];
    if (R.dll) return 1;
    s_cpy(path, g_dir); s_cat(path, "binkw32_orig.dll");
    R.dll = LoadLibraryA(path);
    if (!R.dll) { logline("FATAL: cannot load binkw32_orig.dll"); return 0; }
    R.Open            = (pOpen)   GetProcAddress(R.dll, "_BinkOpen@8");
    R.DoFrame         = (pOne)    GetProcAddress(R.dll, "_BinkDoFrame@4");
    R.Wait            = (pOne)    GetProcAddress(R.dll, "_BinkWait@4");
    R.NextFrame       = (pVoidOne)GetProcAddress(R.dll, "_BinkNextFrame@4");
    R.Close           = (pVoidOne)GetProcAddress(R.dll, "_BinkClose@4");
    R.CopyToBuffer    = (pCopy)   GetProcAddress(R.dll, "_BinkCopyToBuffer@28");
    R.Goto            = (pGoto)   GetProcAddress(R.dll, "_BinkGoto@12");
    R.SetVolume       = (pSetVol) GetProcAddress(R.dll, "_BinkSetVolume@8");
    R.SetSoundSystem  = (pSetSS)  GetProcAddress(R.dll, "_BinkSetSoundSystem@8");
    R.OpenDirectSound = (pOpenDS) GetProcAddress(R.dll, "_BinkOpenDirectSound@4");
    if (!R.Open || !R.DoFrame || !R.Wait || !R.NextFrame || !R.Close || !R.CopyToBuffer ||
        !R.Goto || !R.SetVolume || !R.SetSoundSystem || !R.OpenDirectSound) {
        logline("FATAL: binkw32_orig.dll is missing an export");
        return 0;
    }
    return 1;
}

/* ---- Media Foundation, declared by hand ------------------------------------
 * Every GUID below was checked byte-for-byte against Proton's own mfplat.dll.
 * Methods are called by vtable index; the indices follow the interface
 * declarations in mfobjects.idl / mfreadwrite.idl:
 *   IUnknown 0-2, IMFAttributes 3-32, IMFSample 33-46, IMFMediaBuffer 3-7,
 *   IMFSourceReader 3-12.                                                     */
static const GUID MFMediaType_Video   = {0x73646976,0x0000,0x0010,{0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71}};
static const GUID MFMediaType_Audio   = {0x73647561,0x0000,0x0010,{0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71}};
static const GUID MFVideoFormat_RGB32 = {0x00000016,0x0000,0x0010,{0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71}};
static const GUID MFVideoFormat_NV12  = {0x3231564E,0x0000,0x0010,{0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71}};
static const GUID MFAudioFormat_PCM   = {0x00000001,0x0000,0x0010,{0x80,0x00,0x00,0xAA,0x00,0x38,0x9B,0x71}};
static const GUID MF_MT_MAJOR_TYPE    = {0x48eba18e,0xf8c9,0x4687,{0xbf,0x11,0x0a,0x74,0xc9,0xf9,0x6a,0x8f}};
static const GUID MF_MT_SUBTYPE       = {0xf7e34c9a,0x42e8,0x4714,{0xb7,0x4b,0xcb,0x29,0xd7,0x2c,0x35,0xe5}};
static const GUID MF_MT_FRAME_SIZE    = {0x1652c33d,0xd6b2,0x4012,{0xb8,0x34,0x72,0x03,0x08,0x49,0xa3,0x7d}};
static const GUID MF_MT_FRAME_RATE    = {0xc459a2e8,0x3d2c,0x4e44,{0xb1,0x32,0xfe,0xe5,0x15,0x6c,0x7b,0xb0}};
static const GUID MF_MT_DEFAULT_STRIDE= {0x644b4e48,0x1e02,0x4516,{0xb0,0xeb,0xc0,0x1c,0xa9,0xd4,0x9a,0xc6}};
static const GUID MF_MT_AUDIO_NUM_CHANNELS       = {0x37e48bf5,0x645e,0x4c5b,{0x89,0xde,0xad,0xa9,0xe2,0x9b,0x69,0x6a}};
static const GUID MF_MT_AUDIO_SAMPLES_PER_SECOND = {0x5faeeae7,0x0290,0x4c31,{0x9e,0x8a,0xc5,0x34,0xf6,0x8d,0x9d,0xba}};
static const GUID MF_MT_AUDIO_BITS_PER_SAMPLE    = {0xf2deb57f,0x40fa,0x4764,{0xaa,0x33,0xed,0x4f,0x2d,0x1f,0xf6,0x69}};
static const GUID MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING = {0xfb394f3d,0xccf1,0x42ee,{0xbb,0xb3,0xf9,0xb8,0x45,0xd5,0x68,0x1d}};
static const GUID MF_PD_DURATION      = {0x6c990d33,0xbb8e,0x477a,{0x85,0x98,0x0d,0x5d,0x96,0xfc,0xd8,0x8a}};

#define MF_SOURCE_READER_FIRST_VIDEO_STREAM 0xFFFFFFFC
#define MF_SOURCE_READER_FIRST_AUDIO_STREAM 0xFFFFFFFD
#define MF_SOURCE_READER_ALL_STREAMS        0xFFFFFFFE
#define MF_SOURCE_READER_MEDIASOURCE        0xFFFFFFFF
#define MF_SOURCE_READERF_ERROR             0x1
#define MF_SOURCE_READERF_ENDOFSTREAM       0x2
#define MF_VERSION                          0x00020070

typedef void **COM;                     /* pointer to vtable pointer */
#define VT(o, i) ((*(void ***)(o))[i])

typedef ULONG_PTR (WINAPI *fRelease)(COM);
typedef HRESULT (WINAPI *fSetUINT32)(COM, const GUID *, UINT);
typedef HRESULT (WINAPI *fGetUINT32)(COM, const GUID *, UINT *);
typedef HRESULT (WINAPI *fGetUINT64)(COM, const GUID *, ULONGLONG *);
typedef HRESULT (WINAPI *fSetGUID)(COM, const GUID *, const GUID *);
typedef HRESULT (WINAPI *fGetGUID)(COM, const GUID *, GUID *);
typedef HRESULT (WINAPI *fGetNative)(COM, DWORD, DWORD, COM *);
typedef HRESULT (WINAPI *fSetStreamSel)(COM, DWORD, BOOL);
typedef HRESULT (WINAPI *fGetCurType)(COM, DWORD, COM *);
typedef HRESULT (WINAPI *fSetCurType)(COM, DWORD, DWORD *, COM);
typedef HRESULT (WINAPI *fReadSample)(COM, DWORD, DWORD, DWORD *, DWORD *, LONGLONG *, COM *);
typedef HRESULT (WINAPI *fGetPresAttr)(COM, DWORD, const GUID *, void *);
typedef HRESULT (WINAPI *fToContig)(COM, COM *);
typedef HRESULT (WINAPI *fLock)(COM, BYTE **, DWORD *, DWORD *);
typedef HRESULT (WINAPI *fUnlock)(COM);

#define RELEASE(o)           do { if (o) ((fRelease)VT(o, 2))(o); (o) = 0; } while (0)
#define ATTR_SET_U32(o,k,v)  ((fSetUINT32)VT(o, 21))(o, k, v)
#define ATTR_GET_U32(o,k,v)  ((fGetUINT32)VT(o, 7))(o, k, v)
#define ATTR_GET_U64(o,k,v)  ((fGetUINT64)VT(o, 8))(o, k, v)
#define ATTR_SET_GUID(o,k,v) ((fSetGUID)VT(o, 24))(o, k, v)
#define ATTR_GET_GUID(o,k,v) ((fGetGUID)VT(o, 10))(o, k, v)
#define SR_NATIVE(o,s,i,t)   ((fGetNative)VT(o, 5))(o, s, i, t)
#define SR_SELECT(o,s,b)     ((fSetStreamSel)VT(o, 4))(o, s, b)
#define SR_GETTYPE(o,s,t)    ((fGetCurType)VT(o, 6))(o, s, t)
#define SR_SETTYPE(o,s,t)    ((fSetCurType)VT(o, 7))(o, s, NULLPTR, t)
#define SR_READ(o,s,i,f,t,x) ((fReadSample)VT(o, 9))(o, s, 0, i, f, t, x)
#define SR_PRESATTR(o,s,k,p) ((fGetPresAttr)VT(o, 12))(o, s, k, p)
#define SAMPLE_CONTIG(o,b)   ((fToContig)VT(o, 41))(o, b)
#define BUF_LOCK(o,p,m,c)    ((fLock)VT(o, 3))(o, p, m, c)
#define BUF_UNLOCK(o)        ((fUnlock)VT(o, 4))(o)

static HRESULT (WINAPI *pCoInitializeEx)(void *, DWORD);
static HRESULT (WINAPI *pMFStartup)(ULONG_PTR, DWORD);
static HRESULT (WINAPI *pMFCreateAttributes)(COM *, UINT);
static HRESULT (WINAPI *pMFCreateMediaType)(COM *);
static HRESULT (WINAPI *pMFCreateSourceReaderFromURL)(const WCHAR *, COM, COM *);

static int mf_ready = 0;   /* 0 untried, 1 ok, -1 failed */

/* Loaded on first use, not imported: a missing Media Foundation must cost
 * the replacement movies, not the whole game. */
static int mf_init(void)
{
    HMODULE ole, plat, rw;
    char m[160];
    HRESULT hr;
    if (mf_ready) return mf_ready > 0;
    mf_ready = -1;
    ole  = LoadLibraryA("ole32.dll");
    plat = LoadLibraryA("mfplat.dll");
    rw   = LoadLibraryA("mfreadwrite.dll");
    if (!ole || !plat || !rw) { logline("MF: a DLL did not load"); return 0; }
    pCoInitializeEx = GetProcAddress(ole, "CoInitializeEx");
    pMFStartup = GetProcAddress(plat, "MFStartup");
    pMFCreateAttributes = GetProcAddress(plat, "MFCreateAttributes");
    pMFCreateMediaType = GetProcAddress(plat, "MFCreateMediaType");
    pMFCreateSourceReaderFromURL = GetProcAddress(rw, "MFCreateSourceReaderFromURL");
    if (!pCoInitializeEx || !pMFStartup || !pMFCreateAttributes || !pMFCreateMediaType ||
        !pMFCreateSourceReaderFromURL) { logline("MF: an export is missing"); return 0; }
    pCoInitializeEx(NULLPTR, 0 /*COINIT_MULTITHREADED*/);   /* RPC_E_CHANGED_MODE is fine */
    hr = pMFStartup(MF_VERSION, 0);
    if (hr < 0) { m[0] = 0; s_cat(m, "MF: MFStartup failed "); s_hex(m, (unsigned long)hr); logline(m); return 0; }
    mf_ready = 1;
    return 1;
}

/* ---- timing ---------------------------------------------------------------- */
static double qpc_freq;
static double now_s(void)
{
    LARGE_INTEGER c;
    if (qpc_freq == 0) { LARGE_INTEGER f; QueryPerformanceFrequency(&f); qpc_freq = (double)f.QuadPart; }
    QueryPerformanceCounter(&c);
    return (double)c.QuadPart / qpc_freq;
}

/* ---- replacement movies ---------------------------------------------------- */
typedef struct {
    DWORD  bink[64];           /* what the game sees; MUST stay first */
    int    used;
    COM    reader;
    int    w, h, stride, nv12;
    double fps;
    BYTE  *frame;              /* w*h BGRA, top-down */
    int    eos, started, late, shown;
    double t0;
    BYTE  *pcm;
    DWORD  pcmLen;
    WAVEFORMATEX wf;
    HANDLE wo;
    WAVEHDR wh;
    int    volume;             /* Bink scale, 32768 = full */
    double decodeMax, decodeSum;
    char   name[64];
} Movie;

#define MAX_MOVIES 4
static Movie g_mv[MAX_MOVIES];

static Movie *ours(void *bink)
{
    int i;
    for (i = 0; i < MAX_MOVIES; i++)
        if (g_mv[i].used && (void *)g_mv[i].bink == bink) return &g_mv[i];
    return NULLPTR;
}

static void *valloc(DWORD n) { return VirtualAlloc(NULLPTR, n, MEM_COMMIT_RESERVE, PAGE_READWRITE); }
static void  vfree(void *p)  { if (p) VirtualFree(p, 0, MEM_RELEASE); }

static void to_wide(const char *s, WCHAR *w, int cap)
{
    int n = MultiByteToWideChar(0 /*CP_ACP*/, 0, s, -1, w, cap);
    if (n <= 0) w[0] = 0;
}

static void base_name(const char *path, char *out, int cap)
{
    const char *b = path, *p;
    int n = 0;
    for (p = path; *p; p++) if (*p == '\\' || *p == '/') b = p + 1;
    while (b[n] && n < cap - 1) { out[n] = b[n]; n++; }
    out[n] = 0;
}

/* Decode the whole audio track up front with its own reader: reading audio
 * through the video reader would make it buffer every video frame it skips. */
static void load_audio(Movie *mv, const WCHAR *url, double seconds)
{
    COM rd = 0, mt = 0, cur = 0, s = 0, buf = 0;
    UINT ch = 0, rate = 0, bits = 0;
    DWORD cap, idx, fl;
    LONGLONG ts;
    char m[200];

    if (pMFCreateSourceReaderFromURL(url, 0, &rd) < 0) { logline("audio: no reader"); return; }
    SR_SELECT(rd, MF_SOURCE_READER_ALL_STREAMS, 0);
    if (SR_SELECT(rd, MF_SOURCE_READER_FIRST_AUDIO_STREAM, 1) < 0) { logline("audio: no audio stream"); goto out; }
    if (pMFCreateMediaType(&mt) < 0) goto out;
    ATTR_SET_GUID(mt, &MF_MT_MAJOR_TYPE, &MFMediaType_Audio);
    ATTR_SET_GUID(mt, &MF_MT_SUBTYPE, &MFAudioFormat_PCM);
    ATTR_SET_U32(mt, &MF_MT_AUDIO_BITS_PER_SAMPLE, 16);
    if (SR_SETTYPE(rd, MF_SOURCE_READER_FIRST_AUDIO_STREAM, mt) < 0) { logline("audio: PCM refused"); goto out; }
    if (SR_GETTYPE(rd, MF_SOURCE_READER_FIRST_AUDIO_STREAM, &cur) < 0) goto out;
    ATTR_GET_U32(cur, &MF_MT_AUDIO_NUM_CHANNELS, &ch);
    ATTR_GET_U32(cur, &MF_MT_AUDIO_SAMPLES_PER_SECOND, &rate);
    ATTR_GET_U32(cur, &MF_MT_AUDIO_BITS_PER_SAMPLE, &bits);
    if (!ch || !rate || bits != 16) { logline("audio: unexpected format"); goto out; }

    cap = (DWORD)((seconds + 2.0) * rate) * ch * 2;
    mv->pcm = valloc(cap);
    if (!mv->pcm) goto out;
    mv->pcmLen = 0;
    for (;;) {
        BYTE *p; DWORD len;
        fl = 0; s = 0;
        if (SR_READ(rd, MF_SOURCE_READER_FIRST_AUDIO_STREAM, &idx, &fl, &ts, &s) < 0) break;
        if (s) {
            if (SAMPLE_CONTIG(s, &buf) >= 0 && BUF_LOCK(buf, &p, NULLPTR, &len) >= 0) {
                if (mv->pcmLen + len > cap) len = cap - mv->pcmLen;
                memcpy(mv->pcm + mv->pcmLen, p, len);
                mv->pcmLen += len;
                BUF_UNLOCK(buf);
            }
            RELEASE(buf);
            RELEASE(s);
        }
        if (fl & (MF_SOURCE_READERF_ENDOFSTREAM | MF_SOURCE_READERF_ERROR)) break;
        if (mv->pcmLen >= cap) break;
    }
    mv->wf.wFormatTag = 1;
    mv->wf.nChannels = (WORD)ch;
    mv->wf.nSamplesPerSec = rate;
    mv->wf.wBitsPerSample = 16;
    mv->wf.nBlockAlign = (WORD)(ch * 2);
    mv->wf.nAvgBytesPerSec = rate * ch * 2;
    m[0] = 0;
    s_cat(m, "audio: "); s_num(m, (long)ch); s_cat(m, "ch "); s_num(m, (long)rate);
    s_cat(m, "Hz, "); s_num(m, (long)((double)mv->pcmLen / (ch * 2) * 1000.0 / rate)); s_cat(m, " ms decoded");
    logline(m);
out:
    RELEASE(cur); RELEASE(mt); RELEASE(rd);
}

/* <name>.wav beside the movie: 16-bit PCM, read here without any decoder.
 * Proton's Media Foundation refuses the AAC in an MP4 (the same media-
 * converter path that refuses H.264), so this is how the audio gets in. */
static void load_wav(Movie *mv, const char *path)
{
    HANDLE f = CreateFileA(path, 0x80000000 /*GENERIC_READ*/, 1, NULLPTR, 3 /*OPEN_EXISTING*/, 0x80, NULLPTR);
    DWORD size, got, pos = 12, fmtOk = 0;
    BYTE *d;
    char m[200];
    if (f == (HANDLE)-1) return;
    size = GetFileSize(f, NULLPTR);
    d = (size > 44 && size < 0x40000000) ? valloc(size) : NULLPTR;
    if (!d || !ReadFile(f, d, size, &got, NULLPTR) || got != size ||
        *(DWORD *)d != 0x46464952 /*RIFF*/ || *(DWORD *)(d + 8) != 0x45564157 /*WAVE*/) {
        CloseHandle(f); vfree(d); logline("audio: .wav unreadable"); return;
    }
    CloseHandle(f);
    while (pos + 8 <= size) {
        DWORD id = *(DWORD *)(d + pos), len = *(DWORD *)(d + pos + 4);
        BYTE *c = d + pos + 8;
        if (len > size - pos - 8) len = size - pos - 8;
        if (id == 0x20746d66 /*fmt */ && len >= 16) {
            memcpy(&mv->wf, c, 16);
            mv->wf.cbSize = 0;
            fmtOk = mv->wf.wFormatTag == 1 && mv->wf.wBitsPerSample == 16 && mv->wf.nChannels &&
                    mv->wf.nChannels <= 2 && mv->wf.nSamplesPerSec;
        } else if (id == 0x61746164 /*data*/ && fmtOk) {
            /* keep the whole file's allocation; play from the data chunk */
            memcpy(d, c, len);
            mv->pcm = d;
            mv->pcmLen = len;
            m[0] = 0; s_cat(m, "audio: "); s_num(m, mv->wf.nChannels); s_cat(m, "ch ");
            s_num(m, (long)mv->wf.nSamplesPerSec); s_cat(m, "Hz from .wav, ");
            s_num(m, (long)((double)len / mv->wf.nBlockAlign * 1000.0 / mv->wf.nSamplesPerSec)); s_cat(m, " ms");
            logline(m);
            return;
        }
        pos += 8 + len + (len & 1);
    }
    vfree(d);
    logline("audio: .wav is not 16-bit PCM");
}

static Movie *open_replacement(const char *bikname)
{
    char mp4[300], full[300], m[400];
    WCHAR url[300];
    COM attr = 0, mt = 0, cur = 0;
    ULONGLONG fs = 0, fr = 0;
    UINT stride = 0;
    struct { WORD vt, r1, r2, r3; ULONGLONG v; } pv;
    double dur;
    Movie *mv = NULLPTR;
    int i, n;
    DWORD a;

    n = s_len(bikname);
    if (n < 5 || n > 250) return NULLPTR;
    s_cpy(mp4, bikname);
    if (mp4[n - 4] != '.') return NULLPTR;
    s_cpy(mp4 + n - 4, ".mp4");
    a = GetFileAttributesA(mp4);
    if (a == INVALID_FILE_ATTRIBUTES || (a & FILE_ATTRIBUTE_DIRECTORY)) return NULLPTR;
    if (!GetFullPathNameA(mp4, sizeof full, full, NULLPTR)) return NULLPTR;
    if (!mf_init()) return NULLPTR;

    for (i = 0; i < MAX_MOVIES; i++) if (!g_mv[i].used) { mv = &g_mv[i]; break; }
    if (!mv) { logline("no free movie slot"); return NULLPTR; }
    memset(mv, 0, sizeof *mv);
    base_name(full, mv->name, sizeof mv->name);
    to_wide(full, url, 300);

    if (pMFCreateAttributes(&attr, 1) < 0) goto fail;
    ATTR_SET_U32(attr, &MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1);
    if (pMFCreateSourceReaderFromURL(url, attr, &mv->reader) < 0) { logline("video: reader failed"); goto fail; }
    SR_SELECT(mv->reader, MF_SOURCE_READER_ALL_STREAMS, 0);
    SR_SELECT(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, 1);
    if (pMFCreateMediaType(&mt) < 0) goto fail;
    ATTR_SET_GUID(mt, &MF_MT_MAJOR_TYPE, &MFMediaType_Video);
    /* RGB32 through the source reader's video processor where it exists;
     * Proton's Wine refuses it, so NV12 straight off the decoder and the
     * conversion done here (nv12_to_bgra). */
    {
        COM nat = 0;
        GUID sub;
        HRESULT hr1, hr2;
        m[0] = 0; s_cat(m, "video: native subtype ");
        if (SR_NATIVE(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, 0, &nat) >= 0 &&
            ATTR_GET_GUID(nat, &MF_MT_SUBTYPE, &sub) >= 0) s_hex(m, sub.Data1);
        else s_cat(m, "unknown");
        RELEASE(nat);
        ATTR_SET_GUID(mt, &MF_MT_SUBTYPE, &MFVideoFormat_RGB32);
        hr1 = SR_SETTYPE(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, mt);
        if (hr1 < 0) {
            ATTR_SET_GUID(mt, &MF_MT_SUBTYPE, &MFVideoFormat_NV12);
            hr2 = SR_SETTYPE(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, mt);
            s_cat(m, "; RGB32 hr "); s_hex(m, (unsigned long)hr1);
            s_cat(m, "; NV12 hr ");  s_hex(m, (unsigned long)hr2);
            logline(m);
            if (hr2 < 0) goto fail;
            mv->nv12 = 1;
        } else logline(m);
    }
    if (SR_GETTYPE(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, &cur) < 0) goto fail;
    ATTR_GET_U64(cur, &MF_MT_FRAME_SIZE, &fs);
    ATTR_GET_U64(cur, &MF_MT_FRAME_RATE, &fr);
    mv->w = (int)(fs >> 32);
    mv->h = (int)(fs & 0xFFFFFFFF);
    mv->fps = (fr & 0xFFFFFFFF) ? (double)(DWORD)(fr >> 32) / (double)(DWORD)(fr & 0xFFFFFFFF) : 0;
    if (ATTR_GET_U32(cur, &MF_MT_DEFAULT_STRIDE, &stride) >= 0) mv->stride = (int)stride;
    else mv->stride = mv->nv12 ? mv->w : mv->w * 4;
    if (mv->nv12 && ((mv->w | mv->h) & 1)) { logline("video: NV12 needs even dimensions"); goto fail; }
    if (mv->w < 16 || mv->h < 16 || mv->w > 8192 || mv->h > 8192 || mv->fps < 1 || mv->fps > 240) {
        logline("video: implausible size or frame rate"); goto fail;
    }

    memset(&pv, 0, sizeof pv);
    dur = 0;
    if (SR_PRESATTR(mv->reader, MF_SOURCE_READER_MEDIASOURCE, &MF_PD_DURATION, &pv) >= 0)
        dur = (double)pv.v / 1e7;
    if (dur <= 0) dur = 3600;

    mv->frame = valloc((DWORD)(mv->w * mv->h * 4));
    if (!mv->frame) goto fail;

    mv->bink[0] = (DWORD)mv->w;
    mv->bink[1] = (DWORD)mv->h;
    mv->bink[2] = (DWORD)(dur * mv->fps + 0.5);
    mv->bink[3] = 1;
    mv->bink[5] = (DWORD)(mv->fps * 1000 + 0.5);
    mv->bink[6] = 1000;
    mv->volume = 32768;

    load_audio(mv, url, dur);
    if (!mv->pcmLen) {
        vfree(mv->pcm);
        mv->pcm = NULLPTR;
        s_cpy(full + s_len(full) - 4, ".wav");
        load_wav(mv, full);
    }

    m[0] = 0;
    s_cat(m, "open "); s_cat(m, mv->name); s_cat(m, ": ");
    s_num(m, mv->w); s_cat(m, "x"); s_num(m, mv->h); s_cat(m, " @ ");
    s_num(m, (long)(mv->fps * 1000)); s_cat(m, "/1000 fps, ");
    s_cat(m, mv->nv12 ? "NV12" : "RGB32"); s_cat(m, " stride "); s_num(m, mv->stride);
    s_cat(m, ", "); s_num(m, (long)mv->bink[2]); s_cat(m, " frames");
    logline(m);
    RELEASE(cur); RELEASE(mt); RELEASE(attr);
    mv->used = 1;
    return mv;

fail:
    RELEASE(cur); RELEASE(mt); RELEASE(attr);
    RELEASE(mv->reader);
    vfree(mv->frame);
    vfree(mv->pcm);
    memset(mv, 0, sizeof *mv);
    logline("replacement failed; falling back to the .bik");
    return NULLPTR;
}

static void audio_start(Movie *mv)
{
    DWORD v;
    if (!mv->pcm || !mv->pcmLen) return;
    if (waveOutOpen(&mv->wo, WAVE_MAPPER, &mv->wf, 0, 0, 0)) { mv->wo = 0; logline("audio: waveOutOpen failed"); return; }
    mv->wh.lpData = (char *)mv->pcm;
    mv->wh.dwBufferLength = mv->pcmLen;
    waveOutPrepareHeader(mv->wo, &mv->wh, sizeof mv->wh);
    v = (DWORD)mv->volume * 0xFFFF / 32768;
    if (v > 0xFFFF) v = 0xFFFF;
    waveOutSetVolume(mv->wo, v | (v << 16));
    waveOutWrite(mv->wo, &mv->wh, sizeof mv->wh);
}

static void movie_close(Movie *mv)
{
    char m[200];
    if (mv->wo) {
        waveOutReset(mv->wo);
        waveOutUnprepareHeader(mv->wo, &mv->wh, sizeof mv->wh);
        waveOutClose(mv->wo);
    }
    m[0] = 0;
    s_cat(m, "close "); s_cat(m, mv->name); s_cat(m, ": ");
    s_num(m, mv->shown); s_cat(m, " frames shown, ");
    s_num(m, mv->late); s_cat(m, " late, decode avg ");
    s_num(m, mv->shown ? (long)(mv->decodeSum / mv->shown * 1000) : 0);
    s_cat(m, " ms, max "); s_num(m, (long)(mv->decodeMax * 1000)); s_cat(m, " ms");
    logline(m);
    RELEASE(mv->reader);
    vfree(mv->frame);
    vfree(mv->pcm);
    memset(mv, 0, sizeof *mv);
}

/* NV12 (Y plane, then interleaved U,V at half resolution) to B,G,R,X.
 * BT.601, limited range -- what Bink's own YUV was, and what
 * cutscenes/binkproxy/make-movie.sh tags the replacement movies as.  If the
 * decoder padded the height, the chroma plane starts after the padded luma
 * plane, which the buffer length gives away. */
static BYTE clamp8(int v) { return (BYTE)(v < 0 ? 0 : v > 255 ? 255 : v); }

static void nv12_to_bgra(const BYTE *p, DWORD len, int stride, int w, int h, BYTE *out)
{
    int ph = h, x, y;
    const BYTE *uv;
    if ((DWORD)(stride * h * 3 / 2) > len) return;
    /* padded height: len == stride * ph * 3/2 */
    if (len >= (DWORD)(stride * 3 / 2) && (int)(len / ((DWORD)stride * 3 / 2)) > h)
        ph = (int)(len * 2 / 3 / (DWORD)stride);
    uv = p + stride * ph;
    for (y = 0; y < h; y++) {
        const BYTE *yr = p + y * stride, *cr = uv + (y >> 1) * stride;
        DWORD *o = (DWORD *)(out + y * w * 4);
        for (x = 0; x < w; x += 2) {
            int u = cr[x] - 128, v = cr[x + 1] - 128;
            int rv = 409 * v, gu = -100 * u - 208 * v, bu = 516 * u;
            int c0 = 298 * (yr[x] - 16) + 128, c1 = 298 * (yr[x + 1] - 16) + 128;
            o[x]     = 0xFF000000u | ((DWORD)clamp8((c0 + rv) >> 8) << 16) |
                       ((DWORD)clamp8((c0 + gu) >> 8) << 8) | clamp8((c0 + bu) >> 8);
            o[x + 1] = 0xFF000000u | ((DWORD)clamp8((c1 + rv) >> 8) << 16) |
                       ((DWORD)clamp8((c1 + gu) >> 8) << 8) | clamp8((c1 + bu) >> 8);
        }
    }
}

static void movie_decode(Movie *mv)
{
    COM s = 0, buf = 0;
    DWORD idx, fl = 0;
    LONGLONG ts;
    BYTE *p;
    DWORD len;
    int tries, y, rs = mv->stride < 0 ? -mv->stride : mv->stride;
    double t = now_s(), dt;

    if (!mv->started) {
        mv->started = 1;
        mv->t0 = t;
        audio_start(mv);
    } else if (t - mv->t0 > (mv->bink[3] - 1) / mv->fps + 2.0 / mv->fps) {
        mv->late++;
    }
    if (mv->eos) return;
    for (tries = 0; tries < 64 && !s; tries++) {
        fl = 0;
        if (SR_READ(mv->reader, MF_SOURCE_READER_FIRST_VIDEO_STREAM, &idx, &fl, &ts, &s) < 0) { fl = MF_SOURCE_READERF_ERROR; break; }
        if (fl & (MF_SOURCE_READERF_ENDOFSTREAM | MF_SOURCE_READERF_ERROR)) break;
    }
    if (!s) {
        mv->eos = 1;
        /* NextBinkFrame ends the movie when FrameNum == Frames-1. */
        mv->bink[2] = mv->bink[3] + 1;
        return;
    }
    if (SAMPLE_CONTIG(s, &buf) >= 0 && BUF_LOCK(buf, &p, NULLPTR, &len) >= 0) {
        /* Proton reports MF_MT_DEFAULT_STRIDE in pixels for RGB32 (1280 for a
         * 1280-wide frame whose rows are 5120 bytes), so a stride that cannot
         * hold a row is replaced by the one the buffer length implies. */
        if (!mv->nv12 && rs < mv->w * 4) rs = (int)(len / (DWORD)mv->h);
        if (mv->nv12 && rs < mv->w) rs = (int)(len * 2 / 3 / (DWORD)mv->h);
        if (mv->nv12) {
            nv12_to_bgra(p, len, rs, mv->w, mv->h, mv->frame);
        } else if (len >= (DWORD)(rs * (mv->h - 1) + mv->w * 4)) {
            for (y = 0; y < mv->h; y++) {
                const BYTE *src = mv->stride < 0 ? p + (mv->h - 1 - y) * rs : p + y * rs;
                memcpy(mv->frame + y * mv->w * 4, src, (unsigned)(mv->w * 4));
            }
        }
        BUF_UNLOCK(buf);
    }
    RELEASE(buf);
    RELEASE(s);
    mv->shown++;
    dt = now_s() - t;
    mv->decodeSum += dt;
    if (dt > mv->decodeMax) mv->decodeMax = dt;
}

/* ---- pixel output ------------------------------------------------------------
 * Bink surface types as the game selects them (NextBinkFrame's jump table):
 *   A8R8G8B8 -> 5, X8R8G8B8 -> 3, R5G6B5 -> 10, X1R5G5B5 -> 9.
 * 3/5 are B,G,R,X in memory.  4/6 are the reversed-order variants.          */
static int type_bpp(int t)
{
    if (t >= 3 && t <= 6) return 4;
    if (t == 1 || t == 2) return 3;
    if (t >= 7 && t <= 12) return 2;
    return 0;
}

static void put_row(BYTE *d, int type, const DWORD *s, int n)
{
    int i;
    switch (type) {
    case 3: case 5:
        for (i = 0; i < n; i++) ((DWORD *)d)[i] = s[i] | 0xFF000000;
        break;
    case 4: case 6:
        for (i = 0; i < n; i++) { DWORD c = s[i];
            ((DWORD *)d)[i] = 0xFF000000 | ((c & 0xFF) << 16) | (c & 0xFF00) | ((c >> 16) & 0xFF); }
        break;
    case 1:
        for (i = 0; i < n; i++) { DWORD c = s[i]; d[3*i] = (BYTE)c; d[3*i+1] = (BYTE)(c >> 8); d[3*i+2] = (BYTE)(c >> 16); }
        break;
    case 2:
        for (i = 0; i < n; i++) { DWORD c = s[i]; d[3*i] = (BYTE)(c >> 16); d[3*i+1] = (BYTE)(c >> 8); d[3*i+2] = (BYTE)c; }
        break;
    case 10:
        for (i = 0; i < n; i++) { DWORD c = s[i];
            ((WORD *)d)[i] = (WORD)(((c >> 8) & 0xF800) | ((c >> 5) & 0x07E0) | ((c >> 3) & 0x001F)); }
        break;
    case 9:
        for (i = 0; i < n; i++) { DWORD c = s[i];
            ((WORD *)d)[i] = (WORD)(0x8000 | ((c >> 9) & 0x7C00) | ((c >> 6) & 0x03E0) | ((c >> 3) & 0x001F)); }
        break;
    default:
        memset(d, 0, (unsigned)(n * type_bpp(type)));
    }
}

static DWORD lerp_px(DWORD a, DWORD b, DWORD f)   /* f in 0..256 */
{
    DWORD g  = 256 - f;
    DWORD rb = (((a & 0x00FF00FF) * g + (b & 0x00FF00FF) * f) >> 8) & 0x00FF00FF;
    DWORD ag = (((a >> 8) & 0x00FF00FF) * g + ((b >> 8) & 0x00FF00FF) * f) & 0xFF00FF00;
    return rb | ag;
}

#define MAX_W 8192
static int   g_xs[MAX_W];         /* per output column: source column */
static BYTE  g_xf[MAX_W];         /* per output column: weight of the next column */
static DWORD g_row[MAX_W];        /* vertically blended source row */
static DWORD g_out[MAX_W];        /* horizontally resampled output row */
static DWORD g_black[MAX_W];

/* Scale a BGRA frame to fit (dw x dh), centred, bars black; bilinear. */
static void fit_blit(const BYTE *src, int sw, int sh, BYTE *dst, int pitch, int dw, int dh, int type)
{
    int bpp = type_bpp(type), ow, oh, ox, oy, x, y;
    if (sw < 2 || sh < 2 || dw > MAX_W || !bpp) return;
    if ((double)dw / sw < (double)dh / sh) { ow = dw; oh = (int)((double)sh * dw / sw + 0.5); }
    else                                   { oh = dh; ow = (int)((double)sw * dh / sh + 0.5); }
    if (ow > dw) ow = dw;
    if (oh > dh) oh = dh;
    ox = (dw - ow) / 2;
    oy = (dh - oh) / 2;

    for (x = 0; x < ow; x++) {
        double fx = ((double)x + 0.5) * sw / ow - 0.5;
        int ix;
        if (fx < 0) fx = 0;
        ix = (int)fx;
        if (ix >= sw - 1) { ix = sw - 2; fx = sw - 1; }
        g_xs[x] = ix;
        g_xf[x] = (BYTE)((fx - ix) * 255.0 + 0.5);
    }
    for (y = 0; y < dh; y++) {
        BYTE *d = dst + y * pitch;
        if (y < oy || y >= oy + oh) { put_row(d, type, g_black, dw); continue; }
        {
            double fy = ((double)(y - oy) + 0.5) * sh / oh - 0.5;
            int iy, f;
            const DWORD *r0, *r1;
            if (fy < 0) fy = 0;
            iy = (int)fy;
            if (iy >= sh - 1) { iy = sh - 2; fy = sh - 1; }
            f = (int)((fy - iy) * 256.0 + 0.5);
            r0 = (const DWORD *)(src + iy * sw * 4);
            r1 = (const DWORD *)(src + (iy + 1) * sw * 4);
            for (x = 0; x < sw; x++) g_row[x] = lerp_px(r0[x], r1[x], (DWORD)f);
            for (x = 0; x < ow; x++) g_out[x] = lerp_px(g_row[g_xs[x]], g_row[g_xs[x] + 1], g_xf[x]);
        }
        if (ox) put_row(d, type, g_black, ox);
        put_row(d + ox * bpp, type, g_out, ow);
        if (dw - ox - ow > 0) put_row(d + (ox + ow) * bpp, type, g_black, dw - ox - ow);
    }
}

/* 1:1 copy at (x,y), clipped to the destination height the caller gave. */
static void plain_blit(const BYTE *src, int sw, int sh, BYTE *dst, int pitch, int dh, int dx, int dy, int type)
{
    int bpp = type_bpp(type), y;
    if (!bpp) return;
    for (y = 0; y < sh && dy + y < dh; y++)
        put_row(dst + (dy + y) * pitch + dx * bpp, type, (const DWORD *)(src + y * sw * 4), sw);
}

/* ---- the intro call site ------------------------------------------------------
 * NextBinkFrame (0x4637f0) calls BinkCopyToBuffer from 0x4638af with the back
 * buffer locked; its D3DSURFACE_DESC is at [ebp-0x28] of that frame: Format
 * at -0x28, Width at -0x10, Height at -0x0c.  Only trusted when our return
 * address is exactly that site and the values are sane.                     */
#define INTRO_COPY_RET 0x4638b5

/* Not a Bink flag.  cutscenes/binkproxy/binktest.c sets it to exercise the
 * fit-to-back-buffer path outside the game: destination = pitch/bpp x destH. */
#define TEST_FIT 0x00100000u

static int intro_backbuffer(void *ret, BYTE *callerEbp, DWORD pitch, int *w, int *h)
{
    DWORD fmt, bw, bh;
    if ((ULONG_PTR)ret != INTRO_COPY_RET || !callerEbp) return 0;
    fmt = *(DWORD *)(callerEbp - 0x28);
    bw  = *(DWORD *)(callerEbp - 0x10);
    bh  = *(DWORD *)(callerEbp - 0x0c);
    if (fmt < 21 || fmt > 24) return 0;
    if (bw < 320 || bh < 200 || bw > MAX_W || bh > 8192) return 0;
    if (pitch < bw * (fmt <= 22 ? 4 : 2)) return 0;
    *w = (int)bw; *h = (int)bh;
    return 1;
}

static BYTE *g_scratch;
static DWORD g_scratchSize;

/* ---- exports ------------------------------------------------------------------- */
EXPORT void *WINAPI BinkOpen(const char *name, DWORD flags)
{
    Movie *mv;
    char m[300];
    if (g_replace && name && (mv = open_replacement(name))) return mv->bink;
    if (!real_load()) return NULLPTR;
    m[0] = 0; s_cat(m, "bink "); if (name && s_len(name) < 200) s_cat(m, name);
    logline(m);
    return R.Open(name, flags);
}

EXPORT int WINAPI BinkDoFrame(void *bink)
{
    Movie *mv = ours(bink);
    if (mv) { movie_decode(mv); return 0; }
    return R.DoFrame(bink);
}

EXPORT void WINAPI BinkNextFrame(void *bink)
{
    Movie *mv = ours(bink);
    if (mv) { if (!mv->eos) mv->bink[3]++; return; }
    R.NextFrame(bink);
}

EXPORT int WINAPI BinkWait(void *bink)
{
    Movie *mv = ours(bink);
    if (mv) {
        double due, left;
        if (!mv->started || mv->eos) return 0;
        due = mv->t0 + (mv->bink[3] - 1) / mv->fps;
        left = due - now_s();
        if (left <= 0) return 0;
        if (left > 0.003) Sleep(1);   /* PlayMovie spins on this; let the decoder breathe */
        return 1;
    }
    return R.Wait(bink);
}

EXPORT void WINAPI BinkClose(void *bink)
{
    Movie *mv = ours(bink);
    if (mv) { movie_close(mv); return; }
    if (R.dll) R.Close(bink);
}

EXPORT void WINAPI BinkGoto(void *bink, DWORD frame, DWORD flags)
{
    if (ours(bink)) return;   /* only the single-player screen seeks, and never ours */
    R.Goto(bink, frame, flags);
}

EXPORT void WINAPI BinkSetVolume(void *bink, int vol)
{
    Movie *mv = ours(bink);
    if (mv) {
        mv->volume = vol;
        if (mv->wo) {
            DWORD v = (DWORD)vol * 0xFFFF / 32768;
            if (v > 0xFFFF) v = 0xFFFF;
            waveOutSetVolume(mv->wo, v | (v << 16));
        }
        return;
    }
    R.SetVolume(bink, vol);
}

EXPORT void *WINAPI BinkOpenDirectSound(ULONG_PTR param)
{
    if (!real_load()) return NULLPTR;
    return R.OpenDirectSound(param);
}

EXPORT int WINAPI BinkSetSoundSystem(void *open, void *param)
{
    if (!real_load()) return 0;
    if (open == (void *)BinkOpenDirectSound) open = (void *)R.OpenDirectSound;
    return R.SetSoundSystem(open, param);
}

/* Built without frame-pointer omission (build.sh), so the saved EBP at our
 * own frame is the caller's. */
EXPORT int WINAPI BinkCopyToBuffer(void *bink, void *dest, DWORD pitch, DWORD destH,
                                   DWORD x, DWORD y, DWORD flags)
{
    Movie *mv = ours(bink);
    int type = (int)(flags & 15), bw, bh, full;
    BYTE *callerEbp = *(BYTE **)__builtin_frame_address(0);

    full = g_fit && intro_backbuffer(__builtin_return_address(0), callerEbp, pitch, &bw, &bh);
    if (!full && (flags & TEST_FIT) && type_bpp(type)) {   /* binktest.exe only */
        full = 1; bw = (int)(pitch / (DWORD)type_bpp(type)); bh = (int)destH;
    }
    flags &= ~TEST_FIT;
    if (!mv && !full) return R.CopyToBuffer(bink, dest, pitch, destH, x, y, flags);
    if (!dest || !type_bpp(type)) return 1;

    if (full) {
        /* A failed lock leaves the game passing us whatever was on its stack.
         * First and last row are enough to tell; probing every page is not free. */
        if (IsBadWritePtr(dest, pitch) || IsBadWritePtr((BYTE *)dest + pitch * (DWORD)(bh - 1), pitch)) {
            logline("back buffer not writable; frame skipped");
            return 1;
        }
        if (mv) {
            fit_blit(mv->frame, mv->w, mv->h, dest, (int)pitch, bw, bh, type);
        } else {
            DWORD sw = ((DWORD *)bink)[0], sh = ((DWORD *)bink)[1], need = sw * sh * 4;
            if (sw < 2 || sh < 2 || sw > MAX_W || sh > 8192) return R.CopyToBuffer(bink, dest, pitch, destH, x, y, flags);
            if (need > g_scratchSize) {
                vfree(g_scratch);
                g_scratch = valloc(need);
                g_scratchSize = g_scratch ? need : 0;
                if (!g_scratch) return R.CopyToBuffer(bink, dest, pitch, destH, x, y, flags);
            }
            if (R.CopyToBuffer(bink, g_scratch, sw * 4, sh, 0, 0, (flags & ~15u) | 3) != 0) return 1;
            fit_blit(g_scratch, (int)sw, (int)sh, dest, (int)pitch, bw, bh, type);
        }
        return 0;
    }
    plain_blit(mv->frame, mv->w, mv->h, dest, (int)pitch, (int)destH, (int)x, (int)y, type);
    return 0;
}

/* ---- load-time: config, and the intro display mode --------------------------- */
typedef struct { int len, at; BYTE sig[20]; } ModeSite;

/* Both sites are in Program::PlayIntroMovie and each signature occurs exactly
 * once in Armada2.exe (the bare 640/480 push pair occurs a third time
 * elsewhere, which this must not touch).  `at` is the offset of the push-bpp. */
static const ModeSite k_sites[2] = {
    /* push 32; push 480; push 640; mov edi,[eax+0x104] */
    { 18, 0, { 0x6A, 0x20, 0x68, 0xE0, 0x01, 0x00, 0x00, 0x68, 0x80, 0x02, 0x00, 0x00,
               0x8B, 0xB8, 0x04, 0x01, 0x00, 0x00 } },
    /* mov ecx,[edi+0x1c]; push 16; push 480; push 640; call */
    { 16, 3, { 0x8B, 0x4F, 0x1C, 0x6A, 0x10, 0x68, 0xE0, 0x01, 0x00, 0x00,
               0x68, 0x80, 0x02, 0x00, 0x00, 0xE8 } },
};

static int patch_intro_mode(int w, int h)
{
    BYTE *base = (BYTE *)GetModuleHandleA(NULLPTR);
    DWORD pe, va, vs, old;
    BYTE *nt, *sec;
    int nsec, optsz, i, n, hits = 0;
    if (!base) return 0;
    pe = *(DWORD *)(base + 0x3C);
    nt = base + pe;
    nsec = *(WORD *)(nt + 6);
    optsz = *(WORD *)(nt + 20);
    sec = nt + 24 + optsz;
    for (i = 0; i < nsec; i++, sec += 40) {
        BYTE *p, *end;
        if (sec[0] != '.' || sec[1] != 't' || sec[2] != 'e' || sec[3] != 'x') continue;
        va = *(DWORD *)(sec + 12);
        vs = *(DWORD *)(sec + 8);
        end = base + va + vs;
        for (n = 0; n < 2; n++) {
            const ModeSite *m = &k_sites[n];
            for (p = base + va; p + m->len <= end; p++) {
                int k;
                for (k = 0; k < m->len && p[k] == m->sig[k]; k++) ;
                if (k != m->len) continue;
                {
                    BYTE *q = p + m->at;        /* push bpp (2 bytes), push h, push w */
                    if (!VirtualProtect(q, 12, PAGE_EXECUTE_READWRITE, &old)) continue;
                    *(DWORD *)(q + 3) = (DWORD)h;
                    *(DWORD *)(q + 8) = (DWORD)w;
                    VirtualProtect(q, 12, old, &old);
                    FlushInstructionCache(GetCurrentProcess(), q, 12);
                    hits++;
                    break;
                }
            }
        }
    }
    return hits;
}

BOOL WINAPI DllMain(HANDLE inst, DWORD reason, void *res)
{
    char m[200];
    int i, w, h;
    (void)inst; (void)res;
    if (reason != 1) return 1;

    GetModuleFileNameA(NULLPTR, g_dir, sizeof g_dir);
    for (i = s_len(g_dir); i > 0 && g_dir[i - 1] != '\\' && g_dir[i - 1] != '/'; i--) ;
    g_dir[i] = 0;
    s_cpy(g_ini, g_dir); s_cat(g_ini, "BinkProxy.ini");

    g_log     = (int)GetPrivateProfileIntA("BinkProxy", "Log",            1, g_ini);
    g_replace = (int)GetPrivateProfileIntA("BinkProxy", "ReplaceMovies",  1, g_ini);
    g_raise   = (int)GetPrivateProfileIntA("BinkProxy", "RaiseIntroMode", 1, g_ini);
    g_fit     = (int)GetPrivateProfileIntA("BinkProxy", "FitIntro",       1, g_ini);
    w         = (int)GetPrivateProfileIntA("BinkProxy", "IntroWidth",     0, g_ini);
    h         = (int)GetPrivateProfileIntA("BinkProxy", "IntroHeight",    0, g_ini);

    /* At DllMain time no mode change has happened yet, so this is the desktop
     * (the same observation Menus.asi relies on). */
    if (w < 640) w = GetSystemMetrics(SM_CXSCREEN);
    if (h < 480) h = GetSystemMetrics(SM_CYSCREEN);

    for (i = 0; i < MAX_W; i++) g_black[i] = 0xFF000000;

    m[0] = 0;
    s_cat(m, "--- BinkProxy loaded: replace="); s_num(m, g_replace);
    s_cat(m, " fit="); s_num(m, g_fit);
    if (g_raise && w >= 640 && h >= 480) {
        int hits = patch_intro_mode(w, h);
        s_cat(m, " intro mode -> "); s_num(m, w); s_cat(m, "x"); s_num(m, h);
        s_cat(m, " ("); s_num(m, hits); s_cat(m, "/2 sites)");
    } else {
        s_cat(m, " intro mode left at 640x480");
    }
    logline(m);
    return 1;
}
