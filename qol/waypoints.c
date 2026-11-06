/* ---- the move queue (QOL-10) ------------------------------------------- */

/* Stock: with the waypoint key held (the `enter_waypoint` binding, Tab), each
 * right-click adds the point under the cursor to the path being sketched
 * (cOverViewImp::mNormalModeRightClick calls mAddPathPoint) and sends nothing; when the
 * key is let go, mProcessWaypoints sends the whole path once, GameObject::QueueCommand
 * (the cursor's command, the selection's ids, the path), the ordinary path order
 * (NetOrderObjectsWhatPath), and frees the sketch.  One path order carries one command,
 * and a new order replaces the old one, starting at its first point.
 *
 * PathStartsAtOnce=1 keeps a queue of points per group of ships here instead (see
 * qol/README.md, QOL-10, for the design and the findings beside each hook):
 *   - each right-click with the key held (mp_add_hook), and each right-click on the UI's
 *     minimap with it held (QOL_MoveQueuePoint), appends a point to the queue of every
 *     selected ship of the player's, with the mode the click had (GO, GO_WARP or
 *     PATROL); ships with no queue start one together;
 *   - each ship of a queue has a place in the group, from stock's formation for its
 *     ships (`spread`), and flies to each point plus its place turned to the leg
 *     (`target`), with a path order of its own;
 *   - the queue is flown one run (the longest stretch of legs of one mode) at a time,
 *     each run as the stock path order for that run's points, sent at once when it
 *     extends the run being flown, and to each ship on its own when it reaches the end
 *     of the run before (no ship waits for its group); QOL's own sends are not spoken
 *     (mq_say_hook);
 *   - the player's own orders end it: every order executed for one of the player's
 *     ships passes one of the hooks on the orders' executors (the NetOrder*::Execute
 *     calls of GameObject::DeQueueCommand, FormationManager::Broadcast_Go_Command,
 *     FormFormation and the special attack's SetCommand), and any that replaces the
 *     ship's movement takes it out of its queue; QOL's own path orders are told apart
 *     by their points;
 *   - the end of a run is read off each ship's craft process (+0x44): its path (+0x88)
 *     stops being the copy of the order sent (STOP releases it on arrival), or for a
 *     patrol that has a successor, the point it heads to (+0x12c) wraps past the last.
 * The release's own send is left out (mp_release_hook).  Everything runs on the game
 * thread: the clicks, the executors and the step after the overview's keyboard input
 * (an order sent from inside the simulation step is never carried out); the timers run
 * on the simulation's clock, added up after GameObject::PreSimulateAll. */
typedef BYTE *(__thiscall *AddPointFn)(BYTE *ov, BYTE *path, int after, const float *pt);
typedef void  (__cdecl    *QueueFn)(int cmd, void *ids, void *path);
typedef void  (__cdecl    *DqPathFn)(int node, int cmd, int n, int *ids, int np, const float *pts);
typedef void  (__cdecl    *PreSimFn)(float dt);
typedef const char *(__cdecl *CmdNameFn)(int cmd);
#define ADD_POINT      ((AddPointFn)a2s_qol_mp_add_point)   /* cOverViewImp::mAddPathPoint */
#define QUEUE_PATH     ((QueueFn)a2s_qol_mp_queue)          /* GameObject::QueueCommand */
#define DQ_PATH        ((DqPathFn)a2s_qol_mq_dq_path)       /* GameObject::DeQueueCommand, the path */
#define PRESIM         ((PreSimFn)a2s_qol_mq_presim)        /* GameObject::PreSimulateAll */
typedef void  (__thiscall *OvKeysFn)(BYTE *ov);
#define OV_KEYS        ((OvKeysFn)a2s_qol_mq_ov_keys)       /* cOverViewImp::mProcessKeyboardInput */
#define CMD_NAME       ((CmdNameFn)a2s_qol_mq_cmd_name)     /* AiCommandToName */
#define MP_PROCESS     QOL_F_MP_GO_PROCESS    /* GameObject: its AiProcess */
#define MP_PATH        QOL_F_MP_PROC_PATH     /* CraftProcess: the path it follows (AiPath *) */
#define MP_POINT       QOL_F_MP_PROC_POINT    /* the index of the point it heads to */
#define MP_QUEUED      QOL_V_MP_ACT_QUEUED    /* action flag: stock sends nothing on release */
#define MP_PACKET_MAX  QOL_V_MP_PACKET_MAX
#define MP_CMD_PATROL  QOL_V_MP_CMD_PATROL
#define MQ_NEXT_PATH   QOL_F_MQ_GO_NEXT_PATH     /* GameObject: its next command's path (AiPath *) */
#define OV_KEY         QOL_F_MQ_OV_WAYPOINT_KEY   /* cOverViewImp: int *, the key's live state */
#define PROC_FORMATION QOL_F_MQ_PROC_FORMATION    /* CraftProcess: the formation type it moves in */
#define FRM_COUNT      QOL_F_MQ_FRM_COUNT         /* Formation: its slots */
#define FRM_RADIUS     QOL_F_MQ_FRM_RADIUS        /* the ship radius they are drawn for */
#define FRM_SLOTS      QOL_F_MQ_FRM_SLOTS
#define SLOT_SIZE      QOL_V_MQ_SLOT_SIZE
#define SLOT_PREFS     QOL_F_MQ_SLOT_PREFS        /* seven ship types, best first */
#define SLOT_OFFSET    QOL_F_MQ_SLOT_OFFSET       /* a Vector3: right, up, ahead */
#define SLOT_RANK      QOL_F_MQ_SLOT_RANK
#define ENT_BOUNDS     QOL_F_MQ_ENT_BOUNDS
#define BOUNDS_RADIUS  QOL_V_MQ_BOUNDS_RADIUS
#define CLASS_SHIPTYPE QOL_F_MQ_CLASS_SHIPTYPE

#define MQ_QUEUES   32      /* queues at once */
#define MQ_SHIPS    256     /* ships in queues at once */
#define MQ_PTS      64      /* points in one queue */
#define MQ_RING     512     /* orders of ours remembered (one per ship), to tell them from the player's */
#define MQ_EXEC_MS  1500    /* an order executed whose path never shows: flown, or refused */
#define MQ_POST_MS  8000    /* an order sent that never executes: given up */
#define MQ_SAID_MAX 600     /* log lines */
#define MQ_PASS_NEAR 200.0f /* a ship that goes past a point this near its own target reached it */
#define MQ_NEAR     0.25f   /* a point of ours read back off an order or a path: as far off as it may be */

enum { MQ_GO, MQ_WARP, MQ_PATROL };
enum { MS_WAIT, MS_POSTED, MS_EXEC, MS_FLYING };

typedef struct {
    int   used, serial, n;
    float p[MQ_PTS][3];
    float dir[MQ_PTS][2];   /* the way the leg to each point heads (x, z; unit): its slots face it */
    BYTE  mode[MQ_PTS];
    int   form;             /* the formation type its slots came from (-1: the group's layout) */
    float org[3];           /* where the group was when the queue began */
    int   reached, astray;  /* points a ship reached nearest its own target / nearer another's */
} Queue;

typedef struct {
    int   q;                /* its queue; -1: free */
    int   id;
    BYTE *obj;
    int   state;
    int   cmd, patrol;      /* the order it was sent */
    int   off, cnt, end;    /* that order: from queue point off, cnt points; the point after */
    int   last;             /* the queue point it headed to at the last tick */
    BYTE *path;             /* the AiPath that order made for it (GameObject+0x88 then) */
    DWORD t;                /* when that order was sent / executed (g_simMs) */
    float slot[3];          /* its place in the group: right, up, ahead of the queue's point */
    int   slotIx;           /* the formation slot it holds (-1: by the group's layout) */
    BYTE  quiet;            /* its order was QOL's own, not a click's: no acknowledgement */
    BYTE  mute;             /* that order executed: its acknowledgement is left out until flown */
    BYTE  muted;            /* one was left out for this order (logged once) */
} Member;

typedef struct { int cmd, n; float a[3], b[3]; } Sent;


static int    g_rdyPaths;
static int    g_pathsOk;        /* every call is re-pointed */
int           g_pathStart;      /* PathStartsAtOnce */

static Queue  g_q[MQ_QUEUES];
static Member g_m[MQ_SHIPS];
static int    g_mqLive;         /* ships in queues */
static int    g_mqSerial;
static Sent   g_sent[MQ_RING];
static int    g_sentAt;
static int    g_sketchTaken;    /* a point of the sketch being drawn went to a queue */
static LONG   g_loadTime;       /* g_finish_load_time when the queues were made */
static int    g_mqSaid;
static DWORD  g_simMs;         /* the simulation's clock, from PreSimulateAll's steps */
static float  g_simFrac;
static int    g_clrN, g_clrCmd = -1;   /* clears not logged yet, and their order */
static int    g_inClick;        /* a player's click is being handled: its sends are its orders */

/* What QueueCommand reads: a VC6 std::vector<int> (+4 first, +8 last, +0xc end) and
 * an AiPath's first two fields (the count, the points). */
typedef struct { int pad; int *first, *last, *end; } IdVec;
typedef struct { int n; const float *p; } PathHead;

static int mq_enabled(void) { return g_pathStart && g_pathsOk; }

static void mq_log(const char *b)
{
    if (g_mqSaid >= MQ_SAID_MAX) return;
    if (++g_mqSaid == MQ_SAID_MAX) { logline("move queue: (log lines capped)"); return; }
    logline(b);
}

/* The same point give or take what x87 rounding leaves of one computed twice. */
static int near_point(const float *a, const float *b)
{
    float d0 = a[0] - b[0], d1 = a[1] - b[1], d2 = a[2] - b[2];
    return d0 < MQ_NEAR && d0 > -MQ_NEAR && d1 < MQ_NEAR && d1 > -MQ_NEAR
        && d2 < MQ_NEAR && d2 > -MQ_NEAR;
}

/* Where ship m goes for queue point k: the point, plus its place in the group turned
 * to face the leg that ends there (right = (z, 0, -x) of the heading, as stock's
 * BuildDirectionalMatrix makes it for the formations). */
static void target(const Queue *q, const Member *m, int k, float *out)
{
    float fx = q->dir[k][0], fz = q->dir[k][1];
    out[0] = q->p[k][0] + m->slot[0] * fz + m->slot[2] * fx;
    out[1] = q->p[k][1] + m->slot[1];
    out[2] = q->p[k][2] - m->slot[0] * fx + m->slot[2] * fz;
}

static const char *mode_name(int m) { return m == MQ_WARP ? "warp" : m == MQ_PATROL ? "patrol" : "go"; }
static int mode_cmd(int m) { return m == MQ_WARP ? CMD_GO_WARP : m == MQ_PATROL ? MP_CMD_PATROL : CMD_GO; }

/* The run point i belongs to: the longest stretch of one mode around it. */
static int run_start(const Queue *q, int i)
{
    while (i > 0 && q->mode[i - 1] == q->mode[i]) i--;
    return i;
}
static int run_end(const Queue *q, int i)
{
    while (i + 1 < q->n && q->mode[i + 1] == q->mode[i]) i++;
    return i + 1;
}
/* A patrol loops while it is the queue's last run; one with a run after it is flown
 * once, to its last point, and the queue goes on. */
static int loops(const Queue *q, int i)
{
    return i < q->n && q->mode[i] == MQ_PATROL && run_end(q, i) == q->n;
}

/* The queue point of the last point of the order m was sent. */
static int order_last(const Queue *q, const Member *m)
{
    int ra;
    if (!m->patrol) return m->off + m->cnt - 1;
    ra = run_start(q, m->off);
    return m->off > ra ? m->off - 1 : ra + m->cnt - 1;
}

/* 1 while the ship's process follows the path its order of ours made (the very AiPath
 * DeQueueCommand built for it, which the process takes over); *cur is then the queue
 * point it heads to, found by the point its path index names. */
static int flies_order(const Member *m, int *cur)
{
    BYTE *pr, *path;
    const float *pp, *t;
    const Queue *q = &g_q[m->q];
    int c, i, j, ra;

    pr = *(BYTE **)(m->obj + MP_PROCESS);
    if (!pr || !m->path) return 0;
    path = *(BYTE **)(pr + MP_PATH);
    if (path != m->path) return 0;
    c = *(int *)path;
    pp = *(const float **)(path + 4);
    i = *(int *)(pr + MP_POINT);
    *cur = m->last;
    if (!pp || i < 0 || i >= c) return 1;
    t = pp + 3 * i;
    ra = run_start(q, m->off);
    for (j = 0; j < m->cnt; j++) {
        int k = m->patrol ? ra + (m->off - ra + i + j) % m->cnt : m->off + (i + j) % m->cnt;
        float w[3];
        target(q, m, k, w);
        if (near_point(t, w)) { *cur = k; return 1; }
    }
    return 1;                   /* a point of the engine's own (a detour): as before */
}



/* The queue point m heads to now (q->n when it has flown everything). */
static int member_cur(const Member *m)
{
    int i;
    switch (m->state) {
    case MS_WAIT:   return m->end;
    case MS_FLYING: return flies_order(m, &i) ? i : m->last;
    default:        return m->off;
    }
}

static Member *member_of_obj(const void *o)
{
    int i;
    if (!g_mqLive || !o) return 0;
    for (i = 0; i < MQ_SHIPS; i++)
        if (g_m[i].q >= 0 && g_m[i].obj == (const BYTE *)o) return &g_m[i];
    return 0;
}

static Member *member_of_id(int id)
{
    int i;
    if (!g_mqLive) return 0;
    for (i = 0; i < MQ_SHIPS; i++)
        if (g_m[i].q >= 0 && g_m[i].id == id) return &g_m[i];
    return 0;
}

static int queue_size(int qi)
{
    int i, k = 0;
    for (i = 0; i < MQ_SHIPS; i++) if (g_m[i].q == qi) k++;
    return k;
}

static void drop_member(Member *m)
{
    m->q = -1;
    m->obj = 0;
    g_mqLive--;
}

static void free_queue(int qi)
{
    int i;
    for (i = 0; i < MQ_SHIPS; i++) if (g_m[i].q == qi) drop_member(&g_m[i]);
    g_q[qi].used = 0;
}

static void reset_all(const char *why)
{
    int i, had = g_mqLive;
    for (i = 0; i < MQ_QUEUES; i++) if (g_q[i].used) free_queue(i);
    g_mqLive = 0;
    g_clrN = 0;
    if (had && why) {
        char b[96];
        b[0] = 0;
        s_cat(b, "move queue: reset (");
        s_cat(b, why);
        s_cat(b, ")");
        mq_log(b);
    }
}

static int new_queue(void)
{
    int i;
    for (i = 0; i < MQ_QUEUES; i++)
        if (!g_q[i].used) {
            g_q[i].used = 1;
            g_q[i].n = 0;
            g_q[i].form = -1;
            g_q[i].reached = g_q[i].astray = 0;
            g_q[i].org[0] = g_q[i].org[1] = g_q[i].org[2] = 0.0f;
            g_q[i].serial = ++g_mqSerial;
            return i;
        }
    return -1;
}

/* ---- sending ------------------------------------------------------------ */

static void remember(int cmd, int n, const float *a, const float *b)
{
    Sent *s = &g_sent[g_sentAt++ % MQ_RING];
    s->cmd = cmd; s->n = n;
    s->a[0] = a[0]; s->a[1] = a[1]; s->a[2] = a[2];
    s->b[0] = b[0]; s->b[1] = b[1]; s->b[2] = b[2];
}

static int ours(int cmd, int n, const float *pts)
{
    int i;
    if (n <= 0 || !pts) return 0;
    for (i = 0; i < MQ_RING; i++)
        if (g_sent[i].n == n && g_sent[i].cmd == cmd && near_point(g_sent[i].a, pts)
            && near_point(g_sent[i].b, pts + 3 * (n - 1)))
            return 1;
    return 0;
}

/* Sends the k ships ms[] (all of queue qi, all to fly on from queue point `from`) the
 * stock path order for the run there, one order per ship, each to its own targets (the
 * points plus its place in the group, `target`): the run's points from `from` (a patrol:
 * the whole loop, starting at `from`), as many as the packet takes. */
static void send_from(int qi, Member **ms, int k, int from, const char *what)
{
    static float pts[MQ_PTS][3];
    static int   idx[MQ_PTS];
    Queue *q = &g_q[qi];
    int   id;
    IdVec v;
    PathHead h;
    int   ra, rb, mode, cmd, maxp, cnt, end, i, j, patrol, rest = 0;

    if (k <= 0 || from >= q->n) return;
    ra = run_start(q, from);
    rb = run_end(q, from);
    mode = q->mode[from];
    cmd = mode_cmd(mode);
    maxp = ((MP_PACKET_MAX - 10) / 4 - 1) / 3;
    patrol = mode == MQ_PATROL;
    if (patrol) {
        cnt = rb - ra;
        if (cnt > maxp) cnt = maxp;          /* a loop too long for one order is cut short */
        for (i = 0; i < cnt; i++) idx[i] = ra + (from - ra + i) % (rb - ra);
        end = rb;
    } else {
        cnt = rb - from;
        if (cnt > maxp) { rest = cnt - maxp; cnt = maxp; }
        for (i = 0; i < cnt; i++) idx[i] = from + i;
        end = from + cnt;
    }
    if (4 * (1 + 3 * cnt) + 10 > MP_PACKET_MAX) return;   /* QueueCommand would drop it */

    for (j = 0; j < k; j++) {
        Member *m = ms[j];
        for (i = 0; i < cnt; i++) target(q, m, idx[i], pts[i]);
        id = m->id;
        v.pad = 0; v.first = &id; v.last = &id + 1; v.end = &id + 1;
        h.n = cnt; h.p = &pts[0][0];
        remember(cmd, cnt, pts[0], pts[cnt - 1]);
        QUEUE_PATH(cmd, &v, &h);
        m->state = MS_POSTED;
        m->quiet = (BYTE)!g_inClick;
        m->mute = m->muted = 0;
        m->cmd = cmd;
        m->patrol = patrol;
        m->off = from;
        m->cnt = cnt;
        m->end = end;
        m->last = from;
        m->t = g_simMs;
    }
    {
        char b[200];
        b[0] = 0;
        s_cat(b, "move queue: ");
        s_cat(b, what);
        s_cat(b, ": ");
        s_cat(b, mode_name(mode));
        s_cat(b, ", ");
        s_num(b, cnt);
        s_cat(b, " point(s) from point ");
        s_num(b, from + 1);
        s_cat(b, ", ");
        s_num(b, k);
        s_cat(b, " ship(s), queue ");
        s_num(b, q->serial);
        if (from > 0) {             /* ships of the queue not yet that far: not waited for */
            int behind = 0;
            for (i = 0; i < MQ_SHIPS; i++)
                if (g_m[i].q == qi && member_cur(&g_m[i]) < from) behind++;
            if (behind) { s_cat(b, ", "); s_num(b, behind); s_cat(b, " ship(s) behind"); }
        }
        if (!g_inClick) s_cat(b, " (quiet)");
        if (rest) {
            s_cat(b, " (split for the packet: ");
            s_num(b, rest);
            s_cat(b, " more later)");
        }
        mq_log(b);
    }
}

/* Sends every ship in `sel` (of queue qi) on from its point, one order per point. */
static void send_each(int qi, Member **sel, int k, const char *what)
{
    static Member *batch[MQ_SHIPS];
    int    done[MQ_SHIPS];
    int    i, j, nb, from;

    for (i = 0; i < k; i++) done[i] = 0;
    for (i = 0; i < k; i++) {
        if (done[i]) continue;
        from = member_cur(sel[i]);
        nb = 0;
        for (j = i; j < k; j++) {
            if (done[j] || member_cur(sel[j]) != from) continue;
            done[j] = 1;
            batch[nb++] = sel[j];
        }
        if (nb) send_from(qi, batch, nb, from, what ? what : !from ? "run sent" :
                          run_start(&g_q[qi], from) == from ? "next run sent" : "next part sent");
    }
}

/* One queue's step: each ship that has reached the end of its order flies on at once,
 * on its own: no ship waits for the rest of its group. */
static void queue_step(int qi)
{
    static Member *wait[MQ_SHIPS];
    Queue *q = &g_q[qi];
    int   i, k = 0, busy = 0, total = 0;

    for (i = 0; i < MQ_SHIPS; i++) {
        Member *m = &g_m[i];
        if (m->q != qi) continue;
        total++;
        if (m->state != MS_WAIT) busy++;
        else if (m->end < q->n) wait[k++] = m;
    }
    if (!total) { q->used = 0; return; }
    if (!busy && !k) {
        char b[160];
        b[0] = 0;
        s_cat(b, "move queue: queue ");
        s_num(b, q->serial);
        s_cat(b, " done, ");
        s_num(b, total);
        s_cat(b, " ship(s)");
        if (total > 1 && q->reached && !q->astray) s_cat(b, ", each at its own target at every point");
        else if (q->astray) {
            s_cat(b, ", ");
            s_num(b, q->astray);
            s_cat(b, " time(s) a ship not at its own target");
        }
        mq_log(b);
        free_queue(qi);
        return;
    }
    if (k) send_each(qi, wait, k, 0);
}

/* ---- the player's orders ------------------------------------------------ */

static int keeps_movement(int cmd)
{
    if (cmd < 0 || cmd >= 64) return 0;
    return cmd < 32 ? (QOL_V_MQ_KEEP_LO >> cmd) & 1 : (QOL_V_MQ_KEEP_HI >> (cmd - 32)) & 1;
}

static void flush_clears(void)
{
    char b[120];
    const char *n;
    int  c = g_clrCmd, k;
    if (!g_clrN) return;
    b[0] = 0;
    s_cat(b, "move queue: cleared for ");
    s_num(b, g_clrN);
    s_cat(b, " ship(s): ");
    if (c == QOL_V_MQ_CMD_STOP) n = "stop";
    else if (c == CMD_GO || c == QOL_V_MQ_CMD_GO_SINGLE) n = "move";
    else if (c == CMD_GO_WARP || c == QOL_V_MQ_CMD_GO_WARP_SINGLE) n = "warp";
    else if (c == -2) n = "path";
    else n = 0;
    if (n) s_cat(b, n);
    else {
        n = (c >= 0 && c < QOL_V_MQ_CMD_COUNT) ? CMD_NAME(c) : 0;
        k = s_len(b);
        if (n) s_cat(b, n); else s_num(b, c);
        for (; b[k]; k++) if (b[k] >= 'A' && b[k] <= 'Z') b[k] = (char)(b[k] + 32);
    }
    mq_log(b);
    g_clrN = 0;
}

/* m got an order of the player's that replaces its movement (cmd; -2 a path order) */
static void clear_member(Member *m, int cmd)
{
    if (g_clrN && g_clrCmd != cmd) flush_clears();
    g_clrCmd = cmd;
    g_clrN++;
    drop_member(m);
}

/* The executors' hooks (the stubs below call them first, then the engine's function). */
void __cdecl mq_seen(BYTE *obj, int cmd)
{
    Member *m;
    if (!g_mqLive || keeps_movement(cmd)) return;
    m = member_of_obj(obj);
    if (m) clear_member(m, cmd);
}

void __cdecl mq_seen_ids(IdVec *v, int cmd)
{
    int *p;
    if (!g_mqLive || !v || !v->first || keeps_movement(cmd)) return;
    for (p = v->first; p < v->last; p++) {
        Member *m = member_of_id(*p);
        if (m) clear_member(m, cmd);
    }
}

/* NetOrderObjectsWhatPath::Execute's call: a path order of ours marks the ships it was
 * meant for as under way; anyone else's (the player's) ends their queues. */
void __cdecl mq_dq_path_hook(int node, int cmd, int n, int *ids, int np, const float *pts)
{
    int i, mine;
    if (g_mqLive && ids && n > 0) {
        mine = ours(cmd, np, pts);
        for (i = 0; i < n; i++) {
            Member *m = member_of_id(ids[i]);
            if (!m) continue;
            if (!mine) { clear_member(m, -2); continue; }
            if (m->state == MS_POSTED && m->cmd == cmd && m->cnt == np) {
                float a[3], b[3];
                target(&g_q[m->q], m, m->off, a);
                target(&g_q[m->q], m, order_last(&g_q[m->q], m), b);
                if (!near_point(pts, a) || !near_point(pts + 3 * (np - 1), b)) continue;
                m->state = MS_EXEC;
                m->t = g_simMs;
                m->path = 0;
                m->mute = m->quiet;
            }
        }
    }
    DQ_PATH(node, cmd, n, ids, np, pts);
    /* each ship it reached holds its own copy as its next command's path now */
    if (g_mqLive && ids && n > 0)
        for (i = 0; i < n; i++) {
            Member *m = member_of_id(ids[i]);
            if (m && m->state == MS_EXEC && !m->path) m->path = *(BYTE **)(m->obj + MQ_NEXT_PATH);
        }
}

/* thiscall on the ship, the command its first argument: note it, go on to the engine's */
#define MQ_SEEN_STUB(name, target)                                      \
    __attribute__((naked)) void name(void)                              \
    {                                                                   \
        __asm__ __volatile__(                                           \
            "pushl %ecx\n\t"                                            \
            "pushl 8(%esp)\n\t"                                         \
            "pushl %ecx\n\t"                                            \
            "call _mq_seen\n\t"                                         \
            "addl $8, %esp\n\t"                                         \
            "popl %ecx\n\t"                                             \
            "jmp *_" target "\n\t");                                    \
    }
MQ_SEEN_STUB(mq_dq_what_stub,      "a2s_qol_mq_dq_what")
MQ_SEEN_STUB(mq_dq_who_stub,       "a2s_qol_mq_dq_who")
MQ_SEEN_STUB(mq_dq_who_where_stub, "a2s_qol_mq_dq_who_where")
MQ_SEEN_STUB(mq_dq_wwp_stub,       "a2s_qol_mq_dq_who_where_param")
MQ_SEEN_STUB(mq_dq_class_stub,     "a2s_qol_mq_dq_class")
MQ_SEEN_STUB(mq_dq_cw_stub,        "a2s_qol_mq_dq_class_where")
MQ_SEEN_STUB(mq_dq_param_stub,     "a2s_qol_mq_dq_param")
MQ_SEEN_STUB(mq_dq_where_stub,     "a2s_qol_mq_dq_where")
MQ_SEEN_STUB(mq_dq_wp_stub,        "a2s_qol_mq_dq_where_param")
MQ_SEEN_STUB(mq_dq_who_param_stub, "a2s_qol_mq_dq_who_param")
MQ_SEEN_STUB(mq_special_stub,      "a2s_qol_mq_set_special")

/* Broadcast_Go_Command(cmd, where, ids, ...), thiscall on the FormationManager */
__attribute__((naked)) void mq_go_broadcast_stub(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "pushl 8(%esp)\n\t"         /* cmd */
        "pushl 0x14(%esp)\n\t"      /* ids */
        "call _mq_seen_ids\n\t"
        "addl $8, %esp\n\t"
        "popl %ecx\n\t"
        "jmp *_a2s_qol_mq_go_broadcast\n\t");
}

/* FormFormation(ids, ...), thiscall on the FormationManager */
__attribute__((naked)) void mq_form_formation_stub(void)
{
    __asm__ __volatile__(
        "pushl %ecx\n\t"
        "pushl _a2f_qol_mq_cmd_form_formation\n\t"
        "pushl 0xc(%esp)\n\t"       /* ids */
        "call _mq_seen_ids\n\t"
        "addl $8, %esp\n\t"
        "popl %ecx\n\t"
        "jmp *_a2s_qol_mq_form_formation\n\t");
}

/* ---- a group's places ---------------------------------------------------- */

/* Stock moves a group in its formation (FormationManager::Broadcast_Go_Command ->
 * Give_Formation_Go_Commands -> Do_Group_Move, qol/README.md, QOL-10, "A group's
 * targets"): the formation type its ships move in (CraftProcess+0x13c, set from the
 * bar's formation buttons), that formation's slots (offsets right, up and ahead), scaled
 * by the largest ship (Formation::GetScale), each slot in turn given the nearest ship of
 * the types it asks for, and every offset taken from the leader's (the filled slot of the
 * lowest rank), which goes to the point itself.  A path order sends every ship to the same
 * points, so a queue gives each ship its slot here, turned to face each leg (`target`).
 * Read only: the formations and the ships are the engine's, nothing is written to them. */

static BYTE *formation_of(int type)
{
    BYTE *mgr = *(BYTE **)a2s_qol_mq_formations;
    if (!mgr || type < 0 || type >= QOL_V_MQ_FORM_TYPES) return 0;
    return *(BYTE **)(mgr + 4 * type);
}

static float ship_radius(BYTE *o)
{
    BYTE *b = *(BYTE **)(o + ENT_BOUNDS);
    return b ? *(float *)(b + BOUNDS_RADIUS) : 0.0f;
}

/* The formation type most of queue qi's ships move in (-1: none readable). */
static int queue_formation(int qi)
{
    /* The build's count of types is read at start-up, so the tally has room for a cap
     * of them; a type past it counts as unreadable. */
    enum { FORM_CAP = 64 };
    static int count[FORM_CAP];
    int n = QOL_V_MQ_FORM_TYPES < FORM_CAP ? QOL_V_MQ_FORM_TYPES : FORM_CAP;
    int i, best = -1;
    for (i = 0; i < n; i++) count[i] = 0;
    for (i = 0; i < MQ_SHIPS; i++) {
        BYTE *pr;
        int t;
        if (g_m[i].q != qi) continue;
        pr = *(BYTE **)(g_m[i].obj + MP_PROCESS);
        if (!pr) continue;
        t = *(int *)(pr + PROC_FORMATION);
        if (t >= 0 && t < n) count[t]++;
    }
    for (i = 0; i < n; i++) if (count[i] && (best < 0 || count[i] > count[best])) best = i;
    return best;
}

static void log_place(const Queue *q, const Member *m)
{
    char b[160];
    b[0] = 0;
    s_cat(b, "move queue: queue ");
    s_num(b, q->serial);
    s_cat(b, " ship ");
    s_num(b, m->id);
    if (m->slotIx >= 0) { s_cat(b, " slot "); s_num(b, m->slotIx); }
    else s_cat(b, " by layout");
    s_cat(b, ": right ");
    s_num(b, f_round(m->slot[0]));
    s_cat(b, ", up ");
    s_num(b, f_round(m->slot[1]));
    s_cat(b, ", ahead ");
    s_num(b, f_round(m->slot[2]));
    mq_log(b);
}

/* Gives every ship of queue qi its place, from the group as it is now, facing `aim`
 * (the point it flies to next). */
static void spread(int qi, const float *aim, const char *why)
{
    static Member *ms[MQ_SHIPS];
    static BYTE    used[MQ_SHIPS];
    Queue *q = &g_q[qi];
    BYTE  *f, *slots;
    float  c[3] = { 0, 0, 0 }, fx, fz, len, rmax = (float)QOL_V_MQ_MIN_RADIUS, scale = 1.0f;
    int    n = 0, i, j, s, cnt, placed = 0, lead = -1, layout = 0;
    char   b[160];

    for (i = 0; i < MQ_SHIPS; i++) if (g_m[i].q == qi) ms[n++] = &g_m[i];
    q->form = queue_formation(qi);
    if (!n) return;
    for (i = 0; i < n; i++) {
        const float *p = (const float *)(ms[i]->obj + GO_POS);
        float r = ship_radius(ms[i]->obj);
        c[0] += p[0]; c[1] += p[1]; c[2] += p[2];
        if (r > rmax) rmax = r;
        ms[i]->slot[0] = ms[i]->slot[1] = ms[i]->slot[2] = 0.0f;
        ms[i]->slotIx = -1;
        used[i] = 0;
    }
    c[0] /= (float)n; c[1] /= (float)n; c[2] /= (float)n;
    q->org[0] = c[0]; q->org[1] = c[1]; q->org[2] = c[2];
    fx = aim[0] - c[0]; fz = aim[2] - c[2];
    len = f_sqrt(fx * fx + fz * fz);
    if (len > 1.0f) { fx /= len; fz /= len; } else { fx = 0.0f; fz = 1.0f; }
    if (n == 1) { ms[0]->slotIx = 0; return; }     /* one ship: the point itself */

    f = formation_of(q->form);
    if (f && (slots = *(BYTE **)(f + FRM_SLOTS)) != 0) {
        float rad = *(float *)(f + FRM_RADIUS);
        if (rad > 0.0f) scale = rmax / rad;
        cnt = *(int *)(f + FRM_COUNT);
        for (s = 0; s < cnt && placed < n; s++) {
            BYTE *sl = slots + s * SLOT_SIZE;
            const float *o = (const float *)(sl + SLOT_OFFSET);
            float w[3];
            w[0] = c[0] + scale * (o[0] * fz + o[2] * fx);
            w[1] = c[1] + scale * o[1];
            w[2] = c[2] + scale * (-o[0] * fx + o[2] * fz);
            for (j = 0; j < 7; j++) {
                int want = ((const int *)(sl + SLOT_PREFS))[j], best = -1;
                float bd = 0.0f;
                if (want == '-') break;
                for (i = 0; i < n; i++) {
                    const float *p;
                    float dx, dy, dz, d;
                    BYTE *cls;
                    if (used[i]) continue;
                    cls = *(BYTE **)(ms[i]->obj + GO_CLASS);
                    if (want != '?' && (!cls || *(int *)(cls + CLASS_SHIPTYPE) != want)) continue;
                    p = (const float *)(ms[i]->obj + GO_POS);
                    dx = w[0] - p[0]; dy = w[1] - p[1]; dz = w[2] - p[2];
                    d = dx * dx + dy * dy + dz * dz;
                    if (best < 0 || d < bd) { best = i; bd = d; }
                }
                if (best >= 0) {
                    used[best] = 1;
                    ms[best]->slotIx = s;
                    ms[best]->slot[0] = scale * o[0];
                    ms[best]->slot[1] = scale * o[1];
                    ms[best]->slot[2] = scale * o[2];
                    if (lead < 0 || *(int *)(sl + SLOT_RANK)
                                    < *(int *)(slots + ms[lead]->slotIx * SLOT_SIZE + SLOT_RANK))
                        lead = best;
                    placed++;
                    break;
                }
                if (want == '?') break;
            }
        }
    }
    if (lead >= 0) {
        float l0 = ms[lead]->slot[0], l1 = ms[lead]->slot[1], l2 = ms[lead]->slot[2];
        for (i = 0; i < n; i++)
            if (ms[i]->slotIx >= 0) { ms[i]->slot[0] -= l0; ms[i]->slot[1] -= l1; ms[i]->slot[2] -= l2; }
    }
    /* No formation, or ships it has no slot for: where each is in the group now, turned
     * to the heading, at most a few ship widths from the group's centre, and no two
     * closer than a ship's width. */
    {
        float lim = 2.5f * rmax * (float)(1 + f_round(f_sqrt((float)n) + 0.49f)), gap = 2.0f * rmax;
        int   it;
        for (i = 0; i < n; i++) {
            const float *p;
            float dx, dz, ox, oz, d;
            if (ms[i]->slotIx >= 0) continue;
            p = (const float *)(ms[i]->obj + GO_POS);
            dx = p[0] - c[0]; dz = p[2] - c[2];
            ox = dx * fz - dz * fx;            /* along right = (fz, 0, -fx) */
            oz = dx * fx + dz * fz;            /* along the heading */
            d = f_sqrt(ox * ox + oz * oz);
            if (d > lim) { ox *= lim / d; oz *= lim / d; }
            ms[i]->slot[0] = ox; ms[i]->slot[1] = 0.0f; ms[i]->slot[2] = oz;
            layout++;
        }
        for (it = 0; layout && it < 8; it++)
            for (i = 0; i < n; i++) {
                if (ms[i]->slotIx >= 0) continue;
                for (j = 0; j < n; j++) {
                    float dx, dz, d, push;
                    if (j == i) continue;
                    dx = ms[i]->slot[0] - ms[j]->slot[0];
                    dz = ms[i]->slot[2] - ms[j]->slot[2];
                    d = f_sqrt(dx * dx + dz * dz);
                    if (d >= gap) continue;
                    if (d < 0.01f) { dx = (float)(i - j); dz = 0.0f; d = dx < 0 ? -dx : dx; }
                    push = (gap - d) / d;
                    if (ms[j]->slotIx < 0) push *= 0.5f;
                    ms[i]->slot[0] += dx * push;
                    ms[i]->slot[2] += dz * push;
                }
            }
    }
    b[0] = 0;
    s_cat(b, "move queue: queue ");
    s_num(b, q->serial);
    s_cat(b, " spread (");
    s_cat(b, why);
    s_cat(b, "): formation ");
    s_num(b, q->form);
    s_cat(b, ", scale ");
    s_fixed(b, scale, 2);
    s_cat(b, ", ");
    s_num(b, placed);
    s_cat(b, " ship(s) in slots, ");
    s_num(b, layout);
    s_cat(b, " by layout");
    mq_log(b);
    for (i = 0; i < n; i++) log_place(q, ms[i]);
}

/* The way a leg from `a` to `b` heads, or `keep` when it is too short to tell. */
static void heading(const float *a, const float *b, const float *keep, float *out)
{
    float dx = b[0] - a[0], dz = b[2] - a[2], len = f_sqrt(dx * dx + dz * dz);
    if (len > 1.0f) { out[0] = dx / len; out[1] = dz / len; }
    else if (keep) { out[0] = keep[0]; out[1] = keep[1]; }
    else { out[0] = 0.0f; out[1] = 1.0f; }
}

/* ---- the acknowledgement ------------------------------------------------- */

/* Craft::Say's call of CaptainsVoice::Say, re-pointed: a ship taking a command speaks
 * its acknowledgement there (from the processes' Handle_New_Command, as the order is
 * carried out, and only for a ship of this machine's player: CaptainsVoice::Say checks
 * GameObject::IsLocal), so it is left out, here only, for a ship whose order of QOL's own
 * (not a click's) has just been carried out and not yet taken up by its process. */
typedef BYTE (__thiscall *SayFn)(BYTE *cv, int topic, BYTE *obj);
#define SAY            ((SayFn)a2s_qol_mq_say)               /* CaptainsVoice::Say */

BYTE __thiscall mq_say_hook(BYTE *cv, int topic, BYTE *obj)
{
    Member *m = g_mqLive ? member_of_obj(obj) : 0;
    if (m && m->mute) {
        if (!m->muted) {
            char b[120];
            m->muted = 1;
            b[0] = 0;
            s_cat(b, "move queue: acknowledgement left out, queue ");
            s_num(b, g_q[m->q].serial);
            s_cat(b, " ship ");
            s_num(b, m->id);
            s_cat(b, " (topic ");
            s_num(b, topic);
            s_cat(b, ")");
            mq_log(b);
        }
        return 0;
    }
    return SAY(cv, topic, obj);
}

/* ---- the tick ----------------------------------------------------------- */

/* A ship has gone past queue point k (`what`): how far it is from its own target there. */
static void log_reached(const Member *m, int k, const char *what)
{
    Queue *q = &g_q[m->q];
    const float *p = (const float *)(m->obj + GO_POS);
    float w[3], dx, dy, dz, d, other = -1.0f;
    char  b[180];
    int   i, ok, stop = what[0] == 'a';
    if (k < 0 || k >= q->n) return;
    target(q, m, k, w);
    dx = p[0] - w[0]; dy = p[1] - w[1]; dz = p[2] - w[2];
    d = dx * dx + dy * dy + dz * dz;
    for (i = 0; i < MQ_SHIPS; i++) {           /* the nearest target of another ship there */
        float o[3], e;
        if (g_m[i].q != m->q || &g_m[i] == m) continue;
        target(q, &g_m[i], k, o);
        e = (p[0] - o[0]) * (p[0] - o[0]) + (p[1] - o[1]) * (p[1] - o[1]) + (p[2] - o[2]) * (p[2] - o[2]);
        if (other < 0.0f || e < other) other = e;
    }
    /* stopped at the end of a run: nearer its own target than any other ship's; gone past
     * a point: within the engine's reach of its own (it advances a path about 150 short) */
    ok = stop ? (other < 0.0f || d < other) : d < MQ_PASS_NEAR * MQ_PASS_NEAR;
    if (ok) q->reached++; else q->astray++;
    b[0] = 0;
    s_cat(b, "move queue: queue ");
    s_num(b, q->serial);
    s_cat(b, " ship ");
    s_num(b, m->id);
    s_cat(b, " ");
    s_cat(b, what);
    s_cat(b, " point ");
    s_num(b, k + 1);
    s_cat(b, ", ");
    s_num(b, f_round(f_sqrt(d)));
    s_cat(b, " from its target (");
    s_num(b, f_round(w[0]));
    s_cat(b, ",");
    s_num(b, f_round(w[2]));
    s_cat(b, ")");
    if (!ok) s_cat(b, stop ? ", nearer another ship's" : ", short of it");
    mq_log(b);
}

/* The player chose another formation for a queue's ships (SET_FORMATION leaves the queue
 * alone): the points still ahead take the new slots, sent again from where each ship is. */
static void queue_reform(int qi)
{
    static Member *fly[MQ_SHIPS];
    Queue *q = &g_q[qi];
    int i, t, k = 0, ahead = q->n;
    t = queue_formation(qi);
    if (t == q->form || t < 0) return;
    for (i = 0; i < MQ_SHIPS; i++) {
        Member *m = &g_m[i];
        int c;
        if (m->q != qi) continue;
        c = member_cur(m);
        if (c < ahead) ahead = c;
        if (m->state != MS_WAIT) fly[k++] = m;
    }
    if (ahead >= q->n) { q->form = t; return; }
    spread(qi, q->p[ahead], "formation changed");
    if (k) send_each(qi, fly, k, "run sent");
}

static void mq_tick(void)
{
    DWORD now;
    LONG  lt;
    int   i, team;

    flush_clears();
    if (!g_mqLive) return;
    if (!mq_enabled()) { reset_all("PathStartsAtOnce off"); return; }
    lt = *(volatile LONG *)a2s_qol_mq_load_time;
    if (lt != g_loadTime) { reset_all("a game was loaded"); g_loadTime = lt; return; }
    now = g_simMs;
    team = USER_TEAM();
    for (i = 0; i < MQ_SHIPS; i++) {
        Member *m = &g_m[i];
        BYTE *o;
        int   ix, cur;
        (void)ix;
        if (m->q < 0) continue;
        o = ENTITY_GET(m->id);
        if (o != m->obj || !is_obj(o) || o[GO_DYING] || *(int *)(o + GO_TEAM) != team) {
            char b[96];
            int  qi = m->q;
            drop_member(m);
            b[0] = 0;
            s_cat(b, "move queue: ship gone, queue ");
            s_num(b, g_q[qi].serial);
            s_cat(b, ": ");
            s_num(b, queue_size(qi));
            s_cat(b, " ship(s) left");
            mq_log(b);
            continue;
        }
        switch (m->state) {
        case MS_POSTED:
            if (now - m->t > MQ_POST_MS) m->state = MS_WAIT;
            break;
        case MS_EXEC:
            m->last = m->off;
            if (flies_order(m, &ix)) { m->state = MS_FLYING; m->last = ix; m->mute = 0; }
            else if (now - m->t > MQ_EXEC_MS) {
                m->state = MS_WAIT;
                m->mute = 0;
                mq_log("move queue: an order executed, its path never flown (arrived at once, or refused)");
            }
            break;
        case MS_FLYING:
            if (!flies_order(m, &cur)) {
                log_reached(m, order_last(&g_q[m->q], m), "at the end of its run,");
                m->state = MS_WAIT;
                break;
            }
            if (m->patrol && !loops(&g_q[m->q], m->off)) {
                int rb = run_end(&g_q[m->q], m->off);
                if (m->last == rb - 1 && cur != rb - 1) {
                    log_reached(m, rb - 1, "at the end of its run,");
                    m->state = MS_WAIT;
                    break;
                }
            }
            if (cur != m->last) log_reached(m, m->last, "passed");
            m->last = cur;
            break;
        }
    }
    for (i = 0; i < MQ_QUEUES; i++) if (g_q[i].used) queue_reform(i);
    for (i = 0; i < MQ_QUEUES; i++) if (g_q[i].used) queue_step(i);
}

/* GameObject::PreSimulateAll's call in Simulate, re-pointed: the queues' clock, the
 * simulation's (a paused game's timers stand still). */
void __cdecl mq_presim_hook(float dt)
{
    PRESIM(dt);
    if (dt > 0.0f && dt < 10.0f) {          /* seconds */
        long w;
        static int told;
        if (!told) {
            char b[64];
            told = 1;
            b[0] = 0;
            s_cat(b, "move queue: simulation step ");
            s_fixed(b, dt * 1000.0f, 1);
            s_cat(b, " ms");
            mq_log(b);
        }
        g_simFrac += dt * 1000.0f;
        w = f_round(g_simFrac - 0.5f);
        if (w > 0) { g_simMs += (DWORD)w; g_simFrac -= (float)w; }
    }
}

/* cOverViewImp::Simulate's call of mProcessKeyboardInput, re-pointed: the queues step
 * once a frame after it, where the player's own clicks and keys give their orders.  An
 * order sent from inside the simulation step (after PreSimulateAll) is never carried
 * out, so the step is here, not there. */
void __thiscall mq_ov_keys_hook(BYTE *ov)
{
    OV_KEYS(ov);
    mq_tick();
}

/* ---- adding points ------------------------------------------------------ */

/* The mode of a leg to `pt` appended after `prev`: what the click carried, except that
 * a move QOL-8 would send to warp is judged by the leg's own length, not by how far the
 * ships are from the point now. */
static int leg_mode(int cmd, const float *prev, const float *pt, Member **ms, int k, int minimap)
{
    float dx, dy, dz;
    int   i;
    if (cmd == MP_CMD_PATROL) return MQ_PATROL;
    if (minimap || !prev || **(int **)COMMAND_ALT || !(g_warpDistance > 0.0f))
        return cmd == CMD_GO_WARP ? MQ_WARP : MQ_GO;
    if (!warp_allowed()) return MQ_GO;
    dx = pt[0] - prev[0]; dy = pt[1] - prev[1]; dz = pt[2] - prev[2];
    if (!(f_sqrt(dx * dx + dy * dy + dz * dz) > g_warpDistance)) return MQ_GO;
    for (i = 0; i < k; i++) if (can_warp(ms[i]->obj)) return MQ_WARP;
    return MQ_GO;
}

/* Appends pt to queue qi, whose ships are ms[0..k). */
static void append_point(int qi, const float *pt, int cmd, Member **ms, int k, int minimap)
{
    static Member *tail[MQ_SHIPS];
    Queue *q = &g_q[qi];
    int    old = q->n, mode, i, nt = 0, ahead;
    char   b[160];

    if (old >= MQ_PTS) {
        b[0] = 0;
        s_cat(b, "move queue: queue ");
        s_num(b, q->serial);
        s_cat(b, " is full, point not added");
        mq_log(b);
        return;
    }
    mode = leg_mode(cmd, old ? q->p[old - 1] : 0, pt, ms, k, minimap);
    q->p[old][0] = pt[0]; q->p[old][1] = pt[1]; q->p[old][2] = pt[2];
    q->mode[old] = (BYTE)mode;
    if (!old) {
        spread(qi, pt, "new queue");
        heading(q->org, pt, 0, q->dir[0]);
    } else heading(q->p[old - 1], pt, q->dir[old - 1], q->dir[old]);
    q->n = old + 1;

    ahead = q->n;
    for (i = 0; i < k; i++) { int c = member_cur(ms[i]); if (c < ahead) ahead = c; }
    b[0] = 0;
    s_cat(b, "move queue: appended point ");
    s_num(b, q->n);
    s_cat(b, " (");
    s_cat(b, mode_name(mode));
    s_cat(b, "), queue ");
    s_num(b, q->serial);
    s_cat(b, ": ");
    s_num(b, q->n - ahead);
    s_cat(b, " point(s), ships ");
    s_num(b, k);
    mq_log(b);

    /* the ships flying the queue's last run take the point at once when it is of that
     * run's mode: the order is sent again, from the point each heads to */
    if (old && q->mode[old - 1] == mode)
        for (i = 0; i < k; i++)
            if (ms[i]->state != MS_WAIT && ms[i]->end == old) tail[nt++] = ms[i];
    if (nt) send_each(qi, tail, nt, "run sent");
    queue_step(qi);
}

/* A point for the selection's queues: 1 when taken. */
static int mq_point(BYTE *ov, const float *pt, int cmd, int minimap)
{
    typedef const int *(__thiscall *ListFn)(BYTE *);
    static Member *ms[MQ_SHIPS];
    static int     qsel[MQ_QUEUES], qall[MQ_QUEUES], qsplit[MQ_QUEUES];
    const int *sel;
    int   k, i, j, n, team, fresh = -1, took = 0;

    if (!mq_enabled() || !pt) return 0;
    if (cmd != CMD_GO && cmd != CMD_GO_WARP && cmd != MP_CMD_PATROL) return 0;
    k = *(int *)(ov + OV_COUNT);
    if (k <= 0) return 0;
    if (k > SEL_MAX) k = SEL_MAX;
    sel = ((ListFn)(*(BYTE ***)ov)[OV_GETLIST])(ov);
    team = USER_TEAM();
    if (*(volatile LONG *)a2s_qol_mq_load_time != g_loadTime) {
        reset_all("a game was loaded");
        g_loadTime = *(volatile LONG *)a2s_qol_mq_load_time;
    }
    flush_clears();

    for (i = 0; i < MQ_QUEUES; i++) { qsel[i] = 0; qall[i] = 0; qsplit[i] = 0; }
    for (i = 0; i < MQ_SHIPS; i++) if (g_m[i].q >= 0) qall[g_m[i].q]++;

    /* the selected ships of the player's that can fly a path; new ones join a fresh queue */
    for (i = 0; i < k; i++) {
        BYTE *o = ENTITY_GET(sel[i]);
        Member *m;
        if (!is_obj(o) || o[GO_DYING] || *(int *)(o + GO_TEAM) != team) continue;
        if (files_as_station(o) || !*(BYTE **)(o + MP_PROCESS)) continue;
        m = member_of_obj(o);
        if (m) { qsel[m->q]++; continue; }
        if (fresh < 0 && (fresh = new_queue()) < 0) break;
        for (j = 0; j < MQ_SHIPS && g_m[j].q >= 0; j++) {}
        if (j == MQ_SHIPS) break;
        m = &g_m[j];
        m->q = fresh; m->id = sel[i]; m->obj = o;
        m->state = MS_WAIT; m->end = 0; m->off = 0; m->cnt = 0; m->last = 0; m->patrol = 0;
        g_mqLive++;
    }

    /* a queue only some of whose ships are selected: those ships go on in a copy of it */
    for (i = 0; i < MQ_QUEUES; i++) {
        int cp;
        if (!qsel[i] || qsel[i] == qall[i]) continue;
        cp = new_queue();
        if (cp < 0) { qsel[i] = 0; continue; }
        n = g_q[cp].serial;
        {
            BYTE *d = (BYTE *)&g_q[cp];
            const BYTE *f = (const BYTE *)&g_q[i];
            unsigned z;
            for (z = 0; z < sizeof(Queue); z++) d[z] = f[z];
        }
        g_q[cp].serial = n;
        qsplit[cp] = g_q[i].serial;
        for (j = 0; j < k; j++) {
            BYTE *o = ENTITY_GET(sel[j]);
            Member *m = member_of_obj(o);
            if (m && m->q == i) m->q = cp;
        }
        qsel[cp] = qall[cp] = qsel[i];     /* whole now: not split again further on */
        qsel[i] = 0;
    }
    if (fresh >= 0) qsel[fresh] = 1;

    for (i = 0; i < MQ_QUEUES; i++) {
        if (!qsel[i] || !g_q[i].used) continue;
        for (n = 0, j = 0; j < MQ_SHIPS; j++) if (g_m[j].q == i) ms[n++] = &g_m[j];
        if (!n) { g_q[i].used = 0; continue; }
        if (i == fresh || qsplit[i]) {
            char b[80];
            b[0] = 0;
            s_cat(b, "move queue: new queue ");
            s_num(b, g_q[i].serial);
            if (qsplit[i]) { s_cat(b, " (split from queue "); s_num(b, qsplit[i]); s_cat(b, ")"); }
            s_cat(b, ", ships ");
            s_num(b, n);
            mq_log(b);
        }
        g_inClick = 1;
        append_point(i, pt, cmd, ms, n, minimap);
        g_inClick = 0;
        took = 1;
    }
    return took;
}

/* mNormalModeRightClick's two calls of mAddPathPoint, re-pointed: the sketch is kept as
 * stock keeps it (the engine draws it while the key is held), and the point goes to the
 * queues. */
BYTE *__thiscall mp_add_hook(BYTE *ov, BYTE *path, int after, const float *pt)
{
    BYTE *r = ADD_POINT(ov, path, after, pt);
    int   a = *(int *)(ov + OV_ACTION);
    if (!path) g_sketchTaken = 0;
    if (!(a & MP_QUEUED) && mq_point(ov, pt, a & ACT_COMMAND, 0)) g_sketchTaken = 1;
    return r;
}

/* mProcessWaypoints' call of QueueCommand, re-pointed: left out when the sketch's points
 * went to the queues. */
void __cdecl mp_release_hook(int cmd, void *ids, BYTE *path)
{
    int skip = g_sketchTaken && mq_enabled();
    g_sketchTaken = 0;
    if (skip) {
        mq_log("move queue: key let go, the points are queued already");
        return;
    }
    QUEUE_PATH(cmd, ids, path);
}

/* ---- exports ------------------------------------------------------------- */

/* For the UI's minimap: a right-click there at xyz with the waypoint key held adds the
 * point to the selection's queues, as one in the 3D view does.  `action` is the order the
 * click would give (GO, GO_WARP or PATROL in its low twelve bits).  1: taken, send
 * nothing; 0: not a queue point, give the stock order.  Game thread. */
__declspec(dllexport) int __cdecl QOL_MoveQueuePoint(const float *xyz, int action)
{
    BYTE *ov;
    int  *key;
    char  b[96];
    if (!mq_enabled() || !xyz || (action & MP_QUEUED)) return 0;
    ov = (BYTE *)ov_base();
    if (!ov) return 0;
    key = *(int **)(ov + OV_KEY);
    if (!key || !*key) return 0;
    b[0] = 0;
    s_cat(b, "move queue: minimap point at ");
    s_num(b, f_round(xyz[0]));
    s_cat(b, ",");
    s_num(b, f_round(xyz[2]));
    mq_log(b);
    return mq_point(ov, xyz, action & ACT_COMMAND, 1);
}

__declspec(dllexport) int __cdecl QOL_MoveQueueVersion(void) { return 1; }

/* The points still ahead of m, in order, from the one it heads to now: its own targets
 * into own[], the queue's points into pts[] (either may be null), flags as below. */
static int queue_ahead(const Member *m, float *own, float *pts, int *flags, int cap)
{
    const Queue *q = &g_q[m->q];
    int cur, i, k = 0, ra, rb, L, idx;

    cur = member_cur(m);
    if (cur >= q->n) return 0;
    if (loops(q, cur)) {
        ra = run_start(q, cur); rb = q->n; L = rb - ra;
        for (i = 0; i < L; i++, k++) {
            idx = ra + (cur - ra + i) % L;
            if (k < cap) {
                if (own) target(q, m, idx, own + 3 * k);
                if (pts) { pts[3 * k] = q->p[idx][0]; pts[3 * k + 1] = q->p[idx][1]; pts[3 * k + 2] = q->p[idx][2]; }
                if (flags) flags[k] = 2;
            }
        }
        return k;
    }
    for (i = cur; i < q->n; i++, k++)
        if (k < cap) {
            if (own) target(q, m, i, own + 3 * k);
            if (pts) { pts[3 * k] = q->p[i][0]; pts[3 * k + 1] = q->p[i][1]; pts[3 * k + 2] = q->p[i][2]; }
            if (flags) flags[k] = (q->mode[i] == MQ_WARP ? 1 : 0) | (loops(q, i) ? 2 : 0);
        }
    return k;
}

/* For render3d: the points still ahead of `ship` (a GameObject *), in order, from the one
 * it heads to now, each where this ship goes (the point plus its place in the group);
 * flags bit 0 a warp leg, bit 1 a leg of a patrol that loops back to its run's first
 * point.  -1: no queues kept here; 0: none for this ship; else the count (at most `cap`
 * written).  Game thread, every frame: no allocation, no log. */
__declspec(dllexport) int __cdecl QOL_MoveQueue(const void *ship, float *xyz, int *flags, int cap)
{
    const Member *m;
    if (!mq_enabled()) return -1;
    m = member_of_obj(ship);
    if (!m) return 0;
    return queue_ahead(m, xyz, 0, flags, cap);
}

/* As QOL_MoveQueue, and the queue's own points (where the group's leader goes) into
 * `pts` and the queue's serial into *queue, so that several ships of one group can be
 * drawn as one path.  Optional for the caller: found by name beside QOL_MoveQueue. */
__declspec(dllexport) int __cdecl QOL_MoveQueueGroup(const void *ship, float *own, float *pts,
                                                     int *flags, int cap, int *queue)
{
    const Member *m;
    if (!mq_enabled()) return -1;
    m = member_of_obj(ship);
    if (!m) return 0;
    if (queue) *queue = g_q[m->q].serial;
    return queue_ahead(m, own, pts, flags, cap);
}

static int patch_paths(void)
{
    int i;
    if (!g_rdyPaths) return 0;
    for (i = 0; i < MQ_SHIPS; i++) g_m[i].q = -1;
    g_loadTime = *(volatile LONG *)a2s_qol_mq_load_time;
    return a2_write_call(a2e_addr(QOL_S_MP_ADD_CALL), mp_add_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MP_FIRST_CALL), mp_add_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MP_RELEASE_CALL), mp_release_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHAT_CALL), mq_dq_what_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHAT_ON_CALL), mq_dq_what_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHO_CALL), mq_dq_who_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHO_WHERE_CALL), mq_dq_who_where_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHO_WHERE_PARAM_CALL), mq_dq_wwp_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_CLASS_CALL), mq_dq_class_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_CLASS_WHERE_CALL), mq_dq_cw_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_PARAM_CALL), mq_dq_param_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHERE_CALL), mq_dq_where_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHERE_PARAM_CALL), mq_dq_wp_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_WHO_PARAM_CALL), mq_dq_who_param_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_SPECIAL_CALL), mq_special_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_GO_BROADCAST_CALL), mq_go_broadcast_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_FORM_FORMATION_CALL), mq_form_formation_stub, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_PATH_CALL), mq_dq_path_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_PRESIM_CALL), mq_presim_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_OV_KEYS_CALL), mq_ov_keys_hook, 5)
        && a2_write_call(a2e_addr(QOL_S_MQ_SAY_CALL), mq_say_hook, 5);
}
