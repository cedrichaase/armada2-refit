/*
 * binktest.exe -- drive binkw32.dll the way Program::PlayMovie does, outside
 * the game.  Not installed; a test harness for build.sh's output.
 *
 *   binktest.exe <movie.bik> [maxFrames] [fitW fitH]
 *
 * Loads binkw32.dll from its own directory, plays up to maxFrames (default
 * 150) in real time -- BinkWait, BinkDoFrame, BinkCopyToBuffer, BinkNextFrame
 * -- and writes frame 1, every 150th and the last as binktest_NNNN.bmp.
 * With fitW/fitH it copies into a buffer of that size through the proxy's
 * fit-to-back-buffer path (the one the intro uses in game).  A summary goes
 * to binktest.txt; the proxy's own account goes to BinkProxy.log.
 */
typedef unsigned char BYTE;
typedef unsigned short WORD;
typedef unsigned long DWORD;
typedef int BOOL;
typedef void *HANDLE;
typedef void *HMODULE;
#define WINAPI __stdcall
#define NULLPTR ((void *)0)

__declspec(dllimport) HMODULE WINAPI LoadLibraryA(const char *);
__declspec(dllimport) void *WINAPI GetProcAddress(HMODULE, const char *);
__declspec(dllimport) HANDLE WINAPI CreateFileA(const char *, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL WINAPI WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE);
__declspec(dllimport) void *WINAPI VirtualAlloc(void *, DWORD, DWORD, DWORD);
__declspec(dllimport) char *WINAPI GetCommandLineA(void);
__declspec(dllimport) void WINAPI ExitProcess(DWORD);
__declspec(dllimport) void WINAPI Sleep(DWORD);

int _fltused = 0;
void *memset(void *d, int c, unsigned n) { BYTE *p = d; while (n--) *p++ = (BYTE)c; return d; }

typedef void *(WINAPI *pOpen)(const char *, DWORD);
typedef int (WINAPI *pOne)(void *);
typedef void (WINAPI *pVoid)(void *);
typedef int (WINAPI *pCopy)(void *, void *, DWORD, DWORD, DWORD, DWORD, DWORD);

static char out[4096];
static int  outn;
static void put(const char *s) { while (*s && outn < 4000) out[outn++] = *s++; }
static void num(long v)
{
    char b[16]; int i = 0; unsigned long u;
    if (v < 0) { put("-"); u = (unsigned long)(-v); } else u = (unsigned long)v;
    do { b[i++] = (char)('0' + u % 10); u /= 10; } while (u);
    while (i) { char c[2] = { b[--i], 0 }; put(c); }
}
static void flush(void)
{
    HANDLE h = CreateFileA("binktest.txt", 0x40000000, 0, NULLPTR, 2, 0x80, NULLPTR);
    DWORD w;
    if (h != (HANDLE)-1) { WriteFile(h, out, (DWORD)outn, &w, NULLPTR); CloseHandle(h); }
}

static void write_bmp(int n, const BYTE *px, int w, int h)
{
    char name[32] = "binktest_0000.bmp";
    BYTE hd[54];
    DWORD sz = (DWORD)(w * h * 4), wr;
    HANDLE f;
    int i;
    name[9]  = (char)('0' + n / 1000 % 10); name[10] = (char)('0' + n / 100 % 10);
    name[11] = (char)('0' + n / 10 % 10);   name[12] = (char)('0' + n % 10);
    memset(hd, 0, sizeof hd);
    hd[0] = 'B'; hd[1] = 'M';
    *(DWORD *)(hd + 2) = 54 + sz;
    *(DWORD *)(hd + 10) = 54;
    *(DWORD *)(hd + 14) = 40;
    *(long *)(hd + 18) = w;
    *(long *)(hd + 22) = -h;               /* top-down */
    *(WORD *)(hd + 26) = 1;
    *(WORD *)(hd + 28) = 32;
    *(DWORD *)(hd + 34) = sz;
    f = CreateFileA(name, 0x40000000, 0, NULLPTR, 2, 0x80, NULLPTR);
    if (f == (HANDLE)-1) return;
    WriteFile(f, hd, 54, &wr, NULLPTR);
    for (i = 0; i < h; i++) WriteFile(f, px + i * w * 4, (DWORD)(w * 4), &wr, NULLPTR);
    CloseHandle(f);
}

static char *next_arg(char **p)
{
    char *s = *p, *start;
    while (*s == ' ') s++;
    if (!*s) { *p = s; return NULLPTR; }
    if (*s == '"') { start = ++s; while (*s && *s != '"') s++; }
    else { start = s; while (*s && *s != ' ') s++; }
    if (*s) *s++ = 0;
    *p = s;
    return start;
}
static long to_long(const char *s) { long v = 0; while (s && *s >= '0' && *s <= '9') v = v * 10 + (*s++ - '0'); return v; }

void WINAPI start(void)
{
    char *cl = GetCommandLineA(), *movie, *a;
    long maxf = 150, fw = 0, fh = 0;
    HMODULE dll;
    pOpen Open; pOne DoFrame, Wait; pVoid NextFrame, Close; pCopy Copy;
    DWORD *b;
    int dw, dh, n, waits = 0, fit;
    BYTE *buf;

    next_arg(&cl);                       /* program name */
    movie = next_arg(&cl);
    if ((a = next_arg(&cl))) maxf = to_long(a);
    if ((a = next_arg(&cl))) fw = to_long(a);
    if ((a = next_arg(&cl))) fh = to_long(a);
    if (!movie) { put("usage: binktest movie.bik [maxFrames] [fitW fitH]\r\n"); flush(); ExitProcess(1); }

    dll = LoadLibraryA("binkw32.dll");
    if (!dll) { put("cannot load binkw32.dll\r\n"); flush(); ExitProcess(2); }
    Open = (pOpen)GetProcAddress(dll, "_BinkOpen@8");
    DoFrame = (pOne)GetProcAddress(dll, "_BinkDoFrame@4");
    Wait = (pOne)GetProcAddress(dll, "_BinkWait@4");
    NextFrame = (pVoid)GetProcAddress(dll, "_BinkNextFrame@4");
    Close = (pVoid)GetProcAddress(dll, "_BinkClose@4");
    Copy = (pCopy)GetProcAddress(dll, "_BinkCopyToBuffer@28");
    if (!Open || !DoFrame || !Wait || !NextFrame || !Close || !Copy) { put("missing export\r\n"); flush(); ExitProcess(3); }

    b = Open(movie, 0);
    if (!b) { put("BinkOpen failed\r\n"); flush(); ExitProcess(4); }
    put("open: "); num((long)b[0]); put("x"); num((long)b[1]); put(", frames "); num((long)b[2]); put("\r\n");

    fit = fw > 0 && fh > 0;
    dw = fit ? (int)fw : (int)b[0];
    dh = fit ? (int)fh : (int)b[1];
    buf = VirtualAlloc(NULLPTR, (DWORD)(dw * dh * 4), 0x3000, 4);
    if (!buf) { put("alloc failed\r\n"); flush(); ExitProcess(5); }

    for (n = 1; n <= maxf; n++) {
        int r;
        while (Wait(b)) waits++;
        DoFrame(b);
        r = Copy(b, buf, (DWORD)(dw * 4), (DWORD)dh, 0, 0, 0x80000000u | 3 | (fit ? 0x00100000u : 0));
        if (r) { put("copy returned "); num(r); put(" at frame "); num(n); put("\r\n"); }
        /* Sparse on purpose: a 3440x1440 BMP is 19 MB, and writing one every
         * second made the proxy's own late-frame count measure the harness. */
        if (n == 1 || n % 150 == 0) write_bmp(n, buf, dw, dh);
        if (b[3] == b[2] - 1) { put("end of movie at frame "); num(n); put("\r\n"); break; }
        NextFrame(b);
    }
    write_bmp(9999, buf, dw, dh);
    put("played "); num(n > maxf ? maxf : n); put(" frames, FrameNum "); num((long)b[3]);
    put(", waits "); num(waits); put("\r\n");
    Close(b);
    flush();
    ExitProcess(0);
}
