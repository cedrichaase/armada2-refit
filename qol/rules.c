/*
 * QOLRules.asi -- rules changes for Star Trek: Armada II.  See qol/README.md, QOL-7.
 *
 * PayOnQueue=1: an item is paid for the moment it is queued, not when it starts
 * building.  An order the bank cannot cover is refused, so a queue only ever
 * holds what is already paid for; cancelling a queued item gives its cost back;
 * so does losing the building (destroyed, captured, assimilated).
 *
 * THIS IS NOT QOL.asi.  Charging earlier changes the bank, and every node of a
 * network game simulates the bank, so a player with this plugin and one without
 * would desync.  Until the online layer can agree on it when a game is set up,
 * the plugin applies its rules only in a game against the computer (single player
 * and skirmish) and stands down in any network game.
 *
 * HOW STOCK QUEUES (Producer; the class has no name for this in the log, so)
 * ---------------------------------------------------------------------------
 * A producer keeps its queue as a linked list at +0x270 (count at +0x274), at
 * most ten items of 16 bytes: the class to build, the previous and next item,
 * and an id (+0xc) that the delete order carries over the network.  The head is
 * the item being built once StartBuild has run: +0x254 is then its class and
 * +0x2a0 its id.  Costs leave the bank in StartBuild only, which refuses (and the
 * advisor says so, every frame) when the bank is short; CancelBuild gives an
 * item in progress back in full, and deleting or clearing a waiting item gives
 * nothing, because nothing was taken.  FinishBuild pops the head.
 *
 * WHAT THE PLUGIN DOES
 * --------------------
 * The "already paid" mark is bit 30 of the item's id: it travels with the item,
 * through the network order that deletes it, and dies with it, so there is no
 * table to go stale.  (The producer's id counter, +0x2a8, is raised by the bit
 * for the one Push that creates the item.)
 *   - The call in Handle_New_Command that queues a build order (0x431532) comes
 *     here.  If the bank covers the item it is charged exactly as StartBuild
 *     charges it and queued with the mark; else the order is dropped.  Other
 *     callers of Push (loading a save, the Borg stealing technology) are left
 *     alone: they queue and start at once and expect it to succeed.
 *   - StartBuild: a marked head is given back first, the mark is cleared, and
 *     stock charges it as it always did -- it cannot fail, and the advisor stays
 *     quiet.  Should it fail anyway the item is taken again and keeps its mark.
 *   - Pop, CancelBuild (waiting head), ActDeleteBuildQueueItem, ClearBuildQueue:
 *     a marked item that leaves the queue is given back.
 *   - ClearTeam (capture, assimilation) and DestroyShip on a producer: every
 *     marked item is given back, and the item in progress too -- in the old
 *     owner's bank, since ClearTeam runs before the team changes.
 *   - Save / Load: the marks are written into the producer's id counter (a field
 *     the stock file already holds) as a tag and a bitmask by position, and read
 *     back after Load.  A stock save has no tag and loads as unpaid queues, which
 *     stock rules charge as each item starts.
 *
 * Patched in memory only.  Every entry is checked against this build's bytes
 * first; a different Armada2.exe leaves the plugin out and says so in the log.
 */

typedef unsigned char       BYTE;
typedef unsigned long       DWORD;
typedef int                 BOOL;
typedef unsigned int        UINT;
typedef long                LONG;
typedef void               *HANDLE;
typedef HANDLE              HMODULE;
typedef const char         *LPCSTR;
typedef char               *LPSTR;

#define NULLPTR ((void *)0)
#define TRUE  1

int _fltused = 0;

#define GENERIC_WRITE          0x40000000
#define FILE_SHARE_READ        0x00000001
#define OPEN_ALWAYS            4
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
__declspec(dllimport) UINT    __stdcall GetPrivateProfileIntA(LPCSTR, LPCSTR, int, LPCSTR);

/* ---- Producer layout and the engine's own functions (Armada2.exe, patch 1.1) */

#define P_TEAMIDX   0xec     /* the owner's team number */
#define P_TEAM      0xf0     /* its Team* */
#define P_CUR       0x254    /* class being built, or 0 */
#define P_HEAD      0x270    /* queue head: item {class, prev, next, id} */
#define P_COUNT     0x274
#define P_CURID     0x2a0    /* id of the item in progress */
#define P_NEXTID    0x2a8    /* the next id to hand out */
#define Q_MAX       10
#define PAID        0x40000000   /* bit 30 of an item's id: paid at enqueue */
#define SAVE_TAG    0x10000000   /* bit 28 of the saved counter: marks follow in bits 16-25 */

#define IT_CLS(it)  (*(void **)(it))
#define IT_NEXT(it) (*(BYTE **)((it) + 8))
#define IT_ID(it)   (*(int *)((it) + 0xc))

#define G_TRANSPORT 0x76b8d4          /* Transport * */
#define VT_LOCAL         0x6b6d8c     /* TransportLocal: the campaign */
#define VT_LOCAL_INSTANT 0x6b6eb8     /* TransportLocalInstantAction: skirmish */

typedef int   (__thiscall *CostFn)(void *cls, int team);
typedef float (__thiscall *AddFn)(void *team, float v);
typedef void  (__thiscall *CrewFn)(void *team, float v);
typedef void  (__thiscall *OffFn)(void *team, int n);
typedef int   (__thiscall *VacFn)(void *team);
typedef void  (__thiscall *PushFn)(BYTE *p, void *cls);

#define COST_DIL  ((CostFn)0x4cddc0)   /* GameObjectClass::Get_Dilithium_Cost(team) */
#define COST_LAT  ((CostFn)0x4cde90)
#define COST_MET  ((CostFn)0x4cdfa0)
#define COST_BIO  ((CostFn)0x4ce070)
#define COST_OFF  ((CostFn)0x4ce140)
#define COST_CREW ((CostFn)0x4ce160)
#define ADD_DIL   ((AddFn)0x496e30)    /* Team::AddDilithium */
#define ADD_LAT   ((AddFn)0x496f20)
#define ADD_MET   ((AddFn)0x497010)
#define ADD_BIO   ((AddFn)0x496d40)
#define ADD_CREW  ((CrewFn)0x4974b0)
#define ENLIST    ((OffFn)0x498870)    /* Team::EnlistOfficers */
#define DISCHARGE ((OffFn)0x498890)
#define VACANCIES ((VacFn)0x497580)    /* Team::GetOfficerVacancies */
#define ENGINE_PUSH ((PushFn)0x4b7930) /* Producer::PushBuildQueueItem */
#define EPSILON   (*(volatile double *)0x6ae228)

typedef struct { int dil, lat, met, bio, crew, off; } Cost;

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static int  g_logging = 1;

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

static long f_round(float f)
{
    long r;
    __asm__ __volatile__("flds %1\n\tfistpl %0" : "=m"(r) : "m"(f));
    return r;
}

static void logline(const char *s)
{
    HANDLE h;
    DWORD  wrote;
    char   buf[512];

    if (!g_logging || !g_logpath[0]) return;
    h = CreateFileA(g_logpath, GENERIC_WRITE, FILE_SHARE_READ, NULLPTR,
                    OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULLPTR);
    if (h == INVALID_HANDLE_VALUE) return;
    SetFilePointer(h, 0, NULLPTR, FILE_END);
    buf[0] = 0;
    s_cat(buf, s);
    s_cat(buf, "\r\n");
    WriteFile(h, buf, (DWORD)s_len(buf), &wrote, NULLPTR);
    CloseHandle(h);
}

/* ---- costs and the bank ------------------------------------------------ */

static int g_pay;     /* PayOnQueue=1 */
static int g_trace;   /* Log=2: every hook, for finding what a path does */

static void trace(const char *what, const BYTE *p)
{
    char m[120];
    if (!g_trace) return;
    m[0] = 0;
    s_cat(m, "  . "); s_cat(m, what);
    s_cat(m, ": queue "); s_num(m, *(const int *)(p + P_COUNT));
    s_cat(m, *(void *const *)(p + P_CUR) ? ", one in progress" : ", none in progress");
    logline(m);
}

/* Only a game against the computer: both transports of the local kind. */
static int rules_on(void)
{
    BYTE *t = *(BYTE **)G_TRANSPORT;
    DWORD vt;
    if (!g_pay || !t) return 0;
    vt = *(DWORD *)t;
    return vt == VT_LOCAL || vt == VT_LOCAL_INSTANT;
}

static void cost_of(const BYTE *p, void *cls, Cost *c)
{
    int t = *(const int *)(p + P_TEAMIDX);
    c->dil  = COST_DIL(cls, t);
    c->lat  = COST_LAT(cls, t);
    c->met  = COST_MET(cls, t);
    c->bio  = COST_BIO(cls, t);
    c->crew = COST_CREW(cls, t);
    c->off  = COST_OFF(cls, t);
}

/* StartBuild's own test, field for field: a team with 0x16c clear counts as having
 * nothing; dilithium has a small margin; officers are vacancies, not a stock. */
static int afford(const BYTE *team, const Cost *c)
{
    float d = 0, m = 0, l = 0, b = 0, k = 0;
    if (*(const int *)(team + 0x16c)) {
        d = *(const float *)(team + 0x28);
        m = *(const float *)(team + 0xc0);
        l = *(const float *)(team + 0x74);
        b = *(const float *)(team + 0x168);
        k = *(const float *)(team + 0x10c);
    }
    if ((double)d + EPSILON < (double)c->dil) return 0;
    if ((float)c->crew > k) return 0;
    if ((float)c->lat > l) return 0;
    if ((float)c->met > m) return 0;
    if ((float)c->bio > b) return 0;
    if (c->off && c->off > VACANCIES((void *)team)) return 0;
    return 1;
}

/* sign -1 takes the cost out of the bank as StartBuild does, +1 puts it back as
 * CancelBuild does. */
static void charge(BYTE *team, const Cost *c, int sign)
{
    float s = (float)sign;
    ADD_DIL(team, s * (float)c->dil);
    ADD_MET(team, s * (float)c->met);
    ADD_LAT(team, s * (float)c->lat);
    if (sign < 0) ENLIST(team, c->off); else DISCHARGE(team, c->off);
    ADD_BIO(team, s * (float)c->bio);
    /* Crew is capped (+0x110) and AddCrew clamps a gain to it, or drops it when
     * the bank is already over: a start's give-back would then lose crew the
     * charge takes again.  A gain goes into the field itself. */
    if (sign < 0) ADD_CREW(team, -(float)c->crew);
    else *(float *)(team + 0x10c) += (float)c->crew;
}

static void note(const char *what, const BYTE *team, const Cost *c)
{
    char m[200];
    m[0] = 0;
    s_cat(m, what);
    s_cat(m, " dil "); s_num(m, c->dil);
    s_cat(m, " met "); s_num(m, c->met);
    s_cat(m, " lat "); s_num(m, c->lat);
    s_cat(m, " bio "); s_num(m, c->bio);
    s_cat(m, " crew "); s_num(m, c->crew);
    s_cat(m, " off "); s_num(m, c->off);
    s_cat(m, "; bank dil ");
    s_num(m, team ? f_round(*(const float *)(team + 0x28)) : 0);
    s_cat(m, " crew ");
    s_num(m, team ? f_round(*(const float *)(team + 0x10c)) : 0);
    s_cat(m, " officers free ");
    s_num(m, team ? VACANCIES((void *)team) : 0);
    logline(m);
}

/* Give a class's cost back to the producer's owner. */
static void give_back(BYTE *p, void *cls, const char *why)
{
    BYTE *team = *(BYTE **)(p + P_TEAM);
    Cost  c;
    if (!team || !cls) return;
    cost_of(p, cls, &c);
    charge(team, &c, +1);
    note(why, team, &c);
}

/* ---- the queue ---------------------------------------------------------- */

static int is_paid(const BYTE *it) { return it && (IT_ID(it) & PAID); }

/* Every marked item goes back, and the mark is cleared; with `cur`, the item in
 * progress too. */
static void drain(BYTE *p, int cur, const char *why)
{
    BYTE *it;
    for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it)) {
        if (!is_paid(it)) continue;
        give_back(p, IT_CLS(it), why);
        IT_ID(it) &= ~PAID;
    }
    if (cur && *(void **)(p + P_CUR)) give_back(p, *(void **)(p + P_CUR), why);
}

static int is_producer(const BYTE *o)
{
    /* ClearBuildQueue is Producer's alone; another Craft's table is shorter */
    return (*(const DWORD **)o)[0x13c / 4] == 0x4b78c0;
}

/* ---- hooks -------------------------------------------------------------- */

typedef BYTE (__thiscall *BoolFn)(BYTE *);
typedef void (__thiscall *VoidFn)(BYTE *);
typedef void (__thiscall *IntFn)(BYTE *, int);
typedef BYTE (__thiscall *PtrFn)(BYTE *, void *);

static BoolFn t_start, t_cancel;
static VoidFn t_pop, t_clearq, t_clearteam;
static IntFn  t_actdel, t_destroy;
static PtrFn  t_save, t_load;

/* in place of `call Producer::PushBuildQueueItem` in Handle_New_Command */
static void __thiscall h_push(BYTE *p, void *cls)
{
    BYTE *team = *(BYTE **)(p + P_TEAM);
    Cost  c;

    if (!cls || !team || !rules_on() || *(int *)(p + P_COUNT) >= Q_MAX) {
        ENGINE_PUSH(p, cls);
        return;
    }
    cost_of(p, cls, &c);
    if (!afford(team, &c)) {
        note("refused", team, &c);
        return;
    }
    charge(team, &c, -1);
    *(int *)(p + P_NEXTID) |= PAID;           /* this one id carries the mark */
    ENGINE_PUSH(p, cls);
    *(int *)(p + P_NEXTID) &= ~PAID;
    note("paid", team, &c);
}

static BYTE __thiscall h_start(BYTE *p)
{
    trace("StartBuild", p);
    BYTE *head = *(BYTE **)(p + P_HEAD);
    BYTE *team = *(BYTE **)(p + P_TEAM);
    Cost  c;
    BYTE  r;
    int   owed = !*(void **)(p + P_CUR) && is_paid(head) && team;

    if (owed) {
        cost_of(p, IT_CLS(head), &c);
        charge(team, &c, +1);                 /* stock charges it now, and cannot refuse */
        IT_ID(head) &= ~PAID;
    }
    r = t_start(p);
    if (owed && !r) {                         /* it did anyway: back to paid */
        charge(team, &c, -1);
        IT_ID(head) |= PAID;
    } else if (owed) {
        note("started", team, &c);
    }
    return r;
}

static void __thiscall h_pop(BYTE *p)
{
    trace("Pop", p);
    BYTE *head = *(BYTE **)(p + P_HEAD);
    void *cls = head ? IT_CLS(head) : NULLPTR;
    int   paid = is_paid(head);

    t_pop(p);
    if (paid && *(BYTE **)(p + P_HEAD) != head) give_back(p, cls, "popped");
}

static BYTE __thiscall h_cancel(BYTE *p)
{
    BYTE *head = *(BYTE **)(p + P_HEAD);
    BYTE *team = *(BYTE **)(p + P_TEAM);
    void *cls = head ? IT_CLS(head) : NULLPTR;
    void *cur = *(void **)(p + P_CUR);
    int   paid = !cur && is_paid(head);
    float crew0 = team ? *(float *)(team + 0x10c) : 0.0f;
    BYTE  r;

    trace("CancelBuild", p);
    r = t_cancel(p);
    if (paid && *(BYTE **)(p + P_HEAD) != head) {
        give_back(p, cls, "cancelled");
    } else if (cur && team && !*(void **)(p + P_CUR) && rules_on()) {
        /* the item in progress: stock gives it back in full except crew, which
         * AddCrew clamps to the cap -- what the cancel did not return is added */
        Cost  c;
        float got = *(float *)(team + 0x10c) - crew0;
        cost_of(p, cur, &c);
        if (got < (float)c.crew) *(float *)(team + 0x10c) += (float)c.crew - got;
        note("cancelled in progress", team, &c);
    }
    return r;
}

static void __thiscall h_actdel(BYTE *p, int id)
{
    trace("ActDeleteBuildQueueItem", p);
    BYTE *it;
    void *cls = NULLPTR;
    int   paid = 0;

    if (!(*(void **)(p + P_CUR) && id == *(int *)(p + P_CURID))) {
        for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it))
            if (IT_ID(it) == id) { paid = is_paid(it); cls = IT_CLS(it); break; }
    }
    t_actdel(p, id);
    if (paid) {
        for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it))
            if (IT_ID(it) == id) return;      /* still queued: nothing left */
        give_back(p, cls, "deleted");
    }
}

static void __thiscall h_clearq(BYTE *p)
{
    trace("ClearBuildQueue", p);
    drain(p, 0, "cleared");
    t_clearq(p);
}

static void __thiscall h_clearteam(BYTE *p)
{
    trace("ClearTeam", p);
    drain(p, rules_on(), "lost");
    t_clearteam(p);
}

static void __thiscall h_destroy(BYTE *p, int a)
{
    if (!*(p + 0x113) && is_producer(p)) drain(p, rules_on(), "destroyed");
    t_destroy(p, a);
}

/* The marks ride in the id counter, which the stock file already holds. */
static BYTE __thiscall h_save(BYTE *p, void *w)
{
    int   saved = *(int *)(p + P_NEXTID), mask = 0, i = 0;
    BYTE *it;
    BYTE  r;

    for (it = *(BYTE **)(p + P_HEAD); it && i < Q_MAX; it = IT_NEXT(it), i++)
        if (is_paid(it)) mask |= 1 << i;
    if (mask && saved >= 0 && saved < 0x10000)
        *(int *)(p + P_NEXTID) = saved | SAVE_TAG | (mask << 16);
    r = t_save(p, w);
    *(int *)(p + P_NEXTID) = saved;
    return r;
}

static BYTE __thiscall h_load(BYTE *p, void *rd)
{
    BYTE  r = t_load(p, rd);
    int   c = *(int *)(p + P_NEXTID), i = 0;
    BYTE *it;

    if (c & SAVE_TAG) {
        int mask = (c >> 16) & 0x3ff;
        for (it = *(BYTE **)(p + P_HEAD); it && i < Q_MAX; it = IT_NEXT(it), i++)
            if (mask & (1 << i)) IT_ID(it) |= PAID;
        *(int *)(p + P_NEXTID) = c & 0xffff;
    }
    return r;
}

/* ---- patching ------------------------------------------------------------ */

static BYTE g_tramp[16][16] __attribute__((aligned(16)));
static int  g_ntramp;

typedef struct { DWORD at; int len; BYTE sig[8]; } Entry;

static void poke(BYTE *at, const BYTE *bytes, int n)
{
    DWORD old;
    int   k;
    if (!VirtualProtect(at, (UINT)n, PAGE_EXECUTE_READWRITE, &old)) return;
    for (k = 0; k < n; k++) at[k] = bytes[k];
    VirtualProtect(at, (UINT)n, old, &old);
    FlushInstructionCache(GetCurrentProcess(), at, (UINT)n);
}

static int sig_ok(const Entry *e)
{
    int k;
    for (k = 0; k < e->len; k++)
        if (((const BYTE *)e->at)[k] != e->sig[k]) return 0;
    return 1;
}

/* Entry hooks.  The first `len` bytes of each (whole instructions, no relative
 * ones) are copied behind a jump back, and the entry jumps to the hook. */
static const Entry k_start    = { 0x4b8180, 6, { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x14 } };
static const Entry k_pop      = { 0x4b79b0, 6, { 0x53, 0x56, 0x8B, 0xF1, 0x33, 0xDB } };
static const Entry k_actdel   = { 0x4b7a60, 5, { 0x55, 0x8B, 0xEC, 0x53, 0x56 } };
static const Entry k_cancel   = { 0x4b84b0, 6, { 0x55, 0x8B, 0xEC, 0x51, 0x53, 0x56 } };
static const Entry k_clearq   = { 0x4b78c0, 6, { 0x53, 0x56, 0x8B, 0xF1, 0x33, 0xDB } };
static const Entry k_clearteam= { 0x4b7790, 5, { 0x56, 0x8B, 0xF1, 0x8B, 0x06 } };
static const Entry k_destroy  = { 0x4c96f0, 6, { 0x55, 0x8B, 0xEC, 0x56, 0x8B, 0xF1 } };
static const Entry k_save     = { 0x4b8aa0, 6, { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x08 } };
static const Entry k_load     = { 0x4b88d0, 6, { 0x55, 0x8B, 0xEC, 0x83, 0xEC, 0x10 } };

static void *hook(const Entry *e, void *fn)
{
    BYTE *tr = g_tramp[g_ntramp++];
    BYTE  jmp[8];
    int   k;

    for (k = 0; k < e->len; k++) tr[k] = ((const BYTE *)e->at)[k];
    tr[e->len] = 0xE9;
    *(LONG *)(tr + e->len + 1) = (LONG)((e->at + (DWORD)e->len) - ((DWORD)tr + (DWORD)e->len + 5));
    jmp[0] = 0xE9;
    *(LONG *)(jmp + 1) = (LONG)((DWORD)fn - (e->at + 5));
    for (k = 5; k < e->len; k++) jmp[k] = 0x90;
    poke((BYTE *)e->at, jmp, e->len);
    return tr;
}

static int patch_rules(void)
{
    DWORD old;
    BYTE  call[5];

    if (!sig_ok(&k_start) || !sig_ok(&k_pop) || !sig_ok(&k_actdel) || !sig_ok(&k_cancel) ||
        !sig_ok(&k_clearq) || !sig_ok(&k_clearteam) || !sig_ok(&k_destroy) ||
        !sig_ok(&k_save) || !sig_ok(&k_load)) return 0;
    /* the push call: its target is checked from its displacement */
    if (!(*(BYTE *)0x431532 == 0xE8 &&
          0x431532 + 5 + *(LONG *)0x431533 == 0x4b7930)) return 0;
    if (!VirtualProtect(g_tramp, sizeof g_tramp, PAGE_EXECUTE_READWRITE, &old)) return 0;

    t_start     = (BoolFn)hook(&k_start, (void *)h_start);
    t_pop       = (VoidFn)hook(&k_pop, (void *)h_pop);
    t_actdel    = (IntFn) hook(&k_actdel, (void *)h_actdel);
    t_cancel    = (BoolFn)hook(&k_cancel, (void *)h_cancel);
    t_clearq    = (VoidFn)hook(&k_clearq, (void *)h_clearq);
    t_clearteam = (VoidFn)hook(&k_clearteam, (void *)h_clearteam);
    t_destroy   = (IntFn) hook(&k_destroy, (void *)h_destroy);
    t_save      = (PtrFn) hook(&k_save, (void *)h_save);
    t_load      = (PtrFn) hook(&k_load, (void *)h_load);

    call[0] = 0xE8;
    *(LONG *)(call + 1) = (LONG)((DWORD)h_push - (0x431532 + 5));
    poke((BYTE *)0x431532, call, 5);
    return 1;
}

/* ---- startup -------------------------------------------------------------- */

static void startup(void)
{
    char path[320], ini[320], b[200];
    int  n = (int)GetModuleFileNameA(NULLPTR, path, sizeof path);

    if (n <= 0) return;
    while (n > 0 && path[n - 1] != '\\' && path[n - 1] != '/') n--;
    path[n] = 0;
    ini[0] = 0;       s_cat(ini, path);       s_cat(ini, "QOLRules.ini");
    g_logpath[0] = 0; s_cat(g_logpath, path); s_cat(g_logpath, "QOLRules.log");
    g_logging = (int)GetPrivateProfileIntA("Rules", "Log", 1, ini);
    g_trace = g_logging > 1;

    b[0] = 0;
    s_cat(b, "--- QOLRules PayOnQueue=");
    if (GetPrivateProfileIntA("Rules", "PayOnQueue", 1, ini)) {
        s_cat(b, "1");
        if (patch_rules()) {
            g_pay = 1;
            s_cat(b, "  -> items are paid when queued (single player and skirmish only), patched");
        } else {
            s_cat(b, "  NOT PATCHED: site bytes differ -- not the Armada2.exe this was built for");
        }
    } else {
        s_cat(b, "0  (items are paid as they start, as in stock)");
    }
    logline(b);
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
