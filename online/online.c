/*
 * Online.asi -- online multiplayer for Star Trek: Armada II.
 *
 * THIS VERSION ONLY WATCHES.  It is the first step of the plan in README.md:
 * it puts a pass-through in front of the game's DirectPlay 8 peer and logs
 * every call the game makes, every message DirectPlay hands back, and the
 * addresses that cross the interface.  That log is the specification of the
 * DirectPlay subset our own transport has to implement.  Nothing the game
 * sends or receives is changed.
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
#define HEAP_ZERO_MEMORY       0x08

typedef struct { BYTE opaque[24]; } CRITICAL_SECTION;

__declspec(dllimport) HMODULE __stdcall GetModuleHandleA(LPCSTR);
__declspec(dllimport) DWORD   __stdcall GetModuleFileNameA(HMODULE, LPSTR, DWORD);
__declspec(dllimport) BOOL    __stdcall VirtualProtect(void *, UINT, DWORD, DWORD *);
__declspec(dllimport) HANDLE  __stdcall CreateFileA(LPCSTR, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
__declspec(dllimport) BOOL    __stdcall WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, INT, LPCSTR);
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

static HRESULT __stdcall hook_cci(const GUID *clsid, void *outer, DWORD ctx,
                                  const GUID *iid, void **out)
{
    HRESULT hr = g_real_cci(clsid, outer, ctx, iid, out);
    char    m[200];

    m[0] = 0;
    s_cat(m, "CoCreateInstance ");  s_guid(m, clsid);
    s_cat(m, " iid ");              s_guid(m, iid);
    s_cat(m, " -> ");               s_hex(m, (DWORD)hr);

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
    char    b[200];
    HMODULE nm;

    InitializeCriticalSection(&g_cs);
    g_t0 = GetTickCount();
    build_paths(ini);
    g_logging = (int)GetPrivateProfileIntA("Online", "Log",     1,  ini);
    g_payload = (int)GetPrivateProfileIntA("Online", "Payload", 16, ini);
    if (g_payload > 64) g_payload = 64;
    if (g_logging && g_logpath[0])
        g_log = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                            CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);

    b[0] = 0;
    s_cat(b, "--- Online (trace) payload=");
    s_num(b, g_payload);
    nm = GetModuleHandleA("NetworkManager.dll");
    if (!nm) {
        s_cat(b, "  NetworkManager.dll not loaded: nothing hooked");
    } else {
        g_real_cci = (CoCreateInstance_t)patch_iat(nm, "ole32.dll", "CoCreateInstance",
                                                   (void *)hook_cci);
        s_cat(b, g_real_cci ? "  NetworkManager.dll CoCreateInstance hooked"
                            : "  NetworkManager.dll imports no CoCreateInstance: nothing hooked");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
