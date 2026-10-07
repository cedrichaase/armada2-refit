/*
 * peer.c -- our own IDirectPlay8Peer, over UDP.  Included by online.c (one
 * translation unit, so it shares the log and string helpers there).
 *
 * The game gets this object instead of DirectPlay's when the player has chosen
 * Internet - Online (g_online, set by the menu entry).  It implements what the
 * game was traced calling (README.md, "What the game calls") and delivers the
 * same messages in the same order:
 *
 *   EnumHosts      queries the typed host every retry interval until cancelled;
 *                  each reply is an ENUM_HOSTS_RESPONSE with a real
 *                  IDirectPlay8Address of the host, which the game hands back
 *                  to Connect.  No typed host: nothing is sent (no LAN
 *                  broadcast; the server's game list is milestone 3).
 *   Host           binds the port (Port=, 2302), CREATE_PLAYER for the host
 *                  before returning, and answers queries with the game's own
 *                  reply data (ENUM_HOSTS_QUERY, then RETURN_BUFFER).
 *   Connect        the host sees INDICATE_CONNECT and may refuse; the joiner
 *                  gets CREATE_PLAYER for itself, then for every other player,
 *                  then CONNECT_COMPLETE.
 *   SendTo         guaranteed and ordered whatever the flags (the game only
 *                  ever asks for that), to a player, to itself, or to all
 *                  (0, itself included).  SEND_COMPLETE once queued.
 *   Close          DESTROY_PLAYER for every player before it returns.
 *
 * Topology: a star through the host.  A joiner talks to the host only, and the
 * host forwards between joiners.  The game's DPNIDs are ours to choose.
 *
 * Wire format: UDP datagrams starting "A2O" + version 1, then a type byte.
 * Per connection there is one reliable ordered stream (DATA with a sequence
 * number, cumulative ACK plus a 32-packet selective bitmask, retransmission on
 * a timer, fragmentation at 1100 bytes); everything else is a lone datagram,
 * retried by whoever wants an answer.  A connection that hears nothing for
 * 15 s is lost.
 *
 * Threads: one network thread per peer receives, retransmits and delivers
 * every message to the game's handler, in order.  The game's calls only queue.
 * The handler is never called with our lock held: the game calls back into us
 * from it (GetPeerInfo in CREATE_PLAYER, SendTo in RECEIVE).  Host's
 * CREATE_PLAYER, Close's DESTROY_PLAYERs and CancelAsyncOperation's
 * ASYNC_OP_COMPLETE are delivered on the calling thread, as DirectPlay does.
 */

/* ---- Winsock and the rest of Win32, as much as is needed -------------- */

typedef unsigned int SOCKET;
#define INVALID_SOCKET   ((SOCKET)~0u)
#define AF_INET          2
#define SOCK_DGRAM       2
#define IPPROTO_UDP      17
#define FIONBIO          0x8004667e
#define SOL_SOCKET       0xffff
#define SO_RCVBUF        0x1002
#define SO_SNDBUF        0x1001

typedef struct { WORD family; WORD port; DWORD addr; BYTE zero[8]; } SOCKADDR_IN;
typedef struct { UINT count; SOCKET fd[64]; } FD_SET;
typedef struct { LONG sec, usec; } TIMEVAL;
typedef struct { char *name; char **aliases; short type, len; char **addrs; } HOSTENT;
typedef struct { WORD ver, high; char desc[257]; char status[129]; WORD maxs, maxdg; char *vendor; } WSADATA;

__declspec(dllimport) int     __stdcall WSAStartup(WORD, WSADATA *);
__declspec(dllimport) SOCKET  __stdcall socket(int, int, int);
__declspec(dllimport) int     __stdcall bind(SOCKET, const void *, int);
__declspec(dllimport) int     __stdcall sendto(SOCKET, const char *, int, int, const void *, int);
__declspec(dllimport) int     __stdcall recvfrom(SOCKET, char *, int, int, void *, int *);
__declspec(dllimport) int     __stdcall closesocket(SOCKET);
__declspec(dllimport) int     __stdcall select(int, FD_SET *, FD_SET *, FD_SET *, const TIMEVAL *);
__declspec(dllimport) int     __stdcall ioctlsocket(SOCKET, LONG, DWORD *);
__declspec(dllimport) int     __stdcall setsockopt(SOCKET, int, int, const char *, int);
__declspec(dllimport) int     __stdcall getsockname(SOCKET, void *, int *);
__declspec(dllimport) int     __stdcall connect(SOCKET, const void *, int);
__declspec(dllimport) HOSTENT *__stdcall gethostbyname(const char *);
__declspec(dllimport) int     __stdcall WSAGetLastError(void);

__declspec(dllimport) HANDLE  __stdcall CreateThread(void *, UINT, DWORD (__stdcall *)(void *), void *, DWORD, DWORD *);
__declspec(dllimport) DWORD   __stdcall WaitForSingleObject(HANDLE, DWORD);
__declspec(dllimport) BOOL    __stdcall CloseHandle(HANDLE);
__declspec(dllimport) BOOL    __stdcall HeapFree(HANDLE, DWORD, void *);
__declspec(dllimport) HRESULT __stdcall CoInitializeEx(void *, DWORD);
__declspec(dllimport) void    __stdcall CoUninitialize(void);

#define COINIT_MULTITHREADED 0

/* ---- DirectPlay 8: messages, codes, flags ----------------------------- */

typedef DWORD DPNID;
typedef DWORD DPNHANDLE;

#define S_OK                       0
#define E_NOINTERFACE              ((HRESULT)0x80004002)
#define E_POINTER                  ((HRESULT)0x80004003)
#define DPNSUCCESS_PENDING         ((HRESULT)0x0015800e)
#define DPNERR(c)                  ((HRESULT)(0x80158000u + (c)))
#define DPNERR_UNSUPPORTED         ((HRESULT)0x80004001)
#define DPNERR_INVALIDPARAM        ((HRESULT)0x80070057)
#define DPNERR_OUTOFMEMORY         ((HRESULT)0x8007000e)
#define DPNERR_GENERIC             ((HRESULT)0x80004005)
#define DPNERR_ADDRESSING          DPNERR(0x040)
#define DPNERR_ALREADYINITIALIZED  DPNERR(0x080)
#define DPNERR_BUFFERTOOSMALL      DPNERR(0x100)
#define DPNERR_CONNECTIONLOST      DPNERR(0x160)
#define DPNERR_HOSTREJECTEDCONNECTION DPNERR(0x260)
#define DPNERR_HOSTTERMINATEDSESSION  DPNERR(0x270)
#define DPNERR_INVALIDHANDLE       DPNERR(0x360)
#define DPNERR_INVALIDPASSWORD     DPNERR(0x410)
#define DPNERR_INVALIDPLAYER       DPNERR(0x420)
#define DPNERR_NORESPONSE          DPNERR(0x510)
#define DPNERR_NOTHOST             DPNERR(0x530)
#define DPNERR_NOTREADY            DPNERR(0x540)
#define DPNERR_SESSIONFULL         DPNERR(0x610)
#define DPNERR_UNINITIALIZED       DPNERR(0x640)
#define DPNERR_USERCANCEL          DPNERR(0x650)

#define DPN_MSGID_APPLICATION_DESC   0xFFFF0002
#define DPN_MSGID_ASYNC_OP_COMPLETE  0xFFFF0003
#define DPN_MSGID_PEER_INFO          0xFFFF0010
#define DPN_MSGID_RETURN_BUFFER      0xFFFF0013
#define DPN_MSGID_SEND_COMPLETE      0xFFFF0014
#define DPN_MSGID_TERMINATE_SESSION  0xFFFF0016

#define DPNID_ALL_PLAYERS_GROUP 0
#define DPNPLAYER_LOCAL         0x0002
#define DPNPLAYER_HOST          0x0004
#define DPNINFO_NAME            0x0001
#define DPNSEND_SYNC            0x80000000
#define DPNSEND_NOCOMPLETE      0x0002
#define DPNSEND_NOLOOPBACK      0x0020
#define DPNSENDCOMPLETE_GUARANTEED 0x0001
#define DPNSESSION_REQUIREPASSWORD 0x0080
#define DPNDESTROYPLAYERREASON_NORMAL            1
#define DPNDESTROYPLAYERREASON_CONNECTIONLOST    2
#define DPNDESTROYPLAYERREASON_SESSIONTERMINATED 3
#define DPNDESTROYPLAYERREASON_HOSTDESTROYEDPLAYER 4
#define DPNA_DATATYPE_STRING 1
#define DPNA_DATATYPE_DWORD  2

typedef struct {
    DWORD size, flags; GUID instance, application; DWORD max, cur;
    WCHAR *name, *password; void *reserved; DWORD reserved_size;
    void *app_reserved; DWORD app_reserved_size;
} AppDesc;
typedef struct { DWORD size; BYTE *data; } BufDesc;
typedef struct { DWORD size, info_flags; WCHAR *name; void *data; DWORD data_size, player_flags; } PlayerInfo;
typedef struct { DWORD size, flags, threads, enum_count, enum_retry, enum_timeout, max_enum_payload, buffers, sysbuf; } SpCaps;
typedef struct { DWORD size, flags, connect_timeout, connect_retries, keepalive; } Caps;

typedef struct { DWORD size; DPNHANDLE op; void *ctx; HRESULT hr; } MsgAsyncDone;
typedef struct { DWORD size; DPNHANDLE op; void *ctx; HRESULT hr; void *reply; DWORD reply_size; DPNID local; } MsgConnectDone;
typedef struct { DWORD size; DPNID id; void *ctx; } MsgPlayer;
typedef struct { DWORD size; DPNID id; void *ctx; DWORD reason; } MsgDestroy;
typedef struct { DWORD size; void *sender, *device; void *data; DWORD data_size, max_reply; void *reply; DWORD reply_size; void *reply_ctx; } MsgEnumQuery;
typedef struct { DWORD size; void *sender, *device; const AppDesc *desc; void *data; DWORD data_size; void *ctx; DWORD rtt; } MsgEnumResponse;
typedef struct { DWORD size; void *data; DWORD data_size; void *reply; DWORD reply_size; void *reply_ctx; void *player_ctx; void *addr, *device; } MsgIndicate;
typedef struct { DWORD size; DPNID from; void *ctx; BYTE *data; DWORD data_size; DPNHANDLE buf; DWORD flags; } MsgReceive;
typedef struct { DWORD size; HRESULT hr; void *buf; void *ctx; } MsgReturnBuffer;
typedef struct { DWORD size; DPNHANDLE op; void *ctx; HRESULT hr; DWORD time, rtt, retries, flags; BufDesc *bufs; DWORD nbufs; } MsgSendDone;
typedef struct { DWORD size; HRESULT hr; void *data; DWORD data_size; } MsgTerminate;

static const GUID CLSID_DirectPlay8Address =
    { 0x934a9523, 0xa3ca, 0x4bc5, { 0xad, 0xa0, 0xd6, 0xd9, 0x5d, 0x97, 0x94, 0x21 } };
static const GUID IID_IDirectPlay8Address =
    { 0x83783300, 0x4063, 0x4c8a, { 0x9d, 0xb3, 0x82, 0x83, 0x0a, 0x7f, 0xeb, 0x31 } };
static const GUID CLSID_DP8SP_TCPIP =
    { 0xebfe7ba0, 0x628d, 0x11d2, { 0xae, 0x0f, 0x00, 0x60, 0x97, 0xb0, 0x14, 0x11 } };

/* IDirectPlay8Address slots used */
#define A_RELEASE      2
#define A_SETSP        13
#define A_GETCOMPONENT 16
#define A_ADDCOMPONENT 18
typedef ULONG   (__stdcall *AddrRelease_t)(void *);
typedef HRESULT (__stdcall *AddrSetSP_t)(void *, const GUID *);
typedef HRESULT (__stdcall *AddrGet_t)(void *, const WCHAR *, void *, DWORD *, DWORD *);
typedef HRESULT (__stdcall *AddrAdd_t)(void *, const WCHAR *, const void *, DWORD, DWORD);

/* ---- the peer ----------------------------------------------------------- */

#define MAX_PLAYERS  16
#define MAX_CONNS    16
#define NAME_CHARS   64
#define FRAG         1100            /* reliable payload per datagram */
#define WINDOW       64              /* out-of-order slots, and packets in flight */
#define LOST_MS      15000
#define PING_MS      1000
#define CONNECT_MS   10000
#define PUNCH_MS     2500            /* how long both sides try to reach each other directly */
#define PUNCH_EVERY  200
#define ANSWER_MS    30000           /* the host answers a joiner's probes for this long */
#define LOOKUP_MS    8000            /* a join code's lookup, before "the server does not answer" */
#define REGISTER_MS  10000           /* a registered host refreshes its code this often */
#define VPORT        0x0100          /* port 1, network order: the port of a relayed address */

enum { ROLE_NONE, ROLE_HOST, ROLE_JOINER };
enum { P_ENUM_Q = 1, P_ENUM_R, P_CONN, P_REJECT, P_DATA, P_ACK, P_PING, P_PONG, P_BYE, P_PUNCH };
/* to and from the server (server/a2online-server.py) */
enum { SV_HOST = 0x40, SV_HOSTED, SV_JOIN, SV_PEER, SV_INTRO, SV_NOTFOUND, SV_RELAY, SV_RELAYED,
       SV_BYE, SV_ERROR };
enum { CHAT_ROOM, CHAT_GAME };       /* where a notice goes: the Internet Game screen, GAME SETUP */
enum { R_ACCEPT = 1, R_APP, R_PLAYER_ADD, R_PLAYER_DEL, R_INFO, R_APPDESC, R_TERMINATE, R_RULES };
enum { EV_RECEIVE = 1, EV_SEND_DONE, EV_CREATE, EV_DESTROY, EV_CONNECT_DONE, EV_ENUM_RESPONSE,
       EV_ENUM_QUERY, EV_INDICATE, EV_TERMINATE, EV_PEER_INFO, EV_APPDESC, EV_ASYNC_DONE, EV_NOTICE };

typedef struct Pkt { struct Pkt *next; DWORD seq, sent, first; int tries, len; BYTE data[FRAG + 16]; } Pkt;

typedef struct {
    int    used, open;
    DWORD  ip; WORD port;            /* network order */
    DPNID  player;
    DWORD  next_seq, expect;
    Pkt   *unacked, *last;
    int    inflight;
    BYTE  *ooo[WINDOW]; int ooo_len[WINDOW];
    BYTE  *msg; int msg_len, msg_cap;
    DWORD  last_rx, last_tx, rtt, rto;
    int    rules;                    /* host: this joiner runs QOLRules with PayOnQueue */
} Conn;

typedef struct {
    int   used;
    DPNID id;
    DWORD flags;                     /* DPNPLAYER_* as GetPeerInfo reports them */
    WCHAR name[NAME_CHARS];
    void *ctx;
    int   conn;                      /* host: the joiner's connection; -1 otherwise */
} Player;

typedef struct Ev {
    struct Ev *next;
    int    kind;
    DWORD  a, b, c;                  /* ids, handles, reasons, hr, ip/port */
    void  *ctx;
    int    len;
    BYTE   data[1];
} Ev;

typedef struct {
    void          **vtbl;
    volatile LONG   refs;
    MsgHandler_t    handler;
    void           *hctx;
    CRITICAL_SECTION cs;
    SOCKET          sock;
    WORD            bound;           /* port, host order; 0 = ephemeral */
    HANDLE          thread;
    DWORD           thread_id;
    volatile int    stop, closed;
    int             role;
    DPNID           self, host, next_id;
    int             rules_all;       /* every player runs QOLRules: what the game plays by */
    WCHAR           myname[NAME_CHARS];
    /* the session, as the host keeps it (a joiner's copy comes with R_ACCEPT) */
    DWORD           sflags, smax, scur;
    GUID            sinst, sapp;
    WCHAR           sname[NAME_CHARS], spass[NAME_CHARS];
    Player          players[MAX_PLAYERS];
    Conn            conns[MAX_CONNS];
    /* EnumHosts */
    int             en_on; DPNHANDLE en_h; void *en_ctx; DWORD en_ip; WORD en_port;
    DWORD           en_left, en_retry, en_next, en_end; BYTE en_pkt[300]; int en_len;
    /* Connect (joiner) */
    int             cn_on; DPNHANDLE cn_h; void *cn_ctx, *cn_pctx; DWORD cn_nonce, cn_start, cn_next;
    BYTE            cn_pkt[600]; int cn_len;
    /* host: connection requests waiting for the game's INDICATE_CONNECT answer */
    DWORD           pend_ip[MAX_CONNS]; WORD pend_port[MAX_CONNS];
    /* the server: our LAN address as we would give it out */
    DWORD           lan_ip; WORD lan_port;
    /* host: the join code (sv_state 1 = asking for one, 2 = have it) */
    int             sv_state, sv_warned; DWORD sv_token, sv_start, sv_next;
    char            code[8];
    /* host: joiners the server introduced: probed for PUNCH_MS, answered for ANSWER_MS */
    struct { DWORD pair, start, next; DWORD ip[2]; WORD port[2]; } punch[MAX_CONNS];
    /* joiner: a join code (jn_state 1 = asking the server, 2 = probing the host, 3 = path chosen) */
    int             jn_state; char jn_code[8];
    DWORD           jn_nonce, jn_start, jn_next, jn_pair, jn_host_id;
    DWORD           jn_ip[4]; WORD jn_port[4]; int jn_n;
    Ev             *in_head, *in_tail;
    DWORD           next_handle;
    DWORD           stat_sent, stat_recv, stat_resent, stat_dropped;
} Peer;

static WORD  g_port = 2302;
static int   g_loss;                 /* Loss=: percent of datagrams dropped on purpose (tests) */
static int   g_direct = 1;           /* Direct=0: never try a direct path, always relay (tests) */
static char  g_server[128];          /* Server=host:port; empty = none */
static DWORD g_srv_ip; static WORD g_srv_port;   /* resolved, network order */

/* posts a line in one of the game's chat boxes (online.c); sticky = again
 * whenever that screen is set up anew, until notice_unstick() */
static void notice(int chat, const char *text, int sticky);
static void notice_unstick(void);
static DWORD g_rand = 0x2a2a2a2a;
static void *g_peer_vtbl[37];

static void *mem(int n)     { return HeapAlloc(GetProcessHeap(), HEAP_ZERO_MEMORY, (UINT)n); }
static void  unmem(void *p) { if (p) HeapFree(GetProcessHeap(), 0, p); }
static void  mcpy(void *d, const void *s, int n) { BYTE *a = d; const BYTE *b = s; while (n-- > 0) *a++ = *b++; }
static void  mzero(void *d, int n) { BYTE *a = d; while (n-- > 0) *a++ = 0; }
static WORD  hton16(WORD v) { return (WORD)((v >> 8) | (v << 8)); }

static void put16(BYTE *p, DWORD v) { p[0] = (BYTE)v; p[1] = (BYTE)(v >> 8); }
static void put32(BYTE *p, DWORD v) { put16(p, v); put16(p + 2, v >> 16); }
static DWORD get16(const BYTE *p) { return p[0] | (p[1] << 8); }
static DWORD get32(const BYTE *p) { return get16(p) | (get16(p + 2) << 16); }

static int wlen(const WCHAR *w) { int n = 0; if (w) while (w[n]) n++; return n; }
static void wcopy(WCHAR *d, const WCHAR *s, int max)
{
    int i = 0;
    if (s) for (; s[i] && i < max - 1; i++) d[i] = s[i];
    d[i] = 0;
}
static int weq(const WCHAR *a, const WCHAR *b)
{
    if (!a) a = (const WCHAR *)L"";
    if (!b) b = (const WCHAR *)L"";
    while (*a && *a == *b) { a++; b++; }
    return *a == *b;
}

/* a wide string on the wire: u16 count, then the characters */
static int put_w(BYTE *p, const WCHAR *w)
{
    int n = wlen(w), i;
    if (n > NAME_CHARS - 1) n = NAME_CHARS - 1;
    put16(p, (DWORD)n);
    for (i = 0; i < n; i++) put16(p + 2 + 2 * i, w[i]);
    return 2 + 2 * n;
}
static int get_w(const BYTE *p, int avail, WCHAR *w)
{
    int n, i;
    if (avail < 2) { w[0] = 0; return avail; }
    n = (int)get16(p);
    if (2 + 2 * n > avail) n = (avail - 2) / 2;
    for (i = 0; i < n && i < NAME_CHARS - 1; i++) w[i] = (WCHAR)get16(p + 2 + 2 * i);
    w[i] = 0;
    return 2 + 2 * n;
}

static void plog(Peer *p, const char *what)
{
    char b[300];
    b[0] = 0;
    s_cat(b, p->role == ROLE_HOST ? "net host: " : p->role == ROLE_JOINER ? "net joiner: " : "net: ");
    s_cat(b, what);
    logline(b);
}

/* A peer reached through the server's relay has the address 0.x.y.z, port 1:
 * x.y.z is the id the server gave it.  No real datagram comes from 0.0.0.0/8, and
 * DirectPlay's address objects carry it like any other, so the game hands it back
 * to Connect and every connection is keyed by it as by a real address. */
static int   is_relayed(DWORD ip) { return ip && (ip & 0xff) == 0; }
static DWORD relayed_ip(DWORD id) { return ((id >> 16) & 0xff) << 8 | ((id >> 8) & 0xff) << 16 | (id & 0xff) << 24; }
static DWORD relay_id(DWORD ip)   { return ((ip >> 8) & 0xff) << 16 | ((ip >> 16) & 0xff) << 8 | (ip >> 24); }

static void s_dotted(char *d, DWORD ip)
{
    const BYTE *b = (const BYTE *)&ip;
    s_num(d, b[0]); s_cat(d, "."); s_num(d, b[1]); s_cat(d, ".");
    s_num(d, b[2]); s_cat(d, "."); s_num(d, b[3]);
}

static void s_ip(char *d, DWORD ip, WORD port)
{
    if (is_relayed(ip)) { s_cat(d, "relay #"); s_num(d, (long)relay_id(ip)); return; }
    s_dotted(d, ip); s_cat(d, ":"); s_num(d, hton16(port));
}

/* "K7M-Q2X" */
static void s_code(char *d, const char *code)
{
    char t[8];
    int  i;
    for (i = 0; i < 3; i++) { t[i] = code[i]; t[i + 4] = code[i + 3]; }
    t[3] = '-'; t[7] = 0;
    s_cat(d, t);
}

/* a join code as typed: six of the server's characters (no I, L, O, 0 or 1), any
 * case, dashes and spaces ignored.  1 and `code` set to the six, upper case */
static int read_code(const char *s, char *code)
{
    static const char alphabet[] = "ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    int n = 0, k;
    for (; *s; s++) {
        char ch = *s;
        if (ch == '-' || ch == ' ') continue;
        if (ch >= 'a' && ch <= 'z') ch = (char)(ch - 32);
        for (k = 0; alphabet[k] && alphabet[k] != ch; k++) {}
        if (!alphabet[k] || n == 6) return 0;
        code[n++] = ch;
    }
    code[n] = 0;
    return n == 6;
}

/* ---- addresses ------------------------------------------------------------ */

static void *make_addr(DWORD ip, WORD port)
{
    void   *a = 0;
    WCHAR   host[24];
    char    t[24];
    DWORD   dport = hton16(port);
    int     i;

    if (!g_real_cci || g_real_cci(&CLSID_DirectPlay8Address, 0, 1, &IID_IDirectPlay8Address, &a) < 0 || !a)
        return 0;
    ((AddrSetSP_t)(*(void ***)a)[A_SETSP])(a, &CLSID_DP8SP_TCPIP);
    if (!ip && !port) return a;                     /* a device: the provider is all */
    t[0] = 0;
    s_dotted(t, ip);
    for (i = 0; t[i]; i++) host[i] = (WCHAR)t[i];
    host[i] = 0;
    ((AddrAdd_t)(*(void ***)a)[A_ADDCOMPONENT])(a, (const WCHAR *)L"hostname", host,
                                                 (DWORD)(2 * (i + 1)), DPNA_DATATYPE_STRING);
    ((AddrAdd_t)(*(void ***)a)[A_ADDCOMPONENT])(a, (const WCHAR *)L"port", &dport, 4, DPNA_DATATYPE_DWORD);
    return a;
}

static void drop_addr(void *a) { if (a) ((AddrRelease_t)(*(void ***)a)[A_RELEASE])(a); }

static int parse_ip(const char *s, DWORD *ip)
{
    DWORD v = 0; int part = 0, n = 0, digits = 0;
    for (;; s++) {
        if (*s >= '0' && *s <= '9') { n = n * 10 + (*s - '0'); digits++; if (n > 255) return 0; }
        else if ((*s == '.' || !*s) && digits) {
            v |= (DWORD)n << (8 * part);
            part++; n = 0; digits = 0;
            if (!*s) break;
            if (part > 3) return 0;
        } else return 0;
    }
    if (part != 4) return 0;
    *ip = v;
    return 1;
}

/* hostname and port out of the game's address: 1 = an address, 2 = a join code
 * (in `code`), 0 = no hostname in it, -1 = a name that does not resolve */
static int addr_target(void *a, DWORD *ip, WORD *port, char *code)
{
    WCHAR w[128];
    char  s[128];
    DWORD size = sizeof w, type = 0, dport = 0;
    int   i;

    *port = hton16(g_port);
    if (!a) return 0;
    if (((AddrGet_t)(*(void ***)a)[A_GETCOMPONENT])(a, (const WCHAR *)L"hostname", w, &size, &type) < 0)
        return 0;
    if (type == DPNA_DATATYPE_STRING) {
        for (i = 0; w[i] && i < 127; i++) s[i] = (char)w[i];
        s[i] = 0;
    } else {
        for (i = 0; ((char *)w)[i] && i < 127; i++) s[i] = ((char *)w)[i];
        s[i] = 0;
    }
    size = 4;
    if (((AddrGet_t)(*(void ***)a)[A_GETCOMPONENT])(a, (const WCHAR *)L"port", &dport, &size, &type) >= 0
        && dport)
        *port = hton16((WORD)dport);
    if (!s[0]) return 0;
    if (parse_ip(s, ip)) return 1;
    if (code && read_code(s, code)) return 2;
    {
        HOSTENT *h = gethostbyname(s);
        if (h && h->addrs && h->addrs[0]) { mcpy(ip, h->addrs[0], 4); return 1; }
    }
    return -1;
}

/* the server named by Server=, resolved once; 0 = none set or not found */
static int server_addr(void)
{
    char host[128];
    int  i, port = 2399;
    if (g_srv_ip) return 1;
    if (!g_server[0]) return 0;
    for (i = 0; g_server[i] && g_server[i] != ':' && i < 127; i++) host[i] = g_server[i];
    host[i] = 0;
    if (g_server[i] == ':') {
        port = 0;
        for (i++; g_server[i] >= '0' && g_server[i] <= '9'; i++) port = port * 10 + (g_server[i] - '0');
    }
    if (!parse_ip(host, &g_srv_ip)) {
        HOSTENT *h = gethostbyname(host);
        if (!h || !h->addrs || !h->addrs[0]) { g_srv_ip = 0; return 0; }
        mcpy(&g_srv_ip, h->addrs[0], 4);
    }
    g_srv_port = hton16((WORD)port);
    return 1;
}

/* the address this machine reaches the server from, and the port we are bound to:
 * what a player on the same network would use to reach us */
static void find_lan(Peer *p)
{
    SOCKADDR_IN a;
    int    al = sizeof a;
    SOCKET s;
    p->lan_ip = 0;
    mzero(&a, sizeof a);
    if (getsockname(p->sock, &a, &al) == 0) p->lan_port = a.port;
    if (!g_srv_ip) return;
    s = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (s == INVALID_SOCKET) return;
    mzero(&a, sizeof a);
    a.family = AF_INET; a.port = g_srv_port; a.addr = g_srv_ip;
    al = sizeof a;
    if (connect(s, &a, sizeof a) == 0 && getsockname(s, &a, &al) == 0) p->lan_ip = a.addr;
    closesocket(s);
}

/* ---- the event queue (delivered by the network thread, in order) --------- */

static Ev *ev_new(int kind, const void *data, int len)
{
    Ev *e = (Ev *)mem((int)sizeof(Ev) + (len > 0 ? len : 0));
    if (!e) return 0;
    e->kind = kind;
    e->len = len > 0 ? len : 0;
    if (data && len > 0) mcpy(e->data, data, len);
    return e;
}

/* caller holds p->cs */
static void ev_push(Peer *p, Ev *e)
{
    if (!e) return;
    if (p->in_tail) p->in_tail->next = e; else p->in_head = e;
    p->in_tail = e;
}

/* ---- players and connections (caller holds p->cs) ------------------------ */

static Player *player_of(Peer *p, DPNID id)
{
    int i;
    for (i = 0; i < MAX_PLAYERS; i++)
        if (p->players[i].used && p->players[i].id == id) return &p->players[i];
    return 0;
}

static Player *player_add(Peer *p, DPNID id, DWORD flags, const WCHAR *name, int conn)
{
    int i;
    Player *pl = player_of(p, id);
    if (pl) return pl;
    for (i = 0; i < MAX_PLAYERS; i++)
        if (!p->players[i].used) {
            pl = &p->players[i];
            mzero(pl, sizeof *pl);
            pl->used = 1; pl->id = id; pl->flags = flags; pl->conn = conn;
            wcopy(pl->name, name, NAME_CHARS);
            return pl;
        }
    return 0;
}

static Conn *conn_find(Peer *p, DWORD ip, WORD port)
{
    int i;
    for (i = 0; i < MAX_CONNS; i++)
        if (p->conns[i].used && p->conns[i].ip == ip && p->conns[i].port == port) return &p->conns[i];
    return 0;
}

static Conn *conn_new(Peer *p, DWORD ip, WORD port)
{
    int i;
    for (i = 0; i < MAX_CONNS; i++)
        if (!p->conns[i].used) {
            Conn *c = &p->conns[i];
            mzero(c, sizeof *c);
            c->used = 1; c->ip = ip; c->port = port;
            c->last_rx = c->last_tx = GetTickCount();
            c->rto = 200;
            return c;
        }
    return 0;
}

static void conn_free(Conn *c)
{
    int i;
    while (c->unacked) { Pkt *n = c->unacked->next; unmem(c->unacked); c->unacked = n; }
    for (i = 0; i < WINDOW; i++) unmem(c->ooo[i]);
    unmem(c->msg);
    mzero(c, sizeof *c);
}

static int hdr(BYTE *b, int type) { b[0] = 'A'; b[1] = '2'; b[2] = 'O'; b[3] = 1; b[4] = (BYTE)type; return 5; }

static void raw_send(Peer *p, DWORD ip, WORD port, const BYTE *d, int n)
{
    SOCKADDR_IN to;
    mzero(&to, sizeof to);
    to.family = AF_INET; to.port = port; to.addr = ip;
    p->stat_sent++;
    if (g_loss) {
        g_rand = g_rand * 1103515245u + 12345u;
        if ((int)((g_rand >> 16) % 100) < g_loss) { p->stat_dropped++; return; }
    }
    if (is_relayed(ip)) {
        BYTE w[1400];                      /* the server takes datagrams up to 1400 bytes */
        int  k;
        if (!g_srv_ip || n + 9 > (int)sizeof w) return;
        k = hdr(w, SV_RELAY);
        put32(w + k, relay_id(ip));
        mcpy(w + k + 4, d, n);
        to.port = g_srv_port; to.addr = g_srv_ip;
        sendto(p->sock, (const char *)w, k + 4 + n, 0, &to, sizeof to);
        return;
    }
    sendto(p->sock, (const char *)d, n, 0, &to, sizeof to);
}


/* one reliable message on a connection, fragmented */
static void rel_send(Peer *p, Conn *c, const BYTE *d, int n)
{
    int off = 0;
    do {
        int   chunk = n - off > FRAG ? FRAG : n - off;
        Pkt  *k = (Pkt *)mem(sizeof(Pkt));
        if (!k) return;
        k->seq = c->next_seq++;
        k->len = hdr(k->data, P_DATA);
        put32(k->data + k->len, k->seq); k->len += 4;
        k->data[k->len++] = (BYTE)(off + chunk < n);         /* more follows */
        mcpy(k->data + k->len, d + off, chunk); k->len += chunk;
        k->sent = k->first = GetTickCount();
        k->tries = 1;
        if (c->last) c->last->next = k; else c->unacked = k;
        c->last = k;
        c->inflight++;
        raw_send(p, c->ip, c->port, k->data, k->len);
        c->last_tx = k->sent;
        off += chunk;
    } while (off < n);
}

static int put_desc(Peer *p, BYTE *b)
{
    int n = 0;
    put32(b + n, p->sflags); n += 4;
    mcpy(b + n, &p->sinst, 16); n += 16;
    mcpy(b + n, &p->sapp, 16);  n += 16;
    put32(b + n, p->smax); n += 4;
    put32(b + n, p->scur); n += 4;
    n += put_w(b + n, p->sname);
    b[n++] = (BYTE)(p->spass[0] != 0);
    return n;
}

static int get_desc(const BYTE *b, int avail, AppDesc *d, WCHAR *name)
{
    int n = 0;
    if (avail < 46) return -1;
    mzero(d, sizeof *d);
    d->size = sizeof *d;
    d->flags = get32(b); n += 4;
    mcpy(&d->instance, b + n, 16); n += 16;
    mcpy(&d->application, b + n, 16); n += 16;
    d->max = get32(b + n); n += 4;
    d->cur = get32(b + n); n += 4;
    n += get_w(b + n, avail - n, name);
    d->name = name;
    if (n < avail && b[n]) d->flags |= DPNSESSION_REQUIREPASSWORD;
    return n + 1;
}

/* host: every joiner but `except` */
static void to_joiners(Peer *p, const BYTE *d, int n, DPNID except)
{
    int i;
    for (i = 0; i < MAX_CONNS; i++) {
        Conn *c = &p->conns[i];
        if (c->used && c->open && c->player != except) rel_send(p, c, d, n);
    }
}

static int put_player(BYTE *b, const Player *pl)
{
    put32(b, pl->id);
    put32(b + 4, pl->flags & DPNPLAYER_HOST);
    return 8 + put_w(b + 8, pl->name);
}

/* ---- losing players and the session (caller holds p->cs) ----------------- */

static void destroy_player(Peer *p, DPNID id, DWORD reason)
{
    Player *pl = player_of(p, id);
    Ev *e;
    if (!pl) return;
    e = ev_new(EV_DESTROY, 0, 0);
    if (e) { e->a = id; e->b = reason; ev_push(p, e); }
    /* the context goes with the event; the slot is freed once it is delivered */
}

static void rules_recompute(Peer *p, Conn *only);
static void rules_reset(Peer *p);

static void end_session(Peer *p, HRESULT hr, DWORD reason)
{
    int i;
    Ev *e = ev_new(EV_TERMINATE, 0, 0);
    if (e) { e->a = (DWORD)hr; ev_push(p, e); }
    for (i = 0; i < MAX_PLAYERS; i++)
        if (p->players[i].used) destroy_player(p, p->players[i].id, reason);
    for (i = 0; i < MAX_CONNS; i++) if (p->conns[i].used) conn_free(&p->conns[i]);
    rules_reset(p);
}

static void conn_lost(Peer *p, Conn *c, DWORD reason)
{
    char b[80];
    b[0] = 0; s_cat(b, reason == DPNDESTROYPLAYERREASON_NORMAL ? "left: " : "lost: "); s_ip(b, c->ip, c->port);
    plog(p, b);
    if (p->role == ROLE_HOST) {
        BYTE m[16];
        DPNID id = c->player;
        Player *pl = player_of(p, id);
        if (pl) pl->conn = -1;
        conn_free(c);
        if (id) {
            if (p->scur) p->scur--;
            destroy_player(p, id, reason);
            m[0] = R_PLAYER_DEL; put32(m + 1, id); put32(m + 5, reason);
            to_joiners(p, m, 9, 0);
            rules_recompute(p, 0);
        }
    } else if (p->role == ROLE_JOINER) {
        end_session(p, reason == DPNDESTROYPLAYERREASON_NORMAL ? DPNERR_HOSTTERMINATEDSESSION
                                                               : DPNERR_CONNECTIONLOST,
                    reason == DPNDESTROYPLAYERREASON_NORMAL ? DPNDESTROYPLAYERREASON_SESSIONTERMINATED
                                                            : DPNDESTROYPLAYERREASON_CONNECTIONLOST);
    }
}

static void say(Peer *p, int chat, const char *text, int sticky);

/* ---- QOLRules: agreeing on the rules (caller holds p->cs) -------------------
 * QOLRules.asi (qol/) changes when the bank is charged, which every node of a
 * network game simulates, so it may run only if every player runs it.  Each joiner
 * tells the host whether it does (R_RULES, right after it is accepted); the host
 * answers every joiner, and again on every change, with whether all of them
 * do, itself included.  The stream is ordered, so the answer reaches a joiner
 * before the host's launch of the game.  QOLRules latches it at its first use in
 * a game; the session ending clears it. */
typedef int  (*RulesWantedFn)(void);
typedef void (*RulesSetFn)(int);

static void *rules_proc(const char *name)
{
    HMODULE m = GetModuleHandleA("QOLRules.asi");
    return m ? (void *)GetProcAddress(m, name) : 0;
}

static int rules_wanted(void)
{
    RulesWantedFn f = (RulesWantedFn)rules_proc("QOLRules_Wanted");
    return f ? f() : 0;
}

static void rules_apply(Peer *p, int all)
{
    RulesSetFn f = (RulesSetFn)rules_proc("QOLRules_Network");
    if (f) f(all);
    if (all != p->rules_all && (all || rules_wanted()))
        say(p, CHAT_GAME, all ? "Orders are paid when queued in this game."
                              : "Orders are paid as they start: not everyone has QOLRules.", 0);
    if (all != p->rules_all)
        plog(p, all ? "rules: QOLRules on for every player" : "rules: stock (not every player has QOLRules)");
    p->rules_all = all;
}

static void rules_reset(Peer *p)
{
    void (*f)(void) = (void (*)(void))rules_proc("QOLRules_Reset");
    if (f) f();
    p->rules_all = 0;
}

/* host: the answer, to every joiner when it changed, and to `only` in any case */
static void rules_recompute(Peer *p, Conn *only)
{
    BYTE m[2];
    int  i, all = rules_wanted(), was = p->rules_all;
    for (i = 0; i < MAX_CONNS; i++)
        if (p->conns[i].used && p->conns[i].open && !p->conns[i].rules) all = 0;
    rules_apply(p, all);
    m[0] = R_RULES; m[1] = (BYTE)all;
    if (all != was) to_joiners(p, m, 2, 0);
    else if (only) rel_send(p, only, m, 2);
}

/* ---- one reliable message in (caller holds p->cs) ------------------------- */

static void on_message(Peer *p, Conn *c, const BYTE *m, int n)
{
    Ev *e;
    if (n < 1) return;
    switch (m[0]) {
    case R_ACCEPT: {                             /* joiner: we are in */
        AppDesc d; WCHAR name[NAME_CHARS];
        int off, k, count;
        DPNID me, host;
        if (p->role != ROLE_JOINER || !p->cn_on || n < 13 || get32(m + 9) != p->cn_nonce) return;
        me = get32(m + 1); host = get32(m + 5);
        off = 13;
        k = get_desc(m + off, n - off, &d, name);
        if (k < 0) return;
        off += k;
        p->sflags = d.flags; p->sinst = d.instance; p->sapp = d.application;
        p->smax = d.max; p->scur = d.cur; wcopy(p->sname, name, NAME_CHARS);
        p->self = me; p->host = host;
        c->open = 1; c->player = host;
        player_add(p, me, DPNPLAYER_LOCAL, p->myname, -1)->ctx = p->cn_pctx;
        e = ev_new(EV_CREATE, 0, 0); e->a = me; ev_push(p, e);
        count = off + 2 <= n ? (int)get16(m + off) : 0;
        off += 2;
        while (count-- > 0 && off + 10 <= n) {
            DPNID id = get32(m + off);
            DWORD fl = get32(m + off + 4);
            off += 8;
            off += get_w(m + off, n - off, name);
            if (id == me) continue;
            player_add(p, id, fl, name, -1);
            e = ev_new(EV_CREATE, 0, 0); e->a = id; ev_push(p, e);
        }
        e = ev_new(EV_CONNECT_DONE, off < n ? m + off : 0, off < n ? n - off : 0);
        e->a = S_OK; e->b = p->cn_h; e->c = me; e->ctx = p->cn_ctx;
        ev_push(p, e);
        p->cn_on = 0;
        plog(p, "connected");
        {
            BYTE r[2];
            r[0] = R_RULES; r[1] = (BYTE)rules_wanted();
            rel_send(p, c, r, 2);
        }
        break;
    }
    case R_APP: {
        DPNID from, to;
        if (n < 9) return;
        from = get32(m + 1); to = get32(m + 5);
        if (p->role == ROLE_HOST) {
            from = c->player;                    /* a joiner speaks only for itself */
            put32((BYTE *)m + 1, from);
            if (to == DPNID_ALL_PLAYERS_GROUP) to_joiners(p, m, n, from);
            else if (to != p->self) {
                Player *t = player_of(p, to);
                if (t && t->conn >= 0 && p->conns[t->conn].open) rel_send(p, &p->conns[t->conn], m, n);
                return;
            }
        }
        e = ev_new(EV_RECEIVE, m + 9, n - 9);
        if (e) { e->a = from; ev_push(p, e); }
        break;
    }
    case R_PLAYER_ADD: {
        WCHAR name[NAME_CHARS];
        if (p->role != ROLE_JOINER || n < 11) return;
        get_w(m + 9, n - 9, name);
        if (!player_of(p, get32(m + 1)) && player_add(p, get32(m + 1), get32(m + 5), name, -1)) {
            e = ev_new(EV_CREATE, 0, 0); e->a = get32(m + 1); ev_push(p, e);
            p->scur++;
        }
        break;
    }
    case R_PLAYER_DEL:
        if (p->role != ROLE_JOINER || n < 9) return;
        if (p->scur) p->scur--;
        destroy_player(p, get32(m + 1), get32(m + 5));
        break;
    case R_INFO: {
        WCHAR name[NAME_CHARS];
        Player *pl;
        DPNID id;
        if (n < 7) return;
        id = p->role == ROLE_HOST ? c->player : get32(m + 1);
        get_w(m + 5, n - 5, name);
        pl = player_of(p, id);
        if (!pl) return;
        wcopy(pl->name, name, NAME_CHARS);
        if (p->role == ROLE_HOST) { put32((BYTE *)m + 1, id); to_joiners(p, m, n, id); }
        e = ev_new(EV_PEER_INFO, 0, 0); e->a = id; ev_push(p, e);
        break;
    }
    case R_APPDESC: {
        AppDesc d; WCHAR name[NAME_CHARS];
        if (p->role != ROLE_JOINER || get_desc(m + 1, n - 1, &d, name) < 0) return;
        p->sflags = d.flags; p->smax = d.max; p->scur = d.cur; wcopy(p->sname, name, NAME_CHARS);
        ev_push(p, ev_new(EV_APPDESC, 0, 0));
        break;
    }
    case R_RULES:
        if (n < 2) return;
        if (p->role == ROLE_HOST) { c->rules = m[1] != 0; rules_recompute(p, c); }
        else if (p->role == ROLE_JOINER) rules_apply(p, m[1] != 0);
        break;
    case R_TERMINATE:
        if (p->role != ROLE_JOINER) return;
        plog(p, "the host ended the session");
        end_session(p, DPNERR_HOSTTERMINATEDSESSION, DPNDESTROYPLAYERREASON_SESSIONTERMINATED);
        break;
    }
}

/* ---- the server and the direct path (caller holds p->cs) ----------------- */

static void say(Peer *p, int chat, const char *text, int sticky)
{
    Ev *e = ev_new(EV_NOTICE, text, s_len(text) + 1);
    if (e) { e->a = (DWORD)chat; e->b = (DWORD)sticky; ev_push(p, e); }
}

static void to_server(Peer *p, const BYTE *d, int n) { raw_send(p, g_srv_ip, g_srv_port, d, n); }

/* [pair][0 = probe, 1 = answer] */
static void send_punch(Peer *p, DWORD ip, WORD port, DWORD pair, int answer)
{
    BYTE b[16];
    int  k = hdr(b, P_PUNCH);
    put32(b + k, pair); b[k + 4] = (BYTE)answer;
    raw_send(p, ip, port, b, k + 5);
}

/* the joiner has a way to the host: the search starts there */
static void use_path(Peer *p, DWORD ip, WORD port, const char *how)
{
    char b[120];
    p->jn_state = 3;
    p->en_ip = ip; p->en_port = port; p->en_next = GetTickCount();
    b[0] = 0; s_cat(b, "join code "); s_code(b, p->jn_code); s_cat(b, ": "); s_cat(b, how);
    s_cat(b, " "); s_ip(b, ip, port);
    plog(p, b);
}

static void add_candidate(Peer *p, DWORD ip, WORD port)
{
    int i;
    if (!ip || ip == 0xffffffff) return;
    for (i = 0; i < p->jn_n; i++) if (p->jn_ip[i] == ip && p->jn_port[i] == port) return;
    if (p->jn_n < 4) { p->jn_ip[p->jn_n] = ip; p->jn_port[p->jn_n] = port; p->jn_n++; }
}

static void on_server(Peer *p, const BYTE *b, int n, int type)
{
    char t[200];
    int  i;
    t[0] = 0;
    switch (type) {
    case SV_HOSTED:                          /* [token][code 6][public ip 4, port 2][our id] */
        if (p->role != ROLE_HOST || !p->sv_state || n < 20 || get32(b) != p->sv_token) break;
        if (p->sv_state != 2 || !(p->code[0] == b[4] && p->code[5] == b[9])) {
            DWORD ip; WORD port;
            for (i = 0; i < 6; i++) p->code[i] = (char)b[4 + i];
            p->code[6] = 0;
            mcpy(&ip, b + 10, 4); mcpy(&port, b + 14, 2);
            s_cat(t, "join code "); s_code(t, p->code); s_cat(t, ", seen by the server as ");
            s_ip(t, ip, port);
            plog(p, t);
            t[0] = 0; s_cat(t, "Join code: "); s_code(t, p->code);
            say(p, CHAT_GAME, t, 1);
        }
        p->sv_state = 2; p->sv_warned = 0;
        p->sv_next = GetTickCount() + REGISTER_MS;
        break;
    case SV_PEER:                            /* [nonce][host public 6][host lan 6][host id][our id][pair] */
        if (p->jn_state != 1 || n < 28 || get32(b) != p->jn_nonce) break;
        {
            DWORD ip; WORD port;
            p->jn_n = 0;
            mcpy(&ip, b + 4, 4);  mcpy(&port, b + 8, 2);  add_candidate(p, ip, port);
            mcpy(&ip, b + 10, 4); mcpy(&port, b + 14, 2); add_candidate(p, ip, port);
            p->jn_host_id = get32(b + 16); p->jn_pair = get32(b + 24);
            s_cat(t, "join code "); s_code(t, p->jn_code); s_cat(t, ": host at ");
            s_ip(t, p->jn_ip[0], p->jn_port[0]);
            if (p->jn_n > 1) { s_cat(t, ", on its network "); s_ip(t, p->jn_ip[1], p->jn_port[1]); }
            plog(p, t);
        }
        if (!g_direct) { use_path(p, relayed_ip(p->jn_host_id), VPORT, "Direct=0, through the relay to"); break; }
        p->jn_state = 2; p->jn_start = p->jn_next = GetTickCount();
        break;
    case SV_INTRO:                           /* [joiner public 6][joiner lan 6][joiner id][pair] */
        if (p->role != ROLE_HOST || n < 20) break;
        for (i = 0; i < MAX_CONNS; i++)
            if (!p->punch[i].pair || GetTickCount() - p->punch[i].start > ANSWER_MS) break;
        if (i == MAX_CONNS) break;
        p->punch[i].pair = get32(b + 16);
        mcpy(&p->punch[i].ip[0], b, 4);     mcpy(&p->punch[i].port[0], b + 4, 2);
        mcpy(&p->punch[i].ip[1], b + 6, 4); mcpy(&p->punch[i].port[1], b + 10, 2);
        p->punch[i].start = p->punch[i].next = GetTickCount();
        s_cat(t, "a joiner is coming: "); s_ip(t, p->punch[i].ip[0], p->punch[i].port[0]);
        s_cat(t, ", on its network "); s_ip(t, p->punch[i].ip[1], p->punch[i].port[1]);
        s_cat(t, ", or relay #"); s_num(t, (long)get32(b + 12));
        plog(p, t);
        break;
    case SV_NOTFOUND:
        if (p->jn_state != 1 || n < 4 || get32(b) != p->jn_nonce) break;
        p->jn_state = 0;
        s_cat(t, "No game with the join code "); s_code(t, p->jn_code);
        plog(p, t);
        say(p, CHAT_ROOM, t, 0);
        break;
    case SV_ERROR:
        s_cat(t, "Online server: ");
        if (n > 4) { int k = s_len(t); for (i = 4; i < n && k < 190; i++) t[k++] = (char)b[i]; t[k] = 0; }
        plog(p, t);
        say(p, p->role == ROLE_HOST ? CHAT_GAME : CHAT_ROOM, t, 0);
        break;
    }
}

static void on_punch(Peer *p, DWORD ip, WORD port, const BYTE *b, int n)
{
    DWORD pair;
    int   i;
    if (n < 5 || !g_direct) return;
    pair = get32(b);
    if (p->jn_state == 2 && pair == p->jn_pair) {
        if (b[4]) { use_path(p, ip, port, "direct to the host at"); return; }
        send_punch(p, ip, port, pair, 1);
        add_candidate(p, ip, port);          /* the host as its NAT shows it to us: probe it too */
        return;
    }
    for (i = 0; i < MAX_CONNS; i++)
        if (p->punch[i].pair == pair && GetTickCount() - p->punch[i].start < ANSWER_MS) {
            if (!b[4]) send_punch(p, ip, port, pair, 1);
            return;
        }
}

/* ---- one datagram in (caller holds p->cs) ---------------------------------- */

static void send_ack(Peer *p, Conn *c)
{
    BYTE  b[16];
    DWORD sack = 0;
    int   i, n = hdr(b, P_ACK);
    for (i = 0; i < 32; i++)
        if (c->ooo[(c->expect + 1 + i) % WINDOW]) sack |= 1u << i;
    put32(b + n, c->expect); put32(b + n + 4, sack);
    raw_send(p, c->ip, c->port, b, n + 8);
}

/* one in-order DATA packet's payload: append, and hand on a finished message.
 * Returns 0 if the message ended the connection. */
static int take(Peer *p, Conn *c, const BYTE *d, int n)
{
    if (c->msg_len + n - 5 > c->msg_cap) {
        int   cap = c->msg_len + n + 2048;
        BYTE *g = (BYTE *)mem(cap);
        if (!g) return 1;
        mcpy(g, c->msg, c->msg_len);
        unmem(c->msg);
        c->msg = g; c->msg_cap = cap;
    }
    mcpy(c->msg + c->msg_len, d + 5, n - 5);
    c->msg_len += n - 5;
    c->expect++;
    if (!d[4]) {                                     /* the last fragment */
        int len = c->msg_len;
        c->msg_len = 0;
        on_message(p, c, c->msg, len);
        return c->used;
    }
    return 1;
}

/* d: seq u32, more u8, payload */
static void on_data(Peer *p, Conn *c, const BYTE *d, int n)
{
    DWORD seq;
    if (n < 5) return;
    seq = get32(d);
    if (seq != c->expect) {                          /* early: keep it; late: a duplicate */
        if (seq - c->expect < WINDOW && !c->ooo[seq % WINDOW]) {
            BYTE *k = (BYTE *)mem(n);
            if (k) { mcpy(k, d, n); c->ooo[seq % WINDOW] = k; c->ooo_len[seq % WINDOW] = n; }
        }
        return;
    }
    if (!take(p, c, d, n)) return;
    while (c->ooo[c->expect % WINDOW]) {
        int   slot = (int)(c->expect % WINDOW);
        BYTE *k = c->ooo[slot];
        int   ok;
        c->ooo[slot] = 0;
        ok = take(p, c, k, c->ooo_len[slot]);
        unmem(k);
        if (!ok) return;
    }
}

static void on_ack(Peer *p, Conn *c, DWORD next, DWORD sack)
{
    Pkt *k = c->unacked, *prev = 0;
    DWORD now = GetTickCount();
    (void)p;
    while (k) {
        Pkt *nx = k->next;
        DWORD d = k->seq - next;
        int acked = (int)(next - k->seq) > 0 || (d >= 1 && d <= 32 && (sack >> (d - 1)) & 1);
        if (acked) {
            if (k->tries == 1) {
                DWORD r = now - k->first;
                c->rtt = c->rtt ? (c->rtt * 7 + r) / 8 : r;
                c->rto = c->rtt * 2 + 50;
                if (c->rto < 100) c->rto = 100;
                if (c->rto > 2000) c->rto = 2000;
            }
            if (prev) prev->next = nx; else c->unacked = nx;
            if (c->last == k) c->last = prev;
            unmem(k);
            c->inflight--;
        } else prev = k;
        k = nx;
    }
}

static void on_conn_request(Peer *p, DWORD ip, WORD port, const BYTE *d, int n)
{
    BYTE  r[32];
    WCHAR pw[NAME_CHARS];
    HRESULT why = 0;
    int   i, off;
    Ev   *e;

    if (n < 24 || conn_find(p, ip, port)) return;       /* a retry of one already in */
    for (i = 0; i < MAX_CONNS; i++)
        if (p->pend_ip[i] == ip && p->pend_port[i] == port) return;   /* already asked */
    off = 20;
    off += get_w(d + off, n - off, pw);
    if (p->role != ROLE_HOST) why = DPNERR_NOTHOST;
    else if (!guid_eq((const GUID *)(d + 4), &p->sapp)) why = DPNERR_HOSTREJECTEDCONNECTION;
    else if (p->scur >= p->smax) why = DPNERR_SESSIONFULL;
    else if (p->spass[0] && !weq(pw, p->spass)) why = DPNERR_INVALIDPASSWORD;
    if (why) {
        int k = hdr(r, P_REJECT);
        put32(r + k, get32(d)); put32(r + k + 4, (DWORD)why);
        raw_send(p, ip, port, r, k + 8);
        return;
    }
    for (i = 0; i < MAX_CONNS; i++)
        if (!p->pend_ip[i]) { p->pend_ip[i] = ip; p->pend_port[i] = port; break; }
    /* the rest (nonce, name, user data) goes to the game with the event */
    e = ev_new(EV_INDICATE, d, n);
    if (e) { e->a = ip; e->b = port; ev_push(p, e); }
}

static void on_packet(Peer *p, DWORD ip, WORD port, const BYTE *b, int n)
{
    Conn *c;
    Ev   *e;
    if (n < 5 || b[0] != 'A' || b[1] != '2' || b[2] != 'O' || b[3] != 1) return;
    if (b[4] >= SV_HOST && ip == g_srv_ip && port == g_srv_port) {
        if (b[4] == SV_RELAYED) {
            if (n >= 9 + 5) on_packet(p, relayed_ip(get32(b + 5)), VPORT, b + 9, n - 9);
        } else on_server(p, b + 5, n - 5, b[4]);
        return;
    }
    p->stat_recv++;
    c = conn_find(p, ip, port);
    if (c) c->last_rx = GetTickCount();
    b += 5; n -= 5;
    switch (b[-1]) {
    case P_ENUM_Q:
        if (p->role == ROLE_HOST && n >= 20 && guid_eq((const GUID *)b, &p->sapp)) {
            e = ev_new(EV_ENUM_QUERY, b + 20, n - 20);
            if (e) { e->a = ip; e->b = port; e->c = get32(b + 16); ev_push(p, e); }
        }
        break;
    case P_ENUM_R:
        if (p->en_on && n >= 4) {
            e = ev_new(EV_ENUM_RESPONSE, b + 4, n - 4);
            if (e) { e->a = ip; e->b = port; e->c = GetTickCount() - get32(b); ev_push(p, e); }
        }
        break;
    case P_CONN:
        on_conn_request(p, ip, port, b, n);
        break;
    case P_REJECT:
        if (p->cn_on && c && n >= 8 && get32(b) == p->cn_nonce) {
            e = ev_new(EV_CONNECT_DONE, 0, 0);
            e->a = get32(b + 4); e->b = p->cn_h; e->ctx = p->cn_ctx;
            ev_push(p, e);
            p->cn_on = 0;
            conn_free(c);
            plog(p, "refused by the host");
        }
        break;
    case P_DATA:
        if (c) { on_data(p, c, b, n); if (c->used) send_ack(p, c); }
        break;
    case P_ACK:
        if (c && n >= 8) on_ack(p, c, get32(b), get32(b + 4));
        break;
    case P_PING:
        if (c) { BYTE r[8]; raw_send(p, ip, port, r, hdr(r, P_PONG)); }
        break;
    case P_PONG:                                    /* last_rx is all it is for */
        break;
    case P_BYE:
        if (c) conn_lost(p, c, DPNDESTROYPLAYERREASON_NORMAL);
        break;
    case P_PUNCH:
        on_punch(p, ip, port, b, n);
        break;
    }
}

/* ---- timers (caller holds p->cs) ----------------------------------------- */

static void tick_server(Peer *p, DWORD now)
{
    BYTE b[64];
    int  i, k, j;

    if (p->sv_state && (int)(now - p->sv_next) >= 0) {     /* host: get, then keep, the code */
        k = hdr(b, SV_HOST);
        put32(b + k, p->sv_token); mcpy(b + k + 4, &p->lan_ip, 4); mcpy(b + k + 8, &p->lan_port, 2);
        to_server(p, b, k + 10);
        p->sv_next = now + (p->sv_state == 2 ? REGISTER_MS : 1000);
        if (p->sv_state == 1 && now - p->sv_start > LOOKUP_MS) {
            p->sv_next = now + 5000;
            if (!p->sv_warned) {
                char t[200];
                t[0] = 0; s_cat(t, "The online server "); s_cat(t, g_server);
                s_cat(t, " does not answer: no join code. Players can still join by address.");
                plog(p, t);
                say(p, CHAT_GAME, t, 0);
                p->sv_warned = 1;
            }
        }
    }
    for (i = 0; i < MAX_CONNS; i++)                         /* host: probe the joiners coming */
        if (p->punch[i].pair && g_direct && (int)(now - p->punch[i].next) >= 0
            && now - p->punch[i].start < PUNCH_MS) {
            for (j = 0; j < 2; j++)
                if (p->punch[i].ip[j]) send_punch(p, p->punch[i].ip[j], p->punch[i].port[j], p->punch[i].pair, 0);
            p->punch[i].next = now + PUNCH_EVERY;
        }
    if (p->jn_state == 1 && (int)(now - p->jn_next) >= 0) {    /* joiner: look the code up */
        if (now - p->jn_start > LOOKUP_MS) {
            char t[200];
            t[0] = 0; s_cat(t, "The online server "); s_cat(t, g_server); s_cat(t, " does not answer");
            plog(p, t);
            say(p, CHAT_ROOM, t, 0);
            p->jn_state = 0;
        } else {
            k = hdr(b, SV_JOIN);
            put32(b + k, p->jn_nonce); mcpy(b + k + 4, p->jn_code, 6);
            mcpy(b + k + 10, &p->lan_ip, 4); mcpy(b + k + 14, &p->lan_port, 2);
            to_server(p, b, k + 16);
            p->jn_next = now + 500;
        }
    }
    if (p->jn_state == 2 && (int)(now - p->jn_next) >= 0) {    /* joiner: probe the host */
        if (now - p->jn_start > PUNCH_MS)
            use_path(p, relayed_ip(p->jn_host_id), VPORT, "no direct path, through the relay to");
        else {
            for (j = 0; j < p->jn_n; j++) send_punch(p, p->jn_ip[j], p->jn_port[j], p->jn_pair, 0);
            p->jn_next = now + PUNCH_EVERY;
        }
    }
}

static void tick(Peer *p)
{
    DWORD now = GetTickCount();
    int   i;

    tick_server(p, now);

    if (p->en_on && p->en_ip && (int)(now - p->en_next) >= 0) {
        if (p->en_left == 0 || (p->en_end && (int)(now - p->en_end) >= 0)) {
            Ev *e = ev_new(EV_ASYNC_DONE, 0, 0);
            e->a = p->en_h; e->b = S_OK; e->ctx = p->en_ctx;
            ev_push(p, e);
            p->en_on = 0;
        } else {
            put32(p->en_pkt + 5 + 16, now);
            raw_send(p, p->en_ip, p->en_port, p->en_pkt, p->en_len);
            if (p->en_left != 0xffffffff) p->en_left--;
            p->en_next = now + p->en_retry;
        }
    }
    if (p->cn_on && (int)(now - p->cn_next) >= 0) {
        Conn *c = &p->conns[0];
        if (now - p->cn_start > CONNECT_MS) {
            Ev *e = ev_new(EV_CONNECT_DONE, 0, 0);
            e->a = DPNERR_NORESPONSE; e->b = p->cn_h; e->ctx = p->cn_ctx;
            ev_push(p, e);
            p->cn_on = 0;
            if (c->used) conn_free(c);
            plog(p, "no answer from the host");
        } else if (c->used) {
            raw_send(p, c->ip, c->port, p->cn_pkt, p->cn_len);
            p->cn_next = now + 500;
        }
    }
    for (i = 0; i < MAX_CONNS; i++) {
        Conn *c = &p->conns[i];
        Pkt  *k;
        if (!c->used || (!c->open && p->role != ROLE_HOST)) continue;
        if (now - c->last_rx > LOST_MS) { conn_lost(p, c, DPNDESTROYPLAYERREASON_CONNECTIONLOST); continue; }
        for (k = c->unacked; k; k = k->next)
            if (now - k->sent > c->rto * (DWORD)(k->tries > 6 ? 6 : k->tries)) {
                raw_send(p, c->ip, c->port, k->data, k->len);
                k->sent = now; k->tries++;
                c->last_tx = now;
                p->stat_resent++;
            }
        if (now - c->last_tx > PING_MS) {
            BYTE r[8];
            raw_send(p, c->ip, c->port, r, hdr(r, P_PING));
            c->last_tx = now;
        }
    }
}

/* ---- delivery to the game (never with p->cs held) ------------------------ */

static HRESULT deliver(Peer *p, DWORD type, void *msg)
{
    return p->handler ? p->handler(p->hctx, type, msg) : S_OK;
}

static void *ctx_of(Peer *p, DPNID id)
{
    Player *pl;
    void *ctx;
    EnterCriticalSection(&p->cs);
    pl = player_of(p, id);
    ctx = pl ? pl->ctx : 0;
    LeaveCriticalSection(&p->cs);
    return ctx;
}

static void accept_joiner(Peer *p, Ev *e, void *pctx, const void *reply, DWORD reply_size)
{
    BYTE    m[2048];
    WCHAR   name[NAME_CHARS], pw[NAME_CHARS];
    Conn   *c;
    Player *pl;
    DPNID   id;
    int     n, i, off = 20, cnt = 0, cntpos;
    MsgPlayer cp;

    off += get_w(e->data + off, e->len - off, pw);
    get_w(e->data + off, e->len - off, name);

    EnterCriticalSection(&p->cs);
    c = conn_new(p, e->a, (WORD)e->b);
    id = p->next_id++;
    pl = c ? player_add(p, id, 0, name, (int)(c - p->conns)) : 0;
    if (!pl) { LeaveCriticalSection(&p->cs); return; }
    pl->ctx = pctx;
    c->player = id; c->open = 1;
    p->scur++;
    m[0] = R_ACCEPT; put32(m + 1, id); put32(m + 5, p->self); put32(m + 9, get32(e->data));
    n = 13 + put_desc(p, m + 13);
    cntpos = n; n += 2;
    for (i = 0; i < MAX_PLAYERS; i++)
        if (p->players[i].used && n < 1800) { n += put_player(m + n, &p->players[i]); cnt++; }
    put16(m + cntpos, (DWORD)cnt);
    if (reply && reply_size && n + (int)reply_size < (int)sizeof m) { mcpy(m + n, reply, (int)reply_size); n += (int)reply_size; }
    rel_send(p, c, m, n);
    m[0] = R_PLAYER_ADD;
    n = 1 + put_player(m + 1, pl);
    to_joiners(p, m, n, id);
    rules_recompute(p, c);               /* until it reports, it counts as without */
    LeaveCriticalSection(&p->cs);

    {
        char b[120];
        b[0] = 0; s_cat(b, "joined: player "); s_hex(b, id); s_cat(b, " from "); s_ip(b, e->a, (WORD)e->b);
        plog(p, b);
    }
    cp.size = sizeof cp; cp.id = id; cp.ctx = pctx;
    deliver(p, DPN_MSGID_CREATE_PLAYER, &cp);
    EnterCriticalSection(&p->cs);
    if ((pl = player_of(p, id))) pl->ctx = cp.ctx;
    LeaveCriticalSection(&p->cs);
}

static void deliver_event(Peer *p, Ev *e)
{
    switch (e->kind) {
    case EV_RECEIVE: {
        MsgReceive m;
        mzero(&m, sizeof m);
        m.size = sizeof m; m.from = e->a; m.ctx = ctx_of(p, e->a);
        m.data = e->data; m.data_size = (DWORD)e->len;
        m.buf = (DPNHANDLE)e;
        if (deliver(p, DPN_MSGID_RECEIVE, &m) == DPNSUCCESS_PENDING) {
            e->kind = 0;                               /* the game keeps it: ReturnBuffer frees */
            return;
        }
        break;
    }
    case EV_SEND_DONE: {
        MsgSendDone m;
        mzero(&m, sizeof m);
        m.size = sizeof m; m.op = e->a; m.ctx = e->ctx; m.flags = DPNSENDCOMPLETE_GUARANTEED;
        deliver(p, DPN_MSGID_SEND_COMPLETE, &m);
        break;
    }
    case EV_CREATE: {
        MsgPlayer m;
        Player *pl;
        m.size = sizeof m; m.id = e->a; m.ctx = ctx_of(p, e->a);
        deliver(p, DPN_MSGID_CREATE_PLAYER, &m);
        EnterCriticalSection(&p->cs);
        if ((pl = player_of(p, e->a))) pl->ctx = m.ctx;
        LeaveCriticalSection(&p->cs);
        break;
    }
    case EV_DESTROY: {
        MsgDestroy m;
        Player *pl;
        m.size = sizeof m; m.id = e->a; m.ctx = ctx_of(p, e->a); m.reason = e->b;
        deliver(p, DPN_MSGID_DESTROY_PLAYER, &m);
        EnterCriticalSection(&p->cs);
        if ((pl = player_of(p, e->a))) pl->used = 0;
        LeaveCriticalSection(&p->cs);
        break;
    }
    case EV_CONNECT_DONE: {
        MsgConnectDone m;
        mzero(&m, sizeof m);
        m.size = sizeof m; m.op = e->b; m.ctx = e->ctx; m.hr = (HRESULT)e->a;
        m.reply = e->len ? e->data : 0; m.reply_size = (DWORD)e->len; m.local = e->c;
        deliver(p, DPN_MSGID_CONNECT_COMPLETE, &m);
        break;
    }
    case EV_ASYNC_DONE: {
        MsgAsyncDone m;
        m.size = sizeof m; m.op = e->a; m.ctx = e->ctx; m.hr = (HRESULT)e->b;
        deliver(p, DPN_MSGID_ASYNC_OP_COMPLETE, &m);
        break;
    }
    case EV_ENUM_RESPONSE: {
        MsgEnumResponse m;
        AppDesc d;
        WCHAR   name[NAME_CHARS];
        int     k = get_desc(e->data, e->len, &d, name);
        if (k < 0 || !p->en_on) break;
        mzero(&m, sizeof m);
        m.size = sizeof m;
        m.sender = make_addr(e->a, (WORD)e->b);
        m.device = make_addr(0, 0);
        m.desc = &d;
        m.data = e->len > k ? e->data + k : 0; m.data_size = (DWORD)(e->len - k);
        m.ctx = p->en_ctx; m.rtt = e->c;
        if (m.sender) deliver(p, DPN_MSGID_ENUM_HOSTS_RESPONSE, &m);
        drop_addr(m.sender); drop_addr(m.device);
        break;
    }
    case EV_ENUM_QUERY: {
        MsgEnumQuery m;
        BYTE r[1200];
        int  n;
        HRESULT hr;
        mzero(&m, sizeof m);
        m.size = sizeof m;
        m.sender = make_addr(e->a, (WORD)e->b);
        m.device = make_addr(0, 0);
        m.data = e->len ? e->data : 0; m.data_size = (DWORD)e->len;
        m.max_reply = 872;
        hr = deliver(p, DPN_MSGID_ENUM_HOSTS_QUERY, &m);
        drop_addr(m.sender); drop_addr(m.device);
        if (hr == S_OK) {
            n = hdr(r, P_ENUM_R);
            put32(r + n, e->c); n += 4;                  /* the asker's time stamp, back */
            EnterCriticalSection(&p->cs);
            n += put_desc(p, r + n);
            LeaveCriticalSection(&p->cs);
            if (m.reply && m.reply_size && n + (int)m.reply_size <= (int)sizeof r) {
                mcpy(r + n, m.reply, (int)m.reply_size); n += (int)m.reply_size;
            }
            raw_send(p, e->a, (WORD)e->b, r, n);
        }
        if (m.reply) {
            MsgReturnBuffer rb;
            rb.size = sizeof rb; rb.hr = S_OK; rb.buf = m.reply; rb.ctx = m.reply_ctx;
            deliver(p, DPN_MSGID_RETURN_BUFFER, &rb);
        }
        break;
    }
    case EV_INDICATE: {
        MsgIndicate m;
        WCHAR w[NAME_CHARS];
        int   off = 20, i;
        HRESULT hr;
        off += get_w(e->data + off, e->len - off, w);
        off += get_w(e->data + off, e->len - off, w);
        mzero(&m, sizeof m);
        m.size = sizeof m;
        m.data = e->len > off ? e->data + off : 0; m.data_size = (DWORD)(e->len - off);
        m.addr = make_addr(e->a, (WORD)e->b);
        m.device = make_addr(0, 0);
        hr = deliver(p, DPN_MSGID_INDICATE_CONNECT, &m);
        drop_addr(m.addr); drop_addr(m.device);
        EnterCriticalSection(&p->cs);
        for (i = 0; i < MAX_CONNS; i++)
            if (p->pend_ip[i] == e->a && p->pend_port[i] == (WORD)e->b) p->pend_ip[i] = 0;
        LeaveCriticalSection(&p->cs);
        if (hr == S_OK) accept_joiner(p, e, m.player_ctx, m.reply, m.reply_size);
        else {
            BYTE r[1200];
            int  n = hdr(r, P_REJECT);
            put32(r + n, get32(e->data)); put32(r + n + 4, (DWORD)DPNERR_HOSTREJECTEDCONNECTION);
            raw_send(p, e->a, (WORD)e->b, r, n + 8);
            plog(p, "the game refused a joiner");
        }
        if (m.reply) {
            MsgReturnBuffer rb;
            rb.size = sizeof rb; rb.hr = S_OK; rb.buf = m.reply; rb.ctx = m.reply_ctx;
            deliver(p, DPN_MSGID_RETURN_BUFFER, &rb);
        }
        break;
    }
    case EV_TERMINATE: {
        MsgTerminate m;
        mzero(&m, sizeof m);
        m.size = sizeof m; m.hr = (HRESULT)e->a;
        deliver(p, DPN_MSGID_TERMINATE_SESSION, &m);
        break;
    }
    case EV_PEER_INFO: {
        MsgPlayer m;
        m.size = sizeof m; m.id = e->a; m.ctx = ctx_of(p, e->a);
        deliver(p, DPN_MSGID_PEER_INFO, &m);
        break;
    }
    case EV_APPDESC:
        deliver(p, DPN_MSGID_APPLICATION_DESC, 0);
        break;
    case EV_NOTICE:
        notice((int)e->a, (const char *)e->data, (int)e->b);
        break;
    }
    unmem(e);
}

static void drain(Peer *p)
{
    for (;;) {
        Ev *e;
        EnterCriticalSection(&p->cs);
        e = p->in_head;
        if (e) { p->in_head = e->next; if (!p->in_head) p->in_tail = 0; }
        LeaveCriticalSection(&p->cs);
        if (!e) return;
        deliver_event(p, e);
    }
}

/* ---- the network thread ----------------------------------------------------- */

static DWORD __stdcall net_thread(void *arg)
{
    Peer *p = (Peer *)arg;
    BYTE  b[2048];
    CoInitializeEx(0, COINIT_MULTITHREADED);       /* the address objects are COM */
    while (!p->stop) {
        FD_SET  r;
        TIMEVAL tv;
        r.count = 1; r.fd[0] = p->sock;
        tv.sec = 0; tv.usec = 10000;
        if (select(0, &r, 0, 0, &tv) > 0) {
            for (;;) {
                SOCKADDR_IN from;
                int fl = sizeof from;
                int n = recvfrom(p->sock, (char *)b, sizeof b, 0, &from, &fl);
                if (n <= 0) break;
                EnterCriticalSection(&p->cs);
                on_packet(p, from.addr, from.port, b, n);
                LeaveCriticalSection(&p->cs);
                drain(p);
            }
        }
        EnterCriticalSection(&p->cs);
        tick(p);
        LeaveCriticalSection(&p->cs);
        drain(p);
    }
    CoUninitialize();
    return 0;
}

/* (re)bind the socket to `port` (host order, 0 = any) and run the thread */
static HRESULT net_open(Peer *p, WORD port)
{
    SOCKADDR_IN a;
    DWORD on = 1;
    int   big = 1 << 20;
    char  b[80];

    if (p->sock != INVALID_SOCKET && (port == 0 || port == p->bound)) return S_OK;
    if (p->thread) {
        p->stop = 1;
        if (GetCurrentThreadId() != p->thread_id) WaitForSingleObject(p->thread, 5000);
        CloseHandle(p->thread);
        p->thread = 0;
        p->stop = 0;
    }
    if (p->sock != INVALID_SOCKET) closesocket(p->sock);
    p->sock = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (p->sock == INVALID_SOCKET) return DPNERR_GENERIC;
    ioctlsocket(p->sock, FIONBIO, &on);
    setsockopt(p->sock, SOL_SOCKET, SO_RCVBUF, (const char *)&big, 4);
    setsockopt(p->sock, SOL_SOCKET, SO_SNDBUF, (const char *)&big, 4);
    mzero(&a, sizeof a);
    a.family = AF_INET; a.port = hton16(port);
    b[0] = 0;
    if (bind(p->sock, &a, sizeof a) != 0) {
        s_cat(b, "cannot bind UDP port "); s_num(b, port); s_cat(b, ", error "); s_num(b, WSAGetLastError());
        plog(p, b);
        closesocket(p->sock);
        p->sock = INVALID_SOCKET;
        return DPNERR_ADDRESSING;
    }
    {
        int al = sizeof a;
        getsockname(p->sock, &a, &al);
    }
    p->bound = port;
    s_cat(b, "UDP port "); s_num(b, hton16(a.port));
    plog(p, b);
    p->thread = CreateThread(0, 0, net_thread, p, 0, &p->thread_id);
    return p->thread ? S_OK : DPNERR_GENERIC;
}

/* ---- IDirectPlay8Peer ------------------------------------------------------ */

#define PEER(x) Peer *p = (Peer *)(x)

static HRESULT __stdcall n_QueryInterface(void *self, const GUID *iid, void **out)
{
    static const GUID IID_IUnknown = { 0, 0, 0, { 0xc0, 0, 0, 0, 0, 0, 0, 0x46 } };
    if (!out) return E_POINTER;
    if (guid_eq(iid, &IID_IDirectPlay8Peer) || guid_eq(iid, &IID_IUnknown)) {
        *out = self;
        __sync_fetch_and_add(&((Peer *)self)->refs, 1);
        return S_OK;
    }
    *out = 0;
    return E_NOINTERFACE;
}

static ULONG __stdcall n_AddRef(void *self) { return (ULONG)__sync_add_and_fetch(&((Peer *)self)->refs, 1); }

static HRESULT __stdcall n_Close(void *self, DWORD flags);

static ULONG __stdcall n_Release(void *self)
{
    LONG r = __sync_sub_and_fetch(&((Peer *)self)->refs, 1);
    if (r == 0) n_Close(self, 0);
    return (ULONG)(r < 0 ? 0 : r);          /* the object itself is never freed: a late
                                               callback from its thread must not crash */
}

static HRESULT __stdcall n_Initialize(void *self, void *ctx, MsgHandler_t handler, DWORD flags)
{
    PEER(self);
    (void)flags;
    if (p->handler) return DPNERR_ALREADYINITIALIZED;
    p->handler = handler; p->hctx = ctx;
    p->closed = 0;
    return S_OK;
}

static HRESULT __stdcall n_EnumHosts(void *self, AppDesc *desc, void *host, void *device,
                                     void *data, DWORD size, DWORD count, DWORD retry,
                                     DWORD timeout, void *ctx, DPNHANDLE *h, DWORD flags)
{
    PEER(self);
    DWORD ip = 0; WORD port = 0;
    int   t;
    char  b[160], code[8];
    HRESULT hr;
    (void)device; (void)flags;

    if (!p->handler) return DPNERR_UNINITIALIZED;
    if ((hr = net_open(p, 0)) < 0) return hr;
    t = addr_target(host, &ip, &port, code);
    if (t < 0) return DPNERR_ADDRESSING;
    if (t == 2 && !server_addr()) {
        b[0] = 0;
        s_cat(b, g_server[0] ? "The online server cannot be found: " : "No online server is set (Server= in Online.ini)");
        s_cat(b, g_server);
        plog(p, b);
        notice(CHAT_ROOM, b, 0);
        t = 0;                                         /* searches nothing, as with no host typed */
    }
    if (t == 2) find_lan(p);
    EnterCriticalSection(&p->cs);
    p->en_on = 1; p->en_h = ++p->next_handle; p->en_ctx = ctx;
    p->en_ip = t == 1 ? ip : 0; p->en_port = port;
    p->jn_state = 0;
    if (t == 2) {
        mcpy(p->jn_code, code, 7);
        g_rand = g_rand * 1103515245u + 12345u;
        p->jn_nonce = g_rand ^ GetTickCount();
        p->jn_state = 1; p->jn_start = p->jn_next = GetTickCount();
    }
    p->en_left = count ? count : 0xffffffff;
    /* the game passes 0, "the provider's default", and drops a listed game it has not
     * heard from for a few seconds ("The host of this game has been lost"): at
     * DirectPlay's 1.5 s, two lost round trips in a row lose the game */
    p->en_retry = retry && retry != 0xffffffff ? retry : 500;
    p->en_end = timeout && timeout != 0xffffffff ? GetTickCount() + timeout + p->en_retry * (count ? count : 1) : 0;
    p->en_next = GetTickCount();
    p->en_len = hdr(p->en_pkt, P_ENUM_Q);
    mcpy(p->en_pkt + p->en_len, desc ? &desc->application : &p->sapp, 16); p->en_len += 16;
    put32(p->en_pkt + p->en_len, 0); p->en_len += 4;          /* time stamp, set per send */
    if (data && size && size <= 200) { mcpy(p->en_pkt + p->en_len, data, (int)size); p->en_len += (int)size; }
    if (h) *h = p->en_h;
    LeaveCriticalSection(&p->cs);
    b[0] = 0;
    if (t == 2) { s_cat(b, "looking up join code "); s_code(b, code); s_cat(b, " at "); s_cat(b, g_server); }
    else if (t) { s_cat(b, "looking for a game at "); s_ip(b, ip, port); }
    else s_cat(b, "no host typed: not looking");
    plog(p, b);
    return DPNSUCCESS_PENDING;
}

static HRESULT __stdcall n_CancelAsyncOperation(void *self, DPNHANDLE h, DWORD flags)
{
    PEER(self);
    MsgAsyncDone m;
    int was;
    (void)flags;
    EnterCriticalSection(&p->cs);
    was = p->en_on && (h == 0 || h == p->en_h);
    if (was) { p->en_on = 0; p->jn_state = 0; }
    LeaveCriticalSection(&p->cs);
    if (was) {
        m.size = sizeof m; m.op = p->en_h; m.ctx = p->en_ctx; m.hr = DPNERR_USERCANCEL;
        deliver(p, DPN_MSGID_ASYNC_OP_COMPLETE, &m);
    }
    return S_OK;
}

static HRESULT __stdcall n_Connect(void *self, const AppDesc *desc, void *host, void *device,
                                   void *sec, void *cred, const void *data, DWORD size,
                                   void *pctx, void *ctx, DPNHANDLE *h, DWORD flags)
{
    PEER(self);
    DWORD ip; WORD port;
    Conn *c;
    int   n;
    char  b[120];
    HRESULT hr;
    (void)device; (void)sec; (void)cred; (void)flags;

    if (!p->handler) return DPNERR_UNINITIALIZED;
    if (addr_target(host, &ip, &port, 0) != 1) return DPNERR_ADDRESSING;
    if ((hr = net_open(p, 0)) < 0) return hr;
    EnterCriticalSection(&p->cs);
    p->role = ROLE_JOINER;
    if (desc) { p->sapp = desc->application; wcopy(p->spass, desc->password, NAME_CHARS); }
    c = &p->conns[0];
    if (c->used) conn_free(c);
    mzero(c, sizeof *c);
    c->used = 1; c->ip = ip; c->port = port;
    c->last_rx = c->last_tx = GetTickCount(); c->rto = 200;
    p->cn_on = 1; p->cn_h = ++p->next_handle; p->cn_ctx = ctx; p->cn_pctx = pctx;
    p->cn_nonce = GetTickCount() ^ (DWORD)(UINT)p;
    p->cn_start = GetTickCount(); p->cn_next = p->cn_start;
    n = hdr(p->cn_pkt, P_CONN);
    put32(p->cn_pkt + n, p->cn_nonce); n += 4;
    mcpy(p->cn_pkt + n, &p->sapp, 16); n += 16;
    n += put_w(p->cn_pkt + n, p->spass);
    n += put_w(p->cn_pkt + n, p->myname);
    if (data && size && n + (int)size <= (int)sizeof p->cn_pkt) { mcpy(p->cn_pkt + n, data, (int)size); n += (int)size; }
    p->cn_len = n;
    if (h) *h = p->cn_h;
    LeaveCriticalSection(&p->cs);
    b[0] = 0; s_cat(b, "connecting to "); s_ip(b, ip, port);
    plog(p, b);
    return DPNSUCCESS_PENDING;
}

static HRESULT __stdcall n_SendTo(void *self, DPNID to, const BufDesc *bufs, DWORD nbufs, DWORD timeout,
                                  void *ctx, DPNHANDLE *h, DWORD flags)
{
    PEER(self);
    BYTE   stack[2048];
    BYTE  *m = stack;
    DWORD  i, len = 9;
    DPNHANDLE handle;
    Player *t;
    (void)timeout;

    if (!p->handler || p->closed) return DPNERR_UNINITIALIZED;
    if (p->role == ROLE_NONE) return DPNERR_NOTREADY;
    for (i = 0; i < nbufs; i++) len += bufs[i].size;
    if (len > sizeof stack && !(m = (BYTE *)mem((int)len))) return DPNERR_OUTOFMEMORY;
    m[0] = R_APP; put32(m + 1, p->self); put32(m + 5, to);
    for (len = 9, i = 0; i < nbufs; i++) { mcpy(m + len, bufs[i].data, (int)bufs[i].size); len += bufs[i].size; }

    EnterCriticalSection(&p->cs);
    t = to == DPNID_ALL_PLAYERS_GROUP ? 0 : player_of(p, to);
    if (to != DPNID_ALL_PLAYERS_GROUP && !t) {
        LeaveCriticalSection(&p->cs);
        if (m != stack) unmem(m);
        return DPNERR_INVALIDPLAYER;
    }
    if (to == p->self || (to == DPNID_ALL_PLAYERS_GROUP && !(flags & DPNSEND_NOLOOPBACK))) {
        Ev *e = ev_new(EV_RECEIVE, m + 9, (int)len - 9);
        if (e) { e->a = p->self; ev_push(p, e); }
    }
    if (to != p->self) {
        if (p->role == ROLE_JOINER) { if (p->conns[0].open) rel_send(p, &p->conns[0], m, (int)len); }
        else if (to == DPNID_ALL_PLAYERS_GROUP) to_joiners(p, m, (int)len, 0);
        else if (t->conn >= 0 && p->conns[t->conn].open) rel_send(p, &p->conns[t->conn], m, (int)len);
    }
    handle = ++p->next_handle;
    if (!(flags & (DPNSEND_NOCOMPLETE | DPNSEND_SYNC))) {
        Ev *e = ev_new(EV_SEND_DONE, 0, 0);
        if (e) { e->a = handle; e->ctx = ctx; ev_push(p, e); }
    }
    LeaveCriticalSection(&p->cs);
    if (m != stack) unmem(m);
    if (flags & DPNSEND_SYNC) return S_OK;
    if (h) *h = handle;
    return DPNSUCCESS_PENDING;
}

static HRESULT __stdcall n_Host(void *self, const AppDesc *desc, void **devs, DWORD ndevs,
                                void *sec, void *cred, void *pctx, DWORD flags)
{
    PEER(self);
    MsgPlayer m;
    Player   *pl;
    HRESULT   hr;
    char      b[160];
    (void)devs; (void)ndevs; (void)sec; (void)cred; (void)flags;

    if (!p->handler) return DPNERR_UNINITIALIZED;
    if (!desc) return DPNERR_INVALIDPARAM;
    if ((hr = net_open(p, g_port)) < 0) return hr;
    EnterCriticalSection(&p->cs);
    p->en_on = 0;
    p->role = ROLE_HOST;
    p->self = p->host = 0x00a20001;
    p->next_id = p->self + 1;
    p->sflags = desc->flags; p->sapp = desc->application;
    p->sinst.d1 = GetTickCount(); p->sinst.d2 = 0xa2; p->sinst.d3 = (WORD)(UINT)p;
    p->smax = desc->max ? desc->max : 8; p->scur = 1;
    wcopy(p->sname, desc->name, NAME_CHARS);
    wcopy(p->spass, desc->password, NAME_CHARS);
    pl = player_add(p, p->self, DPNPLAYER_LOCAL | DPNPLAYER_HOST, p->myname, -1);
    if (pl) pl->ctx = pctx;
    LeaveCriticalSection(&p->cs);
    b[0] = 0; s_cat(b, "hosting \""); s_wide(b, p->sname, 60); s_cat(b, "\"");
    plog(p, b);
    if (server_addr()) {
        find_lan(p);
        EnterCriticalSection(&p->cs);
        g_rand = g_rand * 1103515245u + 12345u;
        p->sv_token = g_rand ^ GetTickCount();
        p->sv_state = 1; p->sv_warned = 0;
        p->sv_start = p->sv_next = GetTickCount();
        LeaveCriticalSection(&p->cs);
        b[0] = 0; s_cat(b, "asking "); s_cat(b, g_server); s_cat(b, " for a join code");
        plog(p, b);
    } else {
        b[0] = 0;
        s_cat(b, g_server[0] ? "The online server cannot be found: " : "No online server is set (Server= in Online.ini)");
        s_cat(b, g_server);
        s_cat(b, ". Players can join by address.");
        plog(p, b);
        notice(CHAT_GAME, b, 0);
    }

    m.size = sizeof m; m.id = p->self; m.ctx = pctx;
    deliver(p, DPN_MSGID_CREATE_PLAYER, &m);
    EnterCriticalSection(&p->cs);
    if ((pl = player_of(p, p->self))) pl->ctx = m.ctx;
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_GetApplicationDesc(void *self, AppDesc *out, DWORD *size, DWORD flags)
{
    PEER(self);
    DWORD need;
    (void)flags;
    if (!size) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    need = sizeof(AppDesc) + 2 * (DWORD)(wlen(p->sname) + 1) + 2 * (DWORD)(wlen(p->spass) + 1);
    if (!out || *size < need) { *size = need; LeaveCriticalSection(&p->cs); return DPNERR_BUFFERTOOSMALL; }
    mzero(out, sizeof *out);
    out->size = sizeof *out;
    out->flags = p->sflags; out->instance = p->sinst; out->application = p->sapp;
    out->max = p->smax; out->cur = p->scur;
    out->name = (WCHAR *)(out + 1);
    wcopy(out->name, p->sname, NAME_CHARS);
    out->password = out->name + wlen(p->sname) + 1;
    wcopy(out->password, p->spass, NAME_CHARS);
    if (!p->spass[0]) out->password = 0;
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_SetApplicationDesc(void *self, const AppDesc *desc, DWORD flags)
{
    PEER(self);
    BYTE m[300];
    int  n;
    (void)flags;
    if (!desc) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    if (p->role != ROLE_HOST) { LeaveCriticalSection(&p->cs); return DPNERR_NOTHOST; }
    p->sflags = desc->flags;
    if (desc->max) p->smax = desc->max;
    wcopy(p->sname, desc->name, NAME_CHARS);
    wcopy(p->spass, desc->password, NAME_CHARS);
    m[0] = R_APPDESC;
    n = 1 + put_desc(p, m + 1);
    to_joiners(p, m, n, 0);
    ev_push(p, ev_new(EV_APPDESC, 0, 0));
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_SetPeerInfo(void *self, const PlayerInfo *info, void *ctx, DPNHANDLE *h, DWORD flags)
{
    PEER(self);
    (void)ctx; (void)flags;
    if (!info) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    if (info->info_flags & DPNINFO_NAME) {
        Player *pl = player_of(p, p->self);
        wcopy(p->myname, info->name, NAME_CHARS);
        if (pl && p->role != ROLE_NONE) {
            BYTE m[200];
            int  n;
            wcopy(pl->name, p->myname, NAME_CHARS);
            m[0] = R_INFO; put32(m + 1, p->self);
            n = 5 + put_w(m + 5, p->myname);
            if (p->role == ROLE_HOST) to_joiners(p, m, n, 0);
            else if (p->conns[0].open) rel_send(p, &p->conns[0], m, n);
        }
    }
    LeaveCriticalSection(&p->cs);
    if (h) *h = 0;
    return S_OK;                     /* DPNSETPEERINFO_SYNC or not: done at once */
}

static HRESULT __stdcall n_GetPeerInfo(void *self, DPNID id, PlayerInfo *out, DWORD *size, DWORD flags)
{
    PEER(self);
    Player *pl;
    DWORD   need;
    (void)flags;
    if (!size) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    pl = player_of(p, id);
    if (!pl) { LeaveCriticalSection(&p->cs); return DPNERR_INVALIDPLAYER; }
    need = sizeof(PlayerInfo) + 2 * (DWORD)(wlen(pl->name) + 1);
    if (!out || *size < need) { *size = need; LeaveCriticalSection(&p->cs); return DPNERR_BUFFERTOOSMALL; }
    out->size = sizeof *out;
    out->info_flags = DPNINFO_NAME | 0x0002;
    out->name = (WCHAR *)(out + 1);
    wcopy(out->name, pl->name, NAME_CHARS);
    out->data = 0; out->data_size = 0;
    out->player_flags = (id == p->self ? DPNPLAYER_LOCAL : 0) | (id == p->host ? DPNPLAYER_HOST : 0);
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_GetPeerAddress(void *self, DPNID id, void **addr, DWORD flags)
{
    PEER(self);
    Player *pl;
    DWORD ip = 0; WORD port = 0;
    (void)flags;
    if (!addr) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    pl = player_of(p, id);
    if (pl && p->role == ROLE_HOST && pl->conn >= 0) { ip = p->conns[pl->conn].ip; port = p->conns[pl->conn].port; }
    else if (pl && p->role == ROLE_JOINER && id == p->host) { ip = p->conns[0].ip; port = p->conns[0].port; }
    LeaveCriticalSection(&p->cs);
    if (!pl) return DPNERR_INVALIDPLAYER;
    *addr = make_addr(ip, port);
    return *addr ? S_OK : DPNERR_GENERIC;
}

static HRESULT __stdcall n_Close(void *self, DWORD flags)
{
    PEER(self);
    BYTE   bye[8];
    DPNID  ids[MAX_PLAYERS];
    void  *ctxs[MAX_PLAYERS];
    int    n = 0, i, k, en, cn;
    (void)flags;

    if (p->closed) return S_OK;
    p->closed = 1;
    if (p->thread) {
        p->stop = 1;
        if (GetCurrentThreadId() != p->thread_id) WaitForSingleObject(p->thread, 5000);
        CloseHandle(p->thread);
        p->thread = 0;
    }
    EnterCriticalSection(&p->cs);
    if (p->sv_state && p->sock != INVALID_SOCKET) {
        BYTE r[16];
        int  m = hdr(r, SV_BYE);
        put32(r + m, p->sv_token);
        to_server(p, r, m + 4);
        to_server(p, r, m + 4);
    }
    k = hdr(bye, P_BYE);
    for (i = 0; i < MAX_CONNS; i++)
        if (p->conns[i].used && p->sock != INVALID_SOCKET) {
            raw_send(p, p->conns[i].ip, p->conns[i].port, bye, k);
            raw_send(p, p->conns[i].ip, p->conns[i].port, bye, k);
            conn_free(&p->conns[i]);
        }
    for (i = 0; i < MAX_PLAYERS; i++)
        if (p->players[i].used) { ids[n] = p->players[i].id; ctxs[n] = p->players[i].ctx; n++; p->players[i].used = 0; }
    while (p->in_head) { Ev *e = p->in_head; p->in_head = e->next; unmem(e); }
    p->in_tail = 0;
    if (p->sock != INVALID_SOCKET) { closesocket(p->sock); p->sock = INVALID_SOCKET; }
    {
        char b[120];
        b[0] = 0; s_cat(b, "closed; datagrams sent "); s_num(b, (long)p->stat_sent);
        s_cat(b, ", received "); s_num(b, (long)p->stat_recv);
        s_cat(b, ", resent "); s_num(b, (long)p->stat_resent);
        if (g_loss) { s_cat(b, ", dropped on purpose "); s_num(b, (long)p->stat_dropped); }
        plog(p, b);
    }
    en = p->en_on; cn = p->cn_on;
    if (p->sv_state) notice_unstick();
    p->role = ROLE_NONE; p->en_on = 0; p->cn_on = 0; p->bound = 0;
    rules_reset(p);
    p->sv_state = 0; p->jn_state = 0;
    mzero(p->punch, sizeof p->punch);
    LeaveCriticalSection(&p->cs);
    /* what is still pending completes as cancelled, before any DESTROY_PLAYER:
     * the game frees its search on this message (without it, quitting crashed) */
    if (en) {
        MsgAsyncDone m;
        m.size = sizeof m; m.op = p->en_h; m.ctx = p->en_ctx; m.hr = DPNERR_USERCANCEL;
        deliver(p, DPN_MSGID_ASYNC_OP_COMPLETE, &m);
    }
    if (cn) {
        MsgConnectDone m;
        mzero(&m, sizeof m);
        m.size = sizeof m; m.op = p->cn_h; m.ctx = p->cn_ctx; m.hr = DPNERR_USERCANCEL;
        deliver(p, DPN_MSGID_CONNECT_COMPLETE, &m);
    }
    for (i = 0; i < n; i++) {
        MsgDestroy m;
        m.size = sizeof m; m.id = ids[i]; m.ctx = ctxs[i]; m.reason = DPNDESTROYPLAYERREASON_NORMAL;
        deliver(p, DPN_MSGID_DESTROY_PLAYER, &m);
    }
    p->handler = 0;
    p->stop = 0;
    return S_OK;
}

static HRESULT __stdcall n_DestroyPeer(void *self, DPNID id, const void *data, DWORD size, DWORD flags)
{
    PEER(self);
    Player *pl;
    (void)data; (void)size; (void)flags;
    EnterCriticalSection(&p->cs);
    if (p->role != ROLE_HOST) { LeaveCriticalSection(&p->cs); return DPNERR_NOTHOST; }
    pl = player_of(p, id);
    if (!pl || pl->conn < 0) { LeaveCriticalSection(&p->cs); return DPNERR_INVALIDPLAYER; }
    {
        BYTE m[8];
        Conn *c = &p->conns[pl->conn];
        m[0] = R_TERMINATE; put32(m + 1, (DWORD)DPNERR_HOSTTERMINATEDSESSION);
        rel_send(p, c, m, 5);
        c->player = 0;                   /* the BYE that follows is no second departure */
        if (p->scur) p->scur--;
        destroy_player(p, id, DPNDESTROYPLAYERREASON_HOSTDESTROYEDPLAYER);
        m[0] = R_PLAYER_DEL; put32(m + 1, id); put32(m + 5, DPNDESTROYPLAYERREASON_HOSTDESTROYEDPLAYER);
        to_joiners(p, m, 9, id);
        rules_recompute(p, 0);
    }
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_TerminateSession(void *self, const void *data, DWORD size, DWORD flags)
{
    PEER(self);
    BYTE m[8];
    (void)data; (void)size; (void)flags;
    EnterCriticalSection(&p->cs);
    if (p->role != ROLE_HOST) { LeaveCriticalSection(&p->cs); return DPNERR_NOTHOST; }
    m[0] = R_TERMINATE; put32(m + 1, (DWORD)DPNERR_HOSTTERMINATEDSESSION);
    to_joiners(p, m, 5, 0);
    end_session(p, DPNERR_HOSTTERMINATEDSESSION, DPNDESTROYPLAYERREASON_SESSIONTERMINATED);
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_ReturnBuffer(void *self, DPNHANDLE h, DWORD flags)
{
    (void)self; (void)flags;
    if (!h) return DPNERR_INVALIDHANDLE;
    unmem((void *)h);                    /* the Ev the RECEIVE pointed into */
    return S_OK;
}

static HRESULT __stdcall n_GetPlayerContext(void *self, DPNID id, void **ctx, DWORD flags)
{
    PEER(self);
    Player *pl;
    (void)flags;
    if (!ctx) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    pl = player_of(p, id);
    *ctx = pl ? pl->ctx : 0;
    LeaveCriticalSection(&p->cs);
    return pl ? S_OK : DPNERR_INVALIDPLAYER;
}

static HRESULT __stdcall n_EnumPlayersAndGroups(void *self, DPNID *ids, DWORD *count, DWORD flags)
{
    PEER(self);
    DWORD n = 0, i;
    (void)flags;
    if (!count) return DPNERR_INVALIDPARAM;
    EnterCriticalSection(&p->cs);
    for (i = 0; i < MAX_PLAYERS; i++) if (p->players[i].used) n++;
    if (!ids || *count < n) { *count = n; LeaveCriticalSection(&p->cs); return DPNERR_BUFFERTOOSMALL; }
    for (n = 0, i = 0; i < MAX_PLAYERS; i++) if (p->players[i].used) ids[n++] = p->players[i].id;
    *count = n;
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

static HRESULT __stdcall n_GetSPCaps(void *self, const GUID *sp, SpCaps *caps, DWORD flags)
{
    (void)self; (void)sp; (void)flags;
    if (!caps || caps->size < sizeof(SpCaps)) return DPNERR_INVALIDPARAM;
    caps->flags = 0; caps->threads = 1;
    /* DirectPlay's figures: the game ages its game list by them, so they stay, while
     * EnumHosts queries three times as often (n_EnumHosts) */
    caps->enum_count = 5; caps->enum_retry = 1500; caps->enum_timeout = 1500;
    caps->max_enum_payload = 983; caps->buffers = 4; caps->sysbuf = 0x2000;
    return S_OK;
}

static HRESULT __stdcall n_GetCaps(void *self, Caps *caps, DWORD flags)
{
    (void)self; (void)flags;
    if (!caps || caps->size < sizeof(Caps)) return DPNERR_INVALIDPARAM;
    caps->flags = 0; caps->connect_timeout = 200; caps->connect_retries = 14; caps->keepalive = 60000;
    return S_OK;
}

static HRESULT __stdcall n_GetSendQueueInfo(void *self, DPNID id, DWORD *msgs, DWORD *bytes, DWORD flags)
{
    PEER(self);
    Player *pl;
    (void)flags;
    EnterCriticalSection(&p->cs);
    pl = player_of(p, id);
    if (msgs) *msgs = 0;
    if (bytes) *bytes = 0;
    if (pl && msgs) {
        Conn *c = p->role == ROLE_JOINER ? &p->conns[0] : pl->conn >= 0 ? &p->conns[pl->conn] : 0;
        if (c) *msgs = (DWORD)c->inflight;
    }
    LeaveCriticalSection(&p->cs);
    return S_OK;
}

/* accepted and ignored; each with its own argument count, since __stdcall
 * pops what it was declared with */
static HRESULT __stdcall n_SetCaps(void *self, const Caps *caps, DWORD flags)
{ (void)self; (void)caps; (void)flags; return S_OK; }
static HRESULT __stdcall n_SetSPCaps(void *self, const GUID *sp, const SpCaps *caps, DWORD flags)
{ (void)self; (void)sp; (void)caps; (void)flags; return S_OK; }
static HRESULT __stdcall n_RegisterLobby(void *self, DPNHANDLE h, void *lobby, DWORD flags)
{ (void)self; (void)h; (void)lobby; (void)flags; return S_OK; }

/* slots nobody was seen calling: logged, and refused */
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-parameter"
#define UNSUPPORTED(slot, N) \
    static HRESULT __stdcall n_unsup_##slot(void *self PARAMS_##N) \
    { (void)self; plog((Peer *)self, "unsupported call: " #slot); return DPNERR_UNSUPPORTED; }

UNSUPPORTED(EnumServiceProviders, 6)
UNSUPPORTED(CreateGroup, 5)
UNSUPPORTED(DestroyGroup, 4)
UNSUPPORTED(AddPlayerToGroup, 5)
UNSUPPORTED(RemovePlayerFromGroup, 5)
UNSUPPORTED(SetGroupInfo, 5)
UNSUPPORTED(GetGroupInfo, 4)
UNSUPPORTED(EnumGroupMembers, 4)
UNSUPPORTED(GetLocalHostAddresses, 3)
UNSUPPORTED(GetGroupContext, 3)
UNSUPPORTED(GetConnectionInfo, 3)
#pragma clang diagnostic pop

static void peer_vtbl_init(void)
{
    void **v = g_peer_vtbl;
    v[S_QueryInterface]        = (void *)n_QueryInterface;
    v[S_AddRef]                = (void *)n_AddRef;
    v[S_Release]               = (void *)n_Release;
    v[S_Initialize]            = (void *)n_Initialize;
    v[S_EnumServiceProviders]  = (void *)n_unsup_EnumServiceProviders;
    v[S_CancelAsyncOperation]  = (void *)n_CancelAsyncOperation;
    v[S_Connect]               = (void *)n_Connect;
    v[S_SendTo]                = (void *)n_SendTo;
    v[S_GetSendQueueInfo]      = (void *)n_GetSendQueueInfo;
    v[S_Host]                  = (void *)n_Host;
    v[S_GetApplicationDesc]    = (void *)n_GetApplicationDesc;
    v[S_SetApplicationDesc]    = (void *)n_SetApplicationDesc;
    v[S_CreateGroup]           = (void *)n_unsup_CreateGroup;
    v[S_DestroyGroup]          = (void *)n_unsup_DestroyGroup;
    v[S_AddPlayerToGroup]      = (void *)n_unsup_AddPlayerToGroup;
    v[S_RemovePlayerFromGroup] = (void *)n_unsup_RemovePlayerFromGroup;
    v[S_SetGroupInfo]          = (void *)n_unsup_SetGroupInfo;
    v[S_GetGroupInfo]          = (void *)n_unsup_GetGroupInfo;
    v[S_EnumPlayersAndGroups]  = (void *)n_EnumPlayersAndGroups;
    v[S_EnumGroupMembers]      = (void *)n_unsup_EnumGroupMembers;
    v[S_SetPeerInfo]           = (void *)n_SetPeerInfo;
    v[S_GetPeerInfo]           = (void *)n_GetPeerInfo;
    v[S_GetPeerAddress]        = (void *)n_GetPeerAddress;
    v[S_GetLocalHostAddresses] = (void *)n_unsup_GetLocalHostAddresses;
    v[S_Close]                 = (void *)n_Close;
    v[S_EnumHosts]             = (void *)n_EnumHosts;
    v[S_DestroyPeer]           = (void *)n_DestroyPeer;
    v[S_ReturnBuffer]          = (void *)n_ReturnBuffer;
    v[S_GetPlayerContext]      = (void *)n_GetPlayerContext;
    v[S_GetGroupContext]       = (void *)n_unsup_GetGroupContext;
    v[S_GetCaps]               = (void *)n_GetCaps;
    v[S_SetCaps]               = (void *)n_SetCaps;
    v[S_SetSPCaps]             = (void *)n_SetSPCaps;
    v[S_GetSPCaps]             = (void *)n_GetSPCaps;
    v[S_GetConnectionInfo]     = (void *)n_unsup_GetConnectionInfo;
    v[S_RegisterLobby]         = (void *)n_RegisterLobby;
    v[S_TerminateSession]      = (void *)n_TerminateSession;
}

/* a new peer, as CoCreateInstance would return it */
static void *peer_new(void)
{
    static int wsa;
    Peer *p;
    if (!wsa) {
        WSADATA w;
        if (WSAStartup(0x0202, &w) != 0) return 0;
        wsa = 1;
        peer_vtbl_init();
    }
    p = (Peer *)mem(sizeof(Peer));
    if (!p) return 0;
    p->vtbl = g_peer_vtbl;
    p->refs = 1;
    p->sock = INVALID_SOCKET;
    InitializeCriticalSection(&p->cs);
    return p;
}
