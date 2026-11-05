/* ---- a move path starts at its first point (QOL-10) ------------------- */

/* Stock: with the waypoint key held (the `enter_waypoint` binding, Tab), each
 * right-click adds the point under the cursor to the path being sketched
 * (cOverViewImp::mNormalModeRightClick calls mAddPathPoint, which makes the path at the
 * first point and inserts after the last one later) and sends nothing; when the key is
 * let go, mProcessWaypoints sends the whole path once, GameObject::QueueCommand(the
 * cursor's command, the selection's ids, the path), the ordinary path order
 * (NetOrderObjectsWhatPath), and frees the sketch.  Each ship that gets it flies a
 * copy: its craft process holds the path (+0x88) and the point it heads to (+0x12c).
 *
 * PathStartsAtOnce=1: the two calls of mAddPathPoint go through mp_add_hook, which,
 * after the point is added, sends the same order stock sends on release, for the
 * sketch as it stands -- so the ships set off at the first point and take each new
 * point as it comes.  An order sent again restarts the ships at its first point, so
 * the points every selected ship has already passed are left out of it (read off each
 * ship's copy: which sketch point its copy ends at, and how far along it it is).  On
 * release, mProcessWaypoints' own send goes through mp_release_hook, which leaves it
 * out when the last send already carried every point (it would turn the ships back to
 * the start); otherwise it is stock's call.  Nothing but the stock order is sent, so
 * other players carry it out like any other. */
typedef BYTE *(__thiscall *AddPointFn)(BYTE *ov, BYTE *path, int after, const float *pt);
typedef void  (__cdecl    *QueueFn)(int cmd, void *ids, void *path);
#define ADD_POINT      ((AddPointFn)a2s_qol_mp_add_point)   /* cOverViewImp::mAddPathPoint */
#define QUEUE_PATH     ((QueueFn)a2s_qol_mp_queue)          /* GameObject::QueueCommand */
#define MP_PROCESS     QOL_F_MP_GO_PROCESS    /* GameObject: its AiProcess */
#define MP_COMMAND     QOL_F_MP_PROC_COMMAND  /* CraftProcess: the command it carries out */
#define MP_PATH        QOL_F_MP_PROC_PATH     /* the path it follows (AiPath *) */
#define MP_POINT       QOL_F_MP_PROC_POINT    /* the index of the point it heads to */
#define MP_QUEUED      QOL_V_MP_ACT_QUEUED    /* action flag: stock sends nothing on release */
#define MP_PACKET_MAX  QOL_V_MP_PACKET_MAX
#define MP_CMD_PATROL  QOL_V_MP_CMD_PATROL
#define MP_SETTLE_MS   1500   /* a ship with no path this long after the first send has arrived */

__declspec(dllimport) DWORD __stdcall GetTickCount(void);

static int  g_rdyPaths;
static int  g_pathsOk;          /* the three calls are re-pointed */
int         g_pathStart;        /* PathStartsAtOnce */

static BYTE *g_mpSketch;        /* the sketch the last send was for */
static int   g_mpSent;          /* its point count then (0: nothing sent) */
static int   g_mpOff;           /* the first of its points that send carried */
static DWORD g_mpFirst;         /* when this sketch was first sent */
static int   g_mpSaid;          /* log lines so far */

/* What QueueCommand reads: a VC6 std::vector<int> (+4 first, +8 last, +0xc end) and
 * an AiPath's first two fields (the count, the points). */
typedef struct { int pad; int *first, *last, *end; } IdVec;
typedef struct { int n; const float *p; } PathHead;

static int same_point(const float *a, const float *b)
{
    return a[0] == b[0] && a[1] == b[1] && a[2] == b[2];
}

/* The sketch point (index into sk's n points) ship `o` heads to now, or -1 when its
 * order is not known to come from this sketch. */
static int ship_at(BYTE *o, const float *sk, int n)
{
    BYTE *pr, *path;
    const float *q;
    int   c, i, j;

    pr = *(BYTE **)(o + MP_PROCESS);
    if (!pr) return -1;
    path = *(BYTE **)(pr + MP_PATH);
    if (!path) {
        /* stopped: sent this sketch a while ago (not still on its way over the
         * network), it reached the end of what was sent */
        return g_mpSent && GetTickCount() - g_mpFirst > MP_SETTLE_MS ? g_mpSent - 1 : -1;
    }
    if (*(int *)(pr + MP_COMMAND) != CMD_GO) return -1;
    c = *(int *)path;
    q = *(const float **)(path + 4);
    i = *(int *)(pr + MP_POINT);
    if (c <= 0 || !q || i < 0 || i >= c) return -1;
    for (j = n - 1; j >= c - 1; j--)
        if (same_point(q + 3 * (c - 1), sk + 3 * j)) return j - (c - 1) + i;
    return -1;
}

/* Sends the sketch as it stands, from the first point a selected ship still has to
 * reach. */
static void mp_send(BYTE *ov, BYTE *sk)
{
    typedef const int *(__thiscall *ListFn)(BYTE *);
    int   ids[SEL_MAX];
    const int *sel;
    const float *pts;
    IdVec v;
    PathHead h;
    int   a, cmd, n, k, i, off, low, team;

    if (!g_pathStart || !g_pathsOk || !sk) return;
    a = *(int *)(ov + OV_ACTION);
    if (a & MP_QUEUED) return;                 /* stock sends nothing on release either */
    cmd = a & ACT_COMMAND;
    n = *(int *)sk;
    pts = *(const float **)(sk + 4);
    k = *(int *)(ov + OV_COUNT);
    if (n <= 0 || !pts || k <= 0) return;
    if (k > SEL_MAX) k = SEL_MAX;
    sel = ((ListFn)(*(BYTE ***)ov)[OV_GETLIST])(ov);
    team = USER_TEAM();
    if (sk != g_mpSketch) { g_mpSketch = sk; g_mpSent = 0; g_mpOff = 0; }

    /* the points every selected ship of ours has passed; none for a patrol, which
     * goes round again */
    off = g_mpOff;
    if (cmd != MP_CMD_PATROL && g_mpSent) {
        low = n;
        for (i = 0; i < k; i++) {
            BYTE *o = ENTITY_GET(sel[i]);
            int t;
            if (!is_obj(o) || *(int *)(o + GO_TEAM) != team) continue;
            t = ship_at(o, pts, n);
            if (t < 0) t = g_mpOff;
            if (t < low) low = t;
        }
        if (low < n && low > off) off = low;
    }
    if (off > n - 1) off = n - 1;

    for (i = 0; i < k; i++) ids[i] = sel[i];
    v.pad = 0; v.first = ids; v.last = ids + k; v.end = ids + k;
    h.n = n - off; h.p = pts + 3 * off;
    if (4 * (k + 3 * h.n) + 10 > MP_PACKET_MAX) return;   /* QueueCommand would drop it */
    QUEUE_PATH(cmd, &v, &h);
    if (!g_mpSent) g_mpFirst = GetTickCount();
    g_mpSent = n;
    g_mpOff = off;
    if (g_mpSaid < 40) {
        char b[120];
        g_mpSaid++;
        b[0] = 0;
        s_cat(b, "move paths: point ");
        s_num(b, n);
        s_cat(b, " sent at once, the order from point ");
        s_num(b, off + 1);
        s_cat(b, ", ");
        s_num(b, k);
        s_cat(b, " selected");
        logline(b);
    }
}

/* mNormalModeRightClick's two calls of mAddPathPoint, re-pointed. */
BYTE *__thiscall mp_add_hook(BYTE *ov, BYTE *path, int after, const float *pt)
{
    BYTE *r = ADD_POINT(ov, path, after, pt);
    if (!path) { g_mpSketch = r; g_mpSent = 0; g_mpOff = 0; }
    mp_send(ov, path ? path : r);
    return r;
}

/* mProcessWaypoints' call of QueueCommand, re-pointed: left out when the last send
 * carried the whole sketch. */
void __cdecl mp_release_hook(int cmd, void *ids, BYTE *path)
{
    int skip = g_pathStart && g_pathsOk && path && path == g_mpSketch && g_mpSent == *(int *)path;
    g_mpSketch = 0; g_mpSent = 0; g_mpOff = 0;
    if (skip) {
        if (g_mpSaid < 40) { g_mpSaid++; logline("move paths: key let go, the path already sent"); }
        return;
    }
    QUEUE_PATH(cmd, ids, path);
}

static int patch_paths(void)
{
    if (!g_rdyPaths) return 0;
    return a2_write_call(a2e_addr(QOL_S_MP_ADD_CALL), mp_add_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MP_FIRST_CALL), mp_add_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MP_RELEASE_CALL), mp_release_hook, 5);
}
