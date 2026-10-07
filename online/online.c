/*
 * Online.asi -- online multiplayer for Star Trek: Armada II.
 *
 * It turns the Multiplayer Connection screen's IPX button into Internet -
 * Online ("the menu entry", below).  A game started there gets our own
 * IDirectPlay8Peer over UDP (peer.c) instead of DirectPlay's.  Every peer,
 * ours or DirectPlay's, is wrapped in a pass-through that logs every call the
 * game makes, every message handed back, and the addresses that cross the
 * interface; for DirectPlay's that log was the specification of what ours had
 * to do.  The pass-through changes nothing.
 *
 * WHERE
 * -----
 * Armada2.exe never creates DirectPlay itself.  NetworkManager.dll does, and
 * it is the only module in the process that imports ole32!CoCreateInstance
 * (Armada2.exe imports only CoCreateGuid from ole32).  So the plugin replaces
 * that one import-table slot in NetworkManager.dll.  Every CoCreateInstance
 * the DLL makes is logged by CLSID; an IDirectPlay8Peer is wrapped.
 *
 * THE WRAPPER
 * -----------
 * A COM object is a pointer to a vtable of __stdcall functions taking `this`
 * first.  The proxy is an object of our own whose vtable has the same 37
 * slots, in the order of dplay8.h; each slot logs and calls the same slot of
 * the real object.  Every argument of every IDirectPlay8Peer method is 32
 * bits wide (pointers, DPNID, DPNHANDLE, DWORD), so a generic slot is "N
 * DWORDs in, HRESULT out", and only the calls worth decoding are written out.
 *
 * Initialize() carries the game's message handler.  The proxy passes its own
 * handler to the real object instead, with the proxy as the user context, and
 * forwards each message to the game's handler with the game's context, so
 * every DPN_MSGID_* is seen on its way in.
 *
 * DirectPlay calls the handler from its own worker threads, so the log is
 * serialised by a critical section.
 */

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

#define NULLPTR ((void *)0)
#define TRUE  1
#define FALSE 0

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define CREATE_ALWAYS          2
#define FILE_ATTRIBUTE_NORMAL  0x80
#define INVALID_HANDLE_VALUE   ((HANDLE)(LONG)-1)
#define PAGE_READWRITE         0x04
#define PAGE_EXECUTE_READWRITE 0x40
#define HEAP_ZERO_MEMORY       0x08

typedef struct { BYTE opaque[24]; } CRITICAL_SECTION;

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) void *  __stdcall GetProcAddress(HMODULE, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetPrivateProfileStringA(LPCSTR, LPCSTR, LPCSTR, LPSTR, DWORD, LPCSTR);
__declspec(dllimport) BOOL    __stdcall IsWindow(HANDLE);
__declspec(dllimport) DWORD   __stdcall GetTickCount(void);
__declspec(dllimport) DWORD   __stdcall GetCurrentThreadId(void);
__declspec(dllimport) void    __stdcall InitializeCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) void    __stdcall EnterCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) void    __stdcall LeaveCriticalSection(CRITICAL_SECTION *);
__declspec(dllimport) HANDLE  __stdcall GetProcessHeap(void);
__declspec(dllimport) void   *__stdcall HeapAlloc(HANDLE, DWORD, UINT);

/* ---- COM and DirectPlay 8, as much as is needed ------------------------ */

typedef struct { DWORD d1; WORD d2, d3; BYTE d4[8]; } GUID;

static const GUID CLSID_DirectPlay8Peer =
    { 0x286f484d, 0x375e, 0x4458, { 0xa2, 0x72, 0xb1, 0x38, 0xe2, 0xf8, 0x0a, 0x6a } };
static const GUID IID_IDirectPlay8Peer =
    { 0x5102dacf, 0x241b, 0x11d3, { 0xae, 0xa7, 0x00, 0x60, 0x97, 0xb0, 0x14, 0x11 } };

typedef HRESULT (__stdcall *CoCreateInstance_t)(const GUID *, void *, DWORD,
                                               const GUID *, void **);
typedef HRESULT (__stdcall *MsgHandler_t)(void *ctx, DWORD type, void *msg);

#define DPN_MSGID_OFFSET            0xFFFF0000
#define DPN_MSGID_CONNECT_COMPLETE  0xFFFF0005
#define DPN_MSGID_CREATE_PLAYER     0xFFFF0007
#define DPN_MSGID_DESTROY_PLAYER    0xFFFF0009
#define DPN_MSGID_ENUM_HOSTS_QUERY  0xFFFF000A
#define DPN_MSGID_ENUM_HOSTS_RESPONSE 0xFFFF000B
#define DPN_MSGID_INDICATE_CONNECT  0xFFFF000E
#define DPN_MSGID_RECEIVE           0xFFFF0011

/* Slot numbers in IDirectPlay8Peer's vtable (dplay8.h order). */
enum {
    S_QueryInterface, S_AddRef, S_Release, S_Initialize, S_EnumServiceProviders,
    S_CancelAsyncOperation, S_Connect, S_SendTo, S_GetSendQueueInfo, S_Host,
    S_GetApplicationDesc, S_SetApplicationDesc, S_CreateGroup, S_DestroyGroup,
    S_AddPlayerToGroup, S_RemovePlayerFromGroup, S_SetGroupInfo, S_GetGroupInfo,
    S_EnumPlayersAndGroups, S_EnumGroupMembers, S_SetPeerInfo, S_GetPeerInfo,
    S_GetPeerAddress, S_GetLocalHostAddresses, S_Close, S_EnumHosts, S_DestroyPeer,
    S_ReturnBuffer, S_GetPlayerContext, S_GetGroupContext, S_GetCaps, S_SetCaps,
    S_SetSPCaps, S_GetSPCaps, S_GetConnectionInfo, S_RegisterLobby,
    S_TerminateSession, S_COUNT
};

/* IDirectPlay8Address slot 10: GetURLA(this, char *url, DWORD *chars). */
#define ADDR_GETURLA 10
typedef HRESULT (__stdcall *GetURLA_t)(void *self, char *url, DWORD *chars);

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char             g_logpath[320];
static int              g_logging = 1;
static int              g_payload = 16;     /* payload bytes shown per packet */
static HANDLE           g_log = INVALID_HANDLE_VALUE;
static CRITICAL_SECTION g_cs;
static DWORD            g_t0;

static int s_len(const char *s) { int n = 0; while (s[n]) n++; return n; }

static void s_cat(char *d, const char *s)
{
    int n = s_len(d);
    while (*s) d[n++] = *s++;
    d[n] = 0;
}

static void s_num(char *d, long v)
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

static void s_hexw(char *d, DWORD v, int digits)
{
    static const char x[] = "0123456789abcdef";
    int i, k = s_len(d);
    for (i = (digits - 1) * 4; i >= 0; i -= 4) d[k++] = x[(v >> i) & 15];
    d[k] = 0;
}

static void s_hex(char *d, DWORD v) { s_cat(d, "0x"); s_hexw(d, v, 8); }

static void s_bytes(char *d, const BYTE *p, DWORD n, int max)
{
    DWORD i;
    if (!p) { s_cat(d, "-"); return; }
    for (i = 0; i < n && (int)i < max; i++) {
        if (i) s_cat(d, " ");
        s_hexw(d, p[i], 2);
    }
    if ((int)n > max) s_cat(d, " ..");
}

static void s_guid(char *d, const GUID *g)
{
    int i;
    if (!g) { s_cat(d, "null"); return; }
    s_hexw(d, g->d1, 8); s_cat(d, "-");
    s_hexw(d, g->d2, 4); s_cat(d, "-");
    s_hexw(d, g->d3, 4); s_cat(d, "-");
    for (i = 0; i < 8; i++) { if (i == 2) s_cat(d, "-"); s_hexw(d, g->d4[i], 2); }
}

/* Wide string to the log, as ASCII; anything else becomes '?'. */
static void s_wide(char *d, const WCHAR *w, int max)
{
    int k = s_len(d), i;
    if (!w) { s_cat(d, "null"); return; }
    d[k++] = '"';
    for (i = 0; w[i] && i < max; i++)
        d[k++] = (w[i] >= 32 && w[i] < 127) ? (char)w[i] : '?';
    d[k++] = '"';
    d[k] = 0;
}

static int guid_eq(const GUID *a, const GUID *b)
{
    const BYTE *x = (const BYTE *)a, *y = (const BYTE *)b;
    int i;
    if (!a || !b) return 0;
    for (i = 0; i < 16; i++) if (x[i] != y[i]) return 0;
    return 1;
}

/* One line: "<ms since start> <thread> <text>".  The file stays open; lines
 * arrive from the game thread and from DirectPlay's workers. */
static void logline(const char *s)
{
    char  buf[1024];
    DWORD wrote;

    if (!g_logging || g_log == INVALID_HANDLE_VALUE) return;
    buf[0] = 0;
    s_num(buf, (long)(GetTickCount() - g_t0));
    s_cat(buf, " t");
    s_num(buf, (long)GetCurrentThreadId());
    s_cat(buf, " ");
    if (s_len(s) > 960) {
        int n = s_len(buf), i;
        for (i = 0; i < 960; i++) buf[n + i] = s[i];
        buf[n + 960] = 0;
    } else {
        s_cat(buf, s);
    }
    s_cat(buf, "\r\n");
    EnterCriticalSection(&g_cs);
    WriteFile(g_log, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    LeaveCriticalSection(&g_cs);
}

/* The URL of an IDirectPlay8Address, e.g.
 * "x-directplay:/provider=%7BEBFE7BA0-...%7D;hostname=10.0.0.19;port=2302". */
static void s_addr(char *d, void *addr)
{
    char  url[400];
    DWORD n = sizeof(url) - 1;
    if (!addr) { s_cat(d, "null"); return; }
    url[0] = 0;
    if (((GetURLA_t)((*(void ***)addr)[ADDR_GETURLA]))(addr, url, &n) < 0) {
        s_cat(d, "<GetURLA failed>");
        return;
    }
    url[sizeof(url) - 1] = 0;
    s_cat(d, url);
}

static void s_appdesc(char *d, const BYTE *a)
{
    if (!a) { s_cat(d, "null"); return; }
    s_cat(d, "{flags ");     s_hex(d, *(const DWORD *)(a + 4));
    s_cat(d, " instance ");  s_guid(d, (const GUID *)(a + 8));
    s_cat(d, " app ");       s_guid(d, (const GUID *)(a + 24));
    s_cat(d, " max ");       s_num(d, (long)*(const DWORD *)(a + 40));
    s_cat(d, " cur ");       s_num(d, (long)*(const DWORD *)(a + 44));
    s_cat(d, " name ");      s_wide(d, *(const WCHAR *const *)(a + 48), 64);
    s_cat(d, *(const WCHAR *const *)(a + 52) ? " password yes" : " password no");
    s_cat(d, " appdata ");   s_num(d, (long)*(const DWORD *)(a + 68));
    s_cat(d, " ");
    s_bytes(d, *(const BYTE *const *)(a + 64), *(const DWORD *)(a + 68), 32);
    s_cat(d, "}");
}

static const char *msg_name(DWORD t)
{
    static const char *const n[] = {
        "?", "ADD_PLAYER_TO_GROUP", "APPLICATION_DESC", "ASYNC_OP_COMPLETE",
        "CLIENT_INFO", "CONNECT_COMPLETE", "CREATE_GROUP", "CREATE_PLAYER",
        "DESTROY_GROUP", "DESTROY_PLAYER", "ENUM_HOSTS_QUERY",
        "ENUM_HOSTS_RESPONSE", "GROUP_INFO", "HOST_MIGRATE", "INDICATE_CONNECT",
        "INDICATED_CONNECT_ABORTED", "PEER_INFO", "RECEIVE",
        "REMOVE_PLAYER_FROM_GROUP", "RETURN_BUFFER", "SEND_COMPLETE",
        "SERVER_INFO", "TERMINATE_SESSION", "CREATE_THREAD", "DESTROY_THREAD",
    };
    if ((t & 0xFFFF0000) == DPN_MSGID_OFFSET && (t & 0xFFFF) < sizeof(n) / sizeof(n[0]))
        return n[t & 0xFFFF];
    return "?";
}

static const char *slot_name(int i)
{
    static const char *const n[S_COUNT] = {
        "QueryInterface", "AddRef", "Release", "Initialize", "EnumServiceProviders",
        "CancelAsyncOperation", "Connect", "SendTo", "GetSendQueueInfo", "Host",
        "GetApplicationDesc", "SetApplicationDesc", "CreateGroup", "DestroyGroup",
        "AddPlayerToGroup", "RemovePlayerFromGroup", "SetGroupInfo", "GetGroupInfo",
        "EnumPlayersAndGroups", "EnumGroupMembers", "SetPeerInfo", "GetPeerInfo",
        "GetPeerAddress", "GetLocalHostAddresses", "Close", "EnumHosts", "DestroyPeer",
        "ReturnBuffer", "GetPlayerContext", "GetGroupContext", "GetCaps", "SetCaps",
        "SetSPCaps", "GetSPCaps", "GetConnectionInfo", "RegisterLobby",
        "TerminateSession",
    };
    return (i >= 0 && i < S_COUNT) ? n[i] : "?";
}

/* ---- the proxy -------------------------------------------------------- */

typedef struct {
    void       **vtbl;      /* must be first: this is what the game calls */
    void        *real;      /* the real IDirectPlay8Peer */
    void       **rv;        /* its vtable */
    MsgHandler_t game_handler;
    void        *game_ctx;
    int          id;        /* proxy number, for the log */
} Proxy;

static int g_nproxies;

static void log_call(Proxy *p, int slot, const DWORD *a, int n, HRESULT hr)
{
    char m[400];
    int  i;
    m[0] = 0;
    s_cat(m, "peer");     s_num(m, p->id);
    s_cat(m, " ");        s_cat(m, slot_name(slot));
    s_cat(m, "(");
    for (i = 0; i < n; i++) { if (i) s_cat(m, ","); s_hex(m, a[i]); }
    s_cat(m, ") -> ");    s_hex(m, (DWORD)hr);
    logline(m);
}

/* Generic slots: N DWORD arguments after `this`, forwarded unchanged. */
#define PARAMS_1 , DWORD a1
#define PARAMS_2 PARAMS_1, DWORD a2
#define PARAMS_3 PARAMS_2, DWORD a3
#define PARAMS_4 PARAMS_3, DWORD a4
#define PARAMS_5 PARAMS_4, DWORD a5
#define PARAMS_6 PARAMS_5, DWORD a6
#define PARAMS_7 PARAMS_6, DWORD a7
#define TYPES_1 , DWORD
#define TYPES_2 TYPES_1, DWORD
#define TYPES_3 TYPES_2, DWORD
#define TYPES_4 TYPES_3, DWORD
#define TYPES_5 TYPES_4, DWORD
#define TYPES_6 TYPES_5, DWORD
#define TYPES_7 TYPES_6, DWORD
#define ARGS_1 , a1
#define ARGS_2 ARGS_1, a2
#define ARGS_3 ARGS_2, a3
#define ARGS_4 ARGS_3, a4
#define ARGS_5 ARGS_4, a5
#define ARGS_6 ARGS_5, a6
#define ARGS_7 ARGS_6, a7

#define GENERIC(slot, N)                                                       \
    static HRESULT __stdcall P_##slot(Proxy *p PARAMS_##N)                     \
    {                                                                          \
        DWORD   v[] = { 0 ARGS_##N };                                          \
        HRESULT hr = ((HRESULT (__stdcall *)(void * TYPES_##N))                \
                      p->rv[S_##slot])(p->real ARGS_##N);                      \
        log_call(p, S_##slot, v + 1, N, hr);                                   \
        return hr;                                                             \
    }

GENERIC(EnumServiceProviders, 6)
GENERIC(CancelAsyncOperation, 2)
GENERIC(GetSendQueueInfo, 4)
GENERIC(GetApplicationDesc, 3)
GENERIC(CreateGroup, 5)
GENERIC(DestroyGroup, 4)
GENERIC(AddPlayerToGroup, 5)
GENERIC(RemovePlayerFromGroup, 5)
GENERIC(SetGroupInfo, 5)
GENERIC(GetGroupInfo, 4)
GENERIC(EnumPlayersAndGroups, 3)
GENERIC(EnumGroupMembers, 4)
GENERIC(GetPeerInfo, 4)
GENERIC(Close, 1)
GENERIC(DestroyPeer, 4)
GENERIC(ReturnBuffer, 2)
GENERIC(GetPlayerContext, 3)
GENERIC(GetGroupContext, 3)
GENERIC(GetCaps, 2)
GENERIC(SetCaps, 2)
GENERIC(SetSPCaps, 3)
GENERIC(GetSPCaps, 3)
GENERIC(GetConnectionInfo, 3)
GENERIC(RegisterLobby, 3)
GENERIC(TerminateSession, 3)

typedef HRESULT (__stdcall *F1)(void *, DWORD);
typedef HRESULT (__stdcall *F2)(void *, DWORD, DWORD);
typedef HRESULT (__stdcall *F3)(void *, DWORD, DWORD, DWORD);
typedef HRESULT (__stdcall *F4)(void *, DWORD, DWORD, DWORD, DWORD);
typedef HRESULT (__stdcall *F7)(void *, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD);
typedef HRESULT (__stdcall *F11)(void *, DWORD, DWORD, DWORD, DWORD, DWORD, DWORD,
                                 DWORD, DWORD, DWORD, DWORD, DWORD);

static HRESULT __stdcall P_QueryInterface(Proxy *p, const GUID *iid, void **out)
{
    HRESULT hr = ((HRESULT (__stdcall *)(void *, const GUID *, void **))
                  p->rv[S_QueryInterface])(p->real, iid, out);
    char m[160];
    /* Hand back the proxy wherever the real object hands back itself, so a
     * QueryInterface cannot lead the game around the log. */
    if (hr >= 0 && out && *out == p->real) *out = p;
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " QueryInterface "); s_guid(m, iid);
    s_cat(m, " -> "); s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static ULONG __stdcall P_AddRef(Proxy *p)
{
    return ((ULONG (__stdcall *)(void *))p->rv[S_AddRef])(p->real);
}

static ULONG __stdcall P_Release(Proxy *p)
{
    ULONG n = ((ULONG (__stdcall *)(void *))p->rv[S_Release])(p->real);
    /* The proxy is never freed: a worker thread that is still delivering a
     * message as the game lets go would otherwise read freed memory.  It is
     * a few dozen bytes per session. */
    if (n == 0) {
        char m[64];
        m[0] = 0;
        s_cat(m, "peer"); s_num(m, p->id); s_cat(m, " released");
        logline(m);
    }
    return n;
}

/* Every message on its way from DirectPlay to the game.  Logged before the
 * game sees it, so whatever the game does in response follows it in the log;
 * the handler's result gets a line of its own only when it is not S_OK. */
static HRESULT __stdcall msg_tap(void *ctx, DWORD type, void *msg)
{
    Proxy      *p = (Proxy *)ctx;
    const BYTE *b = (const BYTE *)msg;
    HRESULT     hr;
    char        m[900];
    DWORD       i, size;

    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " msg ");  s_cat(m, msg_name(type));
    s_cat(m, " ");      s_hex(m, type);
    if (!b) { s_cat(m, " null"); goto out; }

    switch (type) {
    case DPN_MSGID_RECEIVE:
        s_cat(m, " from ");  s_hex(m, *(const DWORD *)(b + 4));
        s_cat(m, " size ");  s_num(m, (long)*(const DWORD *)(b + 16));
        s_cat(m, " buf ");   s_hex(m, *(const DWORD *)(b + 20));
        s_cat(m, " data ");
        s_bytes(m, *(const BYTE *const *)(b + 12), *(const DWORD *)(b + 16), g_payload);
        break;
    case DPN_MSGID_INDICATE_CONNECT:
        s_cat(m, " userdata ");  s_num(m, (long)*(const DWORD *)(b + 8));
        s_cat(m, " ");
        s_bytes(m, *(const BYTE *const *)(b + 4), *(const DWORD *)(b + 8), 32);
        s_cat(m, " player ");    s_addr(m, *(void *const *)(b + 28));
        s_cat(m, " device ");    s_addr(m, *(void *const *)(b + 32));
        break;
    case DPN_MSGID_ENUM_HOSTS_QUERY:
        s_cat(m, " sender ");    s_addr(m, *(void *const *)(b + 4));
        s_cat(m, " data ");      s_num(m, (long)*(const DWORD *)(b + 16));
        s_cat(m, " maxreply ");  s_num(m, (long)*(const DWORD *)(b + 20));
        break;
    case DPN_MSGID_ENUM_HOSTS_RESPONSE:
        s_cat(m, " sender ");    s_addr(m, *(void *const *)(b + 4));
        s_cat(m, " desc ");      s_appdesc(m, *(const BYTE *const *)(b + 12));
        s_cat(m, " data ");      s_num(m, (long)*(const DWORD *)(b + 20));
        s_cat(m, " rtt ");       s_num(m, (long)*(const DWORD *)(b + 28));
        break;
    default:
        /* Everything else: the struct as DWORDs, which is enough to read. */
        size = *(const DWORD *)b;
        s_cat(m, " [");
        for (i = 4; i < size && i < 64; i += 4) {
            if (i > 4) s_cat(m, " ");
            s_hex(m, *(const DWORD *)(b + i));
        }
        s_cat(m, "]");
        break;
    }
out:
    logline(m);

    hr = p->game_handler(p->game_ctx, type, msg);

    if (hr != 0 || (b && type == DPN_MSGID_ENUM_HOSTS_QUERY)) {
        m[0] = 0;
        s_cat(m, "peer"); s_num(m, p->id);
        s_cat(m, "   handled ");  s_cat(m, msg_name(type));
        if (type == DPN_MSGID_ENUM_HOSTS_QUERY) {
            s_cat(m, " reply ");  s_num(m, (long)*(const DWORD *)(b + 28));
            s_cat(m, " ");
            s_bytes(m, *(const BYTE *const *)(b + 24), *(const DWORD *)(b + 28), 32);
        }
        s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
        logline(m);
    }
    return hr;
}

static HRESULT __stdcall P_Initialize(Proxy *p, void *ctx, MsgHandler_t fn, DWORD flags)
{
    DWORD   v[3] = { (DWORD)ctx, (DWORD)fn, flags };
    HRESULT hr;
    p->game_handler = fn;
    p->game_ctx     = ctx;
    hr = ((HRESULT (__stdcall *)(void *, void *, MsgHandler_t, DWORD))
          p->rv[S_Initialize])(p->real, p, fn ? msg_tap : fn, flags);
    log_call(p, S_Initialize, v, 3, hr);
    return hr;
}

static HRESULT __stdcall P_Connect(Proxy *p, DWORD desc, DWORD host, DWORD dev,
                                   DWORD sec, DWORD cred, DWORD udata, DWORD usize,
                                   DWORD pctx, DWORD actx, DWORD handle, DWORD flags)
{
    HRESULT hr;
    char    m[900];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " Connect desc ");  s_appdesc(m, (const BYTE *)desc);
    s_cat(m, " host ");          s_addr(m, (void *)host);
    s_cat(m, " device ");        s_addr(m, (void *)dev);
    s_cat(m, " userdata ");      s_num(m, (long)usize);
    s_cat(m, " ");               s_bytes(m, (const BYTE *)udata, usize, 32);
    s_cat(m, " flags ");         s_hex(m, flags);
    hr = ((F11)p->rv[S_Connect])(p->real, desc, host, dev, sec, cred, udata, usize,
                                 pctx, actx, handle, flags);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static HRESULT __stdcall P_SendTo(Proxy *p, DWORD to, DWORD bufs, DWORD nbufs,
                                  DWORD timeout, DWORD actx, DWORD handle, DWORD flags)
{
    HRESULT hr;
    char    m[600];
    DWORD   i, total = 0;
    const DWORD *bd = (const DWORD *)bufs;   /* {dwBufferSize, pBufferData} pairs */

    for (i = 0; bd && i < nbufs; i++) total += bd[2 * i];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " SendTo ");    s_hex(m, to);
    s_cat(m, " size ");      s_num(m, (long)total);
    if (nbufs != 1) { s_cat(m, " in "); s_num(m, (long)nbufs); }
    s_cat(m, " timeout ");   s_num(m, (long)timeout);
    s_cat(m, " flags ");     s_hex(m, flags);
    s_cat(m, " data ");
    if (bd && nbufs) s_bytes(m, (const BYTE *)bd[1], bd[0], g_payload);
    hr = ((F7)p->rv[S_SendTo])(p->real, to, bufs, nbufs, timeout, actx, handle, flags);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static HRESULT __stdcall P_Host(Proxy *p, DWORD desc, DWORD devs, DWORD ndevs,
                                DWORD sec, DWORD cred, DWORD pctx, DWORD flags)
{
    HRESULT hr;
    char    m[1000];
    DWORD   i;
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " Host desc ");  s_appdesc(m, (const BYTE *)desc);
    for (i = 0; devs && i < ndevs && i < 4; i++) {
        s_cat(m, " device ");  s_addr(m, ((void *const *)devs)[i]);
    }
    s_cat(m, " flags ");  s_hex(m, flags);
    hr = ((F7)p->rv[S_Host])(p->real, desc, devs, ndevs, sec, cred, pctx, flags);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static HRESULT __stdcall P_SetApplicationDesc(Proxy *p, DWORD desc, DWORD flags)
{
    HRESULT hr = ((F2)p->rv[S_SetApplicationDesc])(p->real, desc, flags);
    char    m[600];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " SetApplicationDesc ");  s_appdesc(m, (const BYTE *)desc);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static HRESULT __stdcall P_SetPeerInfo(Proxy *p, DWORD info, DWORD actx,
                                       DWORD handle, DWORD flags)
{
    HRESULT     hr = ((F4)p->rv[S_SetPeerInfo])(p->real, info, actx, handle, flags);
    const BYTE *b = (const BYTE *)info;
    char        m[400];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " SetPeerInfo");
    if (b) {
        s_cat(m, " infoflags ");  s_hex(m, *(const DWORD *)(b + 4));
        s_cat(m, " name ");       s_wide(m, *(const WCHAR *const *)(b + 8), 64);
        s_cat(m, " data ");       s_num(m, (long)*(const DWORD *)(b + 16));
        s_cat(m, " playerflags "); s_hex(m, *(const DWORD *)(b + 20));
    }
    s_cat(m, " flags ");  s_hex(m, flags);
    s_cat(m, " -> ");     s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static HRESULT __stdcall P_GetPeerAddress(Proxy *p, DWORD id, DWORD out, DWORD flags)
{
    HRESULT hr = ((F3)p->rv[S_GetPeerAddress])(p->real, id, out, flags);
    char    m[600];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " GetPeerAddress ");  s_hex(m, id);
    s_cat(m, " -> ");              s_hex(m, (DWORD)hr);
    if (hr >= 0 && out) { s_cat(m, " "); s_addr(m, *(void **)out); }
    logline(m);
    return hr;
}

static HRESULT __stdcall P_GetLocalHostAddresses(Proxy *p, DWORD out, DWORD count,
                                                 DWORD flags)
{
    HRESULT hr = ((F3)p->rv[S_GetLocalHostAddresses])(p->real, out, count, flags);
    char    m[900];
    DWORD   i, n = count ? *(DWORD *)count : 0;
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " GetLocalHostAddresses count ");  s_num(m, (long)n);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    for (i = 0; hr >= 0 && out && i < n && i < 4; i++) {
        s_cat(m, " ");  s_addr(m, ((void **)out)[i]);
    }
    logline(m);
    return hr;
}

static HRESULT __stdcall P_EnumHosts(Proxy *p, DWORD desc, DWORD host, DWORD dev,
                                     DWORD edata, DWORD esize, DWORD count,
                                     DWORD retry, DWORD timeout, DWORD uctx,
                                     DWORD handle, DWORD flags)
{
    HRESULT hr;
    char    m[900];
    m[0] = 0;
    s_cat(m, "peer"); s_num(m, p->id);
    s_cat(m, " EnumHosts desc ");  s_appdesc(m, (const BYTE *)desc);
    s_cat(m, " host ");            s_addr(m, (void *)host);
    s_cat(m, " device ");          s_addr(m, (void *)dev);
    s_cat(m, " data ");            s_num(m, (long)esize);
    s_cat(m, " count ");           s_num(m, (long)count);
    s_cat(m, " retry ");           s_num(m, (long)retry);
    s_cat(m, " timeout ");         s_num(m, (long)timeout);
    s_cat(m, " flags ");           s_hex(m, flags);
    hr = ((F11)p->rv[S_EnumHosts])(p->real, desc, host, dev, edata, esize, count,
                                   retry, timeout, uctx, handle, flags);
    s_cat(m, " -> ");  s_hex(m, (DWORD)hr);
    logline(m);
    return hr;
}

static void *g_vtbl[S_COUNT] = {
    [S_QueryInterface]        = P_QueryInterface,
    [S_AddRef]                = P_AddRef,
    [S_Release]               = P_Release,
    [S_Initialize]            = P_Initialize,
    [S_EnumServiceProviders]  = P_EnumServiceProviders,
    [S_CancelAsyncOperation]  = P_CancelAsyncOperation,
    [S_Connect]               = P_Connect,
    [S_SendTo]                = P_SendTo,
    [S_GetSendQueueInfo]      = P_GetSendQueueInfo,
    [S_Host]                  = P_Host,
    [S_GetApplicationDesc]    = P_GetApplicationDesc,
    [S_SetApplicationDesc]    = P_SetApplicationDesc,
    [S_CreateGroup]           = P_CreateGroup,
    [S_DestroyGroup]          = P_DestroyGroup,
    [S_AddPlayerToGroup]      = P_AddPlayerToGroup,
    [S_RemovePlayerFromGroup] = P_RemovePlayerFromGroup,
    [S_SetGroupInfo]          = P_SetGroupInfo,
    [S_GetGroupInfo]          = P_GetGroupInfo,
    [S_EnumPlayersAndGroups]  = P_EnumPlayersAndGroups,
    [S_EnumGroupMembers]      = P_EnumGroupMembers,
    [S_SetPeerInfo]           = P_SetPeerInfo,
    [S_GetPeerInfo]           = P_GetPeerInfo,
    [S_GetPeerAddress]        = P_GetPeerAddress,
    [S_GetLocalHostAddresses] = P_GetLocalHostAddresses,
    [S_Close]                 = P_Close,
    [S_EnumHosts]             = P_EnumHosts,
    [S_DestroyPeer]           = P_DestroyPeer,
    [S_ReturnBuffer]          = P_ReturnBuffer,
    [S_GetPlayerContext]      = P_GetPlayerContext,
    [S_GetGroupContext]       = P_GetGroupContext,
    [S_GetCaps]               = P_GetCaps,
    [S_SetCaps]               = P_SetCaps,
    [S_SetSPCaps]             = P_SetSPCaps,
    [S_GetSPCaps]             = P_GetSPCaps,
    [S_GetConnectionInfo]     = P_GetConnectionInfo,
    [S_RegisterLobby]         = P_RegisterLobby,
    [S_TerminateSession]      = P_TerminateSession,
};

/* ---- the CoCreateInstance hook ---------------------------------------- */

static CoCreateInstance_t g_real_cci;
extern volatile int g_online;

#include "peer.c"

/* With Internet - Online chosen, the peer is ours (peer.c) and DirectPlay's is
 * never made; otherwise the game gets DirectPlay's.  Either way it is wrapped
 * in the tracing proxy, so the log reads the same for both. */
static HRESULT __stdcall hook_cci(const GUID *clsid, void *outer, DWORD ctx,
                                  const GUID *iid, void **out)
{
    HRESULT hr;
    char    m[200];
    int     ours = g_online && out && guid_eq(clsid, &CLSID_DirectPlay8Peer)
                   && guid_eq(iid, &IID_IDirectPlay8Peer);

    if (ours) {
        *out = peer_new();
        hr = *out ? S_OK : DPNERR_OUTOFMEMORY;
    } else
        hr = g_real_cci(clsid, outer, ctx, iid, out);

    m[0] = 0;
    s_cat(m, "CoCreateInstance ");  s_guid(m, clsid);
    s_cat(m, " iid ");              s_guid(m, iid);
    s_cat(m, " -> ");               s_hex(m, (DWORD)hr);
    if (ours) s_cat(m, "  OUR TRANSPORT (Internet - Online)");

    if (hr >= 0 && out && *out && guid_eq(clsid, &CLSID_DirectPlay8Peer)
        && guid_eq(iid, &IID_IDirectPlay8Peer)) {
        Proxy *p = (Proxy *)HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, sizeof(Proxy));
        if (p) {
            p->vtbl = g_vtbl;
            p->real = *out;
            p->rv   = *(void ***)*out;
            p->id   = ++g_nproxies;
            *out    = p;
            s_cat(m, "  wrapped as peer");
            s_num(m, p->id);
        }
    }
    logline(m);
    return hr;
}

/* Replace `name` from `dll` in `mod`'s import address table.  Matched by
 * name through the import lookup table, so it does not matter what the slot
 * currently points at.  Returns the old pointer, or 0. */
static void *patch_iat(HMODULE mod, const char *dll, const char *name, void *repl)
{
    BYTE        *base = (BYTE *)mod;
    const BYTE  *nt;
    DWORD        rva;
    const DWORD *desc;

    if (!base || *(WORD *)base != 0x5a4d) return 0;
    nt  = base + *(DWORD *)(base + 0x3c);
    rva = *(DWORD *)(nt + 0x80);                /* DataDirectory[IMPORT] */
    if (!rva) return 0;

    for (desc = (const DWORD *)(base + rva); desc[3]; desc += 5) {
        const char *dn = (const char *)(base + desc[3]);
        const char *a = dn, *b = dll;
        DWORD      *lookup, *iat;

        while (*a && *b && ((*a | 0x20) == (*b | 0x20))) { a++; b++; }
        if (*a || *b) continue;

        lookup = (DWORD *)(base + (desc[0] ? desc[0] : desc[4]));
        iat    = (DWORD *)(base + desc[4]);
        for (; *lookup; lookup++, iat++) {
            const char *fn;
            const char *x, *y;
            if (*lookup & 0x80000000) continue;          /* by ordinal */
            fn = (const char *)(base + *lookup + 2);
            for (x = fn, y = name; *x && *x == *y; x++, y++) {}
            if (*x || *y) continue;
            {
                void *old = (void *)*iat;
                DWORD prot;
                if (!VirtualProtect(iat, 4, PAGE_READWRITE, &prot)) return 0;
                *iat = (DWORD)repl;
                VirtualProtect(iat, 4, prot, &prot);
                return old;
            }
        }
    }
    return 0;
}

/* ---- the menu entry: Internet - Online -------------------------------- */
/*
 * The Multiplayer Connection screen (do_multiplayerConnection's dialog) has
 * five ShellButtons: GameSpy, Manual IP, LAN (TCP/IP), LAN (IPX) and Previous
 * Menu, labelled by read_text_label("multiplayer_connection", key) from the
 * game's label.map.  IPX has not existed on Windows since Vista, and neither
 * Wine's DirectPlay nor Microsoft's can use it there, so its button becomes
 * Internet - Online.  Three patches, each checked against the bytes it
 * replaces (and 0x5c00b6, where stub_ipx goes, checked too), all or none:
 *
 *   0x4d9a30  read_text_label's entry jumps here (label_hook): lan_ipx reads
 *             "Internet - Online"; while that entry is the one chosen, the
 *             Manual IP dialog's prompt and the connect/host texts read as
 *             online ones.  Every replacement is no longer than the stock
 *             text, so it fits whatever buffer the stock text fits.
 *   0x5bfd01  the screen's WM_LBUTTONUP case: every click first clears the
 *             online flag (stub_click), so no other button inherits it.
 *   0x5bff9b  the IPX button has just been drawn pressed; instead of its LAN
 *             connection, stub_ipx sets the flag and continues at 0x5c00b6,
 *             the Manual IP branch after its own button: do_manual_ip, then
 *             GenericConnection as an internet connection to the address
 *             typed (manual_ip_address).  Both branches are in the same
 *             frame with the same stack depth there, and the Manual IP path
 *             sets every register it reads after that point.
 *
 * Until our transport exists the entry connects exactly as Manual IP does,
 * through whatever DirectPlay is installed; g_online is what the transport
 * will go by.
 */
#define A_READ_LABEL     0x4d9a30
#define A_CLICK          0x5bfd01
#define A_CLICK_BACK     0x5bfd0a
#define A_IPX_PRESSED    0x5bff9b
#define A_MANUAL_DIALOG  0x5c00b6
#define A_ROOM_INIT      0x5b2ff2   /* InternetGameDlgProc: chatRoom.Init(...) */
#define A_GAME_INIT      0x5c5395   /* MultiplayerSetupDlgProc: chatGame.Init(...) */
#define A_CHAT_INIT      0x5a9890   /* Chat::Init, thiscall, one argument */
#define A_CHAT_APPEND    0x5a98b0   /* Chat::Append(this, format, ...), cdecl */
#define CHAT_ROOM_OBJ    0x79df50   /* chatRoom: the Internet Game screen's chat box */
#define CHAT_GAME_OBJ    0x79b838   /* chatGame: GAME SETUP's */
#define CHAT_HWND        0x2714     /* Chat: the edit control Init was given */

__attribute__((used)) volatile int g_online;   /* Internet - Online is the entry chosen */
static int  g_entry = 1;
static BYTE g_label_tramp[16];
typedef int (__cdecl *ReadLabel_t)(const char *section, const char *key, char *out);

static int s_eq(const char *a, const char *b)
{
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

static void s_copy(char *d, const char *s) { while ((*d++ = *s++)) {} }

static int __cdecl label_hook(const char *section, const char *key, char *out)
{
    static const char *const online[][3] = {
        { "multiplayer_manual_ip",  "manual_ip_address",      "Join code or host address (blank to host):" },
        { "multiplayer_connection", "connect_try_manual_ip",  "Trying to connect online" },
        { "multiplayer_connection", "cant_connect_manual_ip", "Cannot connect online" },
        { "commandline_net",        "title_manual_ip_join",   "JOINING AN ONLINE GAME" },
        { "commandline_net",        "title_manual_ip_host",   "HOSTING AN ONLINE GAME" },
    };
    int r = ((ReadLabel_t)(void *)g_label_tramp)(section, key, out);
    int i;

    if (!section || !key || !out) return r;
    if (s_eq(section, "multiplayer_connection") && s_eq(key, "lan_ipx")) {
        s_copy(out, "Internet - Online");
        return 1;
    }
    if (g_online)
        for (i = 0; i < (int)(sizeof online / sizeof online[0]); i++)
            if (s_eq(section, online[i][0]) && s_eq(key, online[i][1])) {
                s_copy(out, online[i][2]);
                return 1;
            }
    return r;
}

__attribute__((used)) void __cdecl on_online_chosen(void)
{
    g_online = 1;
    logline("menu: Internet - Online chosen");
}

/* ---- notices in the game's chat boxes ---------------------------------- *
 *
 * The join code, and what goes wrong finding a game, are shown where the game
 * shows its own "Local IP Address": as lines in a screen's chat box, through the
 * game's Chat::Append.  Chat::Init empties the box, and each screen calls it as it
 * opens, so a line for a screen that is not open waits for that call, and the
 * host's code is posted again whenever GAME SETUP opens anew (after a match).
 * The two Init calls are hooked at their call sites (install_entry). */

typedef void (__cdecl *ChatAppend_t)(void *chat, const char *format, ...);
static CRITICAL_SECTION g_ncs;
static int  g_chat_hooked;
static char g_pending[2][200];
static char g_sticky[200];

static void s_copyn(char *d, const char *s, int max)
{
    int i = 0;
    if (s) for (; s[i] && i < max - 1; i++) d[i] = s[i];
    d[i] = 0;
}

static void *chat_obj(int chat) { return (void *)(chat == CHAT_GAME ? CHAT_GAME_OBJ : CHAT_ROOM_OBJ); }

static int chat_open(int chat)
{
    HANDLE h = *(HANDLE *)((BYTE *)chat_obj(chat) + CHAT_HWND);
    return h && IsWindow(h);
}

static void chat_put(int chat, const char *text)
{
    ((ChatAppend_t)A_CHAT_APPEND)(chat_obj(chat), "%s", text);
}

static void notice(int chat, const char *text, int sticky)
{
    char t[200];
    int  open;
    if (!g_chat_hooked || !text || !text[0]) return;
    s_copyn(t, text, sizeof t);
    EnterCriticalSection(&g_ncs);
    if (sticky) s_copyn(g_sticky, t, sizeof g_sticky);
    open = chat_open(chat);
    if (!open && !sticky) s_copyn(g_pending[chat], t, sizeof g_pending[chat]);
    LeaveCriticalSection(&g_ncs);
    if (open) chat_put(chat, t);
}

static void notice_unstick(void)
{
    EnterCriticalSection(&g_ncs);
    g_sticky[0] = 0;
    LeaveCriticalSection(&g_ncs);
}

/* after a screen's Chat::Init: what was waiting for it */
__attribute__((used)) void __cdecl on_chat_init(void *obj)
{
    int  chat = obj == (void *)CHAT_GAME_OBJ ? CHAT_GAME : CHAT_ROOM;
    char t[200], s[200];
    EnterCriticalSection(&g_ncs);
    s_copyn(t, g_pending[chat], sizeof t);
    g_pending[chat][0] = 0;
    s_copyn(s, chat == CHAT_GAME ? g_sticky : "", sizeof s);
    LeaveCriticalSection(&g_ncs);
    if (t[0]) chat_put(chat, t);
    if (s[0]) chat_put(chat, s);
}

/* in place of `mov ecx, chat; call Chat::Init` (10 bytes): the same, then
 * on_chat_init(chat).  On entry ecx is free (it is loaded here), the argument is
 * on the stack, and Init pops it. */
__attribute__((naked)) static void stub_room_init(void)
{
    __asm__ volatile(
        "movl $0x79df50, %ecx\n\t"
        "pushl %ecx\n\t"
        "pushl 8(%esp)\n\t"
        "movl $0x5a9890, %eax\n\t"
        "call *%eax\n\t"
        "call _on_chat_init\n\t"
        "addl $4, %esp\n\t"
        "ret $4");
}

__attribute__((naked)) static void stub_game_init(void)
{
    __asm__ volatile(
        "movl $0x79b838, %ecx\n\t"
        "pushl %ecx\n\t"
        "pushl 8(%esp)\n\t"
        "movl $0x5a9890, %eax\n\t"
        "call *%eax\n\t"
        "call _on_chat_init\n\t"
        "addl $4, %esp\n\t"
        "ret $4");
}

/* every click on the connection screen: clear the flag, then the two
 * instructions the jump replaced */
__attribute__((naked)) static void stub_click(void)
{
    __asm__ volatile(
        "movl $0, _g_online\n\t"
        "movl 0x14(%ebp), %ebx\n\t"
        "movl 0x7a30a0, %ecx\n\t"
        "pushl $0x5bfd0a\n\t"
        "ret");
}

/* the IPX button pressed: eax, ecx and edx are free here (the Manual IP
 * branch loads each before use) */
__attribute__((naked)) static void stub_ipx(void)
{
    __asm__ volatile(
        "call _on_online_chosen\n\t"
        "pushl $0x5c00b6\n\t"
        "ret");
}

static int code_is(DWORD addr, const BYTE *bytes, int n)
{
    int i;
    for (i = 0; i < n; i++) if (((const BYTE *)addr)[i] != bytes[i]) return 0;
    return 1;
}

static int write_code(DWORD addr, const BYTE *bytes, int n)
{
    DWORD prot;
    int   i;
    if (!VirtualProtect((void *)addr, (UINT)n, PAGE_EXECUTE_READWRITE, &prot)) return 0;
    for (i = 0; i < n; i++) ((BYTE *)addr)[i] = bytes[i];
    VirtualProtect((void *)addr, (UINT)n, prot, &prot);
    return 1;
}

/* a 5-byte call to `to` at `at`, NOP-padded to n bytes */
static int write_call(DWORD at, void *to, int n)
{
    BYTE b[16];
    DWORD rel = (DWORD)to - (at + 5);
    int   i;
    b[0] = 0xe8;
    for (i = 0; i < 4; i++) b[1 + i] = (BYTE)(rel >> (8 * i));
    for (i = 5; i < n; i++) b[i] = 0x90;
    return write_code(at, b, n);
}

/* a 5-byte jmp to `to` at `at`, NOP-padded to n bytes */
static int write_jmp(DWORD at, void *to, int n)
{
    BYTE b[16];
    DWORD rel = (DWORD)to - (at + 5);
    int   i;
    b[0] = 0xe9;
    for (i = 0; i < 4; i++) b[1 + i] = (BYTE)(rel >> (8 * i));
    for (i = 5; i < n; i++) b[i] = 0x90;
    return write_code(at, b, n);
}

static void install_entry(char *b)
{
    static const BYTE sig_label[8] = { 0x55, 0x8b, 0xec, 0xa0, 0x68, 0x13, 0x76, 0x00 };
    static const BYTE sig_click[9] = { 0x8b, 0x5d, 0x14, 0x8b, 0x0d, 0xa0, 0x30, 0x7a, 0x00 };
    static const BYTE sig_ipx[8]   = { 0xa1, 0x00, 0x2d, 0x7a, 0x00, 0x8b, 0x4d, 0x08 };
    static const BYTE sig_dlg[9]   = { 0x8b, 0x4d, 0x08, 0x51, 0xe8, 0x21, 0x6b, 0xff, 0xff };
    static const BYTE sig_room[10] = { 0xb9, 0x50, 0xdf, 0x79, 0x00, 0xe8, 0x94, 0x68, 0xff, 0xff };
    static const BYTE sig_game[10] = { 0xb9, 0x38, 0xb8, 0x79, 0x00, 0xe8, 0xf1, 0x44, 0xfe, 0xff };
    DWORD prot, rel;
    int   i;

    if (!g_entry) { s_cat(b, "  menu entry off (Entry=0)"); return; }
    if (!code_is(A_READ_LABEL, sig_label, 8) || !code_is(A_CLICK, sig_click, 9)
        || !code_is(A_IPX_PRESSED, sig_ipx, 8) || !code_is(A_MANUAL_DIALOG, sig_dlg, 9)
        || !code_is(A_ROOM_INIT, sig_room, 10) || !code_is(A_GAME_INIT, sig_game, 10)) {
        s_cat(b, "  menu entry NOT installed: Armada2.exe is not the 1.1 this was made for");
        return;
    }
    /* trampoline: read_text_label's first 8 bytes, then back to the rest */
    for (i = 0; i < 8; i++) g_label_tramp[i] = sig_label[i];
    rel = (A_READ_LABEL + 8) - ((DWORD)g_label_tramp + 8 + 5);
    g_label_tramp[8] = 0xe9;
    for (i = 0; i < 4; i++) g_label_tramp[9 + i] = (BYTE)(rel >> (8 * i));
    VirtualProtect(g_label_tramp, sizeof g_label_tramp, PAGE_EXECUTE_READWRITE, &prot);

    if (write_jmp(A_READ_LABEL, (void *)label_hook, 8)
        && write_jmp(A_CLICK, (void *)stub_click, 9)
        && write_jmp(A_IPX_PRESSED, (void *)stub_ipx, 5)
        && write_call(A_ROOM_INIT, (void *)stub_room_init, 10)
        && write_call(A_GAME_INIT, (void *)stub_game_init, 10)) {
        g_chat_hooked = 1;
        s_cat(b, "  menu entry: LAN (IPX) is Internet - Online");
    }
    else
        s_cat(b, "  menu entry: VirtualProtect failed");
}

/* ---- startup ---------------------------------------------------------- */

static void build_paths(char *ini)
{
    char path[320];
    int  n, i, cut = 0;

    n = (int)GetModuleFileNameA(NULLPTR, path, 300);
    if (n <= 0) { ini[0] = 0; g_logpath[0] = 0; return; }
    for (i = 0; i < n; i++) if (path[i] == '\\' || path[i] == '/') cut = i + 1;
    path[cut] = 0;

    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "Online.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "Online.log");
}

static void startup(void)
{
    char    ini[320];
    char    b[400];
    HMODULE nm;

    InitializeCriticalSection(&g_cs);
    InitializeCriticalSection(&g_ncs);
    g_t0 = GetTickCount();
    build_paths(ini);
    g_logging = (int)GetPrivateProfileIntA("Online", "Log",     1,  ini);
    g_payload = (int)GetPrivateProfileIntA("Online", "Payload", 16, ini);
    if (g_payload > 64) g_payload = 64;
    g_entry   = (int)GetPrivateProfileIntA("Online", "Entry",   1,  ini);
    g_port    = (WORD)GetPrivateProfileIntA("Online", "Port",   2302, ini);
    g_loss    = (int)GetPrivateProfileIntA("Online", "Loss",    0,  ini);
    if (g_loss > 50) g_loss = 50;
    g_direct  = (int)GetPrivateProfileIntA("Online", "Direct",  1,  ini);
    GetPrivateProfileStringA("Online", "Server", "c20e.de", g_server, sizeof g_server, ini);
    if (g_logging && g_logpath[0])
        g_log = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                            CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);

    b[0] = 0;
    s_cat(b, "--- Online payload=");
    s_num(b, g_payload);
    s_cat(b, "  server=");
    s_cat(b, g_server[0] ? g_server : "(none)");
    if (!g_direct) s_cat(b, "  Direct=0: relay only");
    nm = GetModuleHandleA("NetworkManager.dll");
    if (!nm) {
        s_cat(b, "  NetworkManager.dll not loaded: nothing hooked");
    } else {
        g_real_cci = (CoCreateInstance_t)patch_iat(nm, "ole32.dll", "CoCreateInstance",
                                                   (void *)hook_cci);
        s_cat(b, g_real_cci ? "  NetworkManager.dll CoCreateInstance hooked"
                            : "  NetworkManager.dll imports no CoCreateInstance: nothing hooked");
    }
    install_entry(b);
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
