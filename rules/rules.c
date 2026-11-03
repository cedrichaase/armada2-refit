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
 * would desync.  The plugin applies its rules in a game against the computer, and
 * in a network game only when the online layer reports that every player runs it
 * (QOLRules_Wanted / _Network / _Reset, called by Online.asi); otherwise it stands
 * down.
 *
 * HOW STOCK QUEUES (Producer; the class has no name for this in the log, so)
 * ---------------------------------------------------------------------------
 * A producer keeps its queue as a linked list at +0x270 (count at +0x274), at
 * most ten items of 16 bytes: the class to build, the previous and next item,
 * and an id (+0xc) that the delete order carries over the network.  The head is
 * the item being built once StartBuild has run: +0x254 is then its class and
 * +0x2a0 its id.  Costs leave the bank in StartBuild only, which refuses (and the
 * advisor says so) when the bank is short; CancelBuild gives an
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
 *     alone: they queue and start at once and expect it to succeed.  A refused
 *     order speaks stock's advisor line for its first shortfall, as StartBuild
 *     would have (advise(); it plays only for the local team).
 *   - StartBuild: a marked head is given back first, the mark is cleared, and
 *     stock charges it as it always did -- it cannot fail, and the advisor stays
 *     quiet.  Should it fail anyway the item is taken again and keeps its mark.
 *   - Pop, CancelBuild (waiting head), ActDeleteBuildQueueItem, ClearBuildQueue:
 *     a marked item that leaves the queue is given back.  A delete order matches
 *     ids with the mark masked: one sent while the item waited still finds it
 *     after StartBuild has cleared the mark.
 *   - ClearTeam (capture, assimilation, and a destroyed producer: GameObject::
 *     Cleanup always calls it when the dead object goes): every marked item is
 *     given back, and the item in progress too -- in the old owner's bank, since
 *     ClearTeam runs before the team changes.  Stock ClearTeam then zeroes the
 *     item in progress, so this happens once.  Its officers are left out when
 *     stock's own mRemoveConstructionEffect returns them (IsBusy, and for
 *     Producer's version a live construction effect at +0x268; Evolver's
 *     always), or they would be discharged twice.  DestroyShip is not hooked: it
 *     would refund the same items again at Cleanup, and a craft that cannot die
 *     (+0x182) passes through it on every fatal hit.
 *   - Program::GameOpenPreLoad (every game, new or loaded): the network latch
 *     below is cleared, so a second game in the same session asks again.
 *   - Save / Load: the marks are written into the producer's id counter (a field
 *     the stock file already holds) as a tag and a bitmask by position, and read
 *     back after Load.  A stock save has no tag and loads as unpaid queues, which
 *     stock rules charge as each item starts.
 *
 * Patched in memory only.  Every entry is checked against this build's bytes
 * first; a different Armada2.exe leaves the plugin out and says so in the log.
 */

#define A2_FLTUSED
#include "../csdk/a2asi.h"
#include "../csdk/a2engine.h"
#include "settings_table.h"
#include "engine_syms.h"

/* ---- Producer layout and the engine's own functions (Armada2.exe, patch 1.1) */

#define P_TEAMIDX   QOLRULES_F_P_TEAMIDX     /* the owner's team number */
#define P_TEAM      QOLRULES_F_P_TEAM     /* its Team* */
#define P_CUR       QOLRULES_F_P_CUR    /* class being built, or 0 */
#define P_HEAD      QOLRULES_F_P_HEAD    /* queue head: item {class, prev, next, id} */
#define P_COUNT     QOLRULES_F_P_COUNT
#define P_CURID     QOLRULES_F_P_CURID    /* id of the item in progress */
#define P_NEXTID    QOLRULES_F_P_NEXTID    /* the next id to hand out */
#define Q_MAX       QOLRULES_V_Q_MAX
#define PAID        0x40000000   /* bit 30 of an item's id: paid at enqueue */
#define SAVE_TAG    0x10000000   /* bit 28 of the saved counter: marks follow in bits 16-25 */

#define IT_CLS(it)  (*(void **)(it))
#define IT_NEXT(it) (*(BYTE **)((it) + QOLRULES_F_IT_NEXT))
#define IT_ID(it)   (*(int *)((it) + QOLRULES_F_IT_ID))

#define G_TRANSPORT a2s_qolrules_transport   /* Transport * (g_transport) */
#define VT_LOCAL         a2s_qolrules_vt_local          /* TransportLocal: the campaign */
#define VT_LOCAL_INSTANT a2s_qolrules_vt_local_instant  /* TransportLocalInstantAction: skirmish */

typedef int   (__thiscall *CostFn)(void *cls, int team);
typedef float (__thiscall *AddFn)(void *team, float v);
typedef void  (__thiscall *CrewFn)(void *team, float v);
typedef void  (__thiscall *OffFn)(void *team, int n);
typedef int   (__thiscall *VacFn)(void *team);
typedef void  (__thiscall *PushFn)(BYTE *p, void *cls);

#define COST_DIL  ((CostFn)a2s_qolrules_cost_dil)   /* GameObjectClass::Get_Dilithium_Cost(team) */
#define COST_LAT  ((CostFn)a2s_qolrules_cost_lat)
#define COST_MET  ((CostFn)a2s_qolrules_cost_met)
#define COST_BIO  ((CostFn)a2s_qolrules_cost_bio)
#define COST_OFF  ((CostFn)a2s_qolrules_cost_off)
#define COST_CREW ((CostFn)a2s_qolrules_cost_crew)
#define ADD_DIL   ((AddFn)a2s_qolrules_add_dil)    /* Team::AddDilithium */
#define ADD_LAT   ((AddFn)a2s_qolrules_add_lat)
#define ADD_MET   ((AddFn)a2s_qolrules_add_met)
#define ADD_BIO   ((AddFn)a2s_qolrules_add_bio)
#define ADD_CREW  ((CrewFn)a2s_qolrules_add_crew)
#define ENLIST    ((OffFn)a2s_qolrules_enlist)    /* Team::EnlistOfficers */
#define DISCHARGE ((OffFn)a2s_qolrules_discharge)
#define VACANCIES ((VacFn)a2s_qolrules_vacancies)    /* Team::GetOfficerVacancies */
#define ENGINE_PUSH ((PushFn)a2s_qolrules_push_item) /* Producer::PushBuildQueueItem */
#define EPSILON   (*(volatile double *)a2s_qolrules_epsilon)

typedef struct { int dil, lat, met, bio, crew, off; } Cost;

/* ---- tiny string/log helpers (no CRT) --------------------------------- */

static char g_logpath[320];
static int  g_logging = 1;

static void logline(const char *s)
{
    if (!g_logging || !g_logpath[0]) return;
    a2_append_line(g_logpath, s);
}

/* ---- costs and the bank ------------------------------------------------ */

static A2Ini g_ini;   /* QOLRules.ini, with RefitPlayer.ini in front of it */
static int g_pay;     /* the rules are in force in this game: PayOnQueue was 1 as it began */
static int g_want;    /* PayOnQueue as the player has it now; g_pay takes it at the next game */
static int g_patched; /* the hooks are in (they stay in, gated by g_pay) */
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

/* A game against the computer: both transports of the local kind.  A network game
 * plays by the rules only if every player runs them: the online layer
 * (online/peer.c, R_RULES) says so through QOLRules_Network, and the answer is
 * latched at the first use in a game, so a late change cannot split the peers.
 * Every game start clears the latch (h_open). */
static int g_net, g_latched, g_latch;

static int rules_on(void)
{
    BYTE *t = *(BYTE **)G_TRANSPORT;
    DWORD vt;
    if (!g_pay || !t) return 0;
    vt = *(DWORD *)t;
    if (vt == VT_LOCAL || vt == VT_LOCAL_INSTANT) return 1;
    if (!g_latched) { g_latched = 1; g_latch = g_net; }
    return g_latch;
}

/* for the online layer (Online.asi finds them by name) */
__declspec(dllexport) int  __cdecl QOLRules_Wanted(void)    { return g_want && g_patched; }
__declspec(dllexport) void __cdecl QOLRules_Network(int on) { g_net = on != 0; }
__declspec(dllexport) void __cdecl QOLRules_Reset(void)     { g_net = 0; g_latched = 0; g_latch = 0; }

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

/* StartBuild's own test, field for field and in its order: a team with 0x16c clear
 * counts as having nothing; dilithium has a small margin; officers are vacancies,
 * not a stock.  0 if the bank covers it, else the first thing it is short of. */
enum { SHORT_NONE, SHORT_DIL, SHORT_OFF, SHORT_CREW, SHORT_LAT, SHORT_MET, SHORT_BIO };

static int short_of(const BYTE *team, const Cost *c)
{
    float d = 0, m = 0, l = 0, b = 0, k = 0;
    if (*(const int *)(team + QOLRULES_F_T_BANK)) {
        d = *(const float *)(team + QOLRULES_F_T_DIL);
        m = *(const float *)(team + QOLRULES_F_T_MET);
        l = *(const float *)(team + QOLRULES_F_T_LAT);
        b = *(const float *)(team + QOLRULES_F_T_BIO);
        k = *(const float *)(team + QOLRULES_F_T_CREW);
    }
    if ((double)d + EPSILON < (double)c->dil) return SHORT_DIL;
    if (c->off && c->off > VACANCIES((void *)team)) return SHORT_OFF;
    if ((float)c->crew > k) return SHORT_CREW;
    if ((float)c->lat > l) return SHORT_LAT;
    if ((float)c->met > m) return SHORT_MET;
    if ((float)c->bio > b) return SHORT_BIO;
    return SHORT_NONE;
}

/* ---- the advisor ----------------------------------------------------------
 * A refused order says so the way stock's StartBuild says it when the bank is
 * short: the same ComputerVoice call, the same topic per resource (and for
 * officers the race's own event), with the producer as the object.  Everything
 * stock does around it stays stock's: GameEvent::TriggerEvent plays only for the
 * local player's team (Entity::GetUserTeam), at most once per the event's
 * interval (wall clock), and picks the sound with g_asyncRandom -- nothing the
 * simulation reads, so it is safe on every node of a network game. */
typedef void (__cdecl *SayFn)(int topic, void *obj, const void *race);
typedef int  (__thiscall *RaceEventFn)(const void *race);

#define SAY_TOPIC     ((SayFn)a2s_qolrules_say_topic)        /* ComputerVoice::Say(Topic, GameObject *, const Race *) */
#define SAY_EVENT     ((SayFn)a2s_qolrules_say_event)        /* ComputerVoice::Say(int, GameObject *, const Race *) */
#define OFFICER_EVENT ((RaceEventFn)a2s_qolrules_officer_event)  /* Race::GetInsufficentOfficerEventId */
#define T_RACE        QOLRULES_F_T_RACE                    /* Team: its Race * */

static int g_voice;   /* StartBuild's calls are the ones this was read from */

static const int k_topic[] = {
    0,
    QOLRULES_V_TOPIC_DIL,   /* InsufficientDilithium */
    0,                      /* officers: the race's event */
    QOLRULES_V_TOPIC_CREW,  /* InsufficientCrew */
    QOLRULES_V_TOPIC_LAT,   /* InsufficientLatinum */
    QOLRULES_V_TOPIC_MET,   /* InsufficientMetal */
    QOLRULES_V_TOPIC_BIO,   /* InsufficientBiomatter */
};

typedef long (__cdecl *UserTeamFn)(void);
#define USER_TEAM ((UserTeamFn)a2s_qolrules_user_team)       /* Entity::GetUserTeam: the local player's */

static void advise(BYTE *p, BYTE *team, int why)
{
    char m[120];
    if (!g_voice || why == SHORT_NONE) return;
    m[0] = 0;
    s_cat(m, "  advisor: topic ");
    s_num(m, why == SHORT_OFF ? -1 : k_topic[why]);
    s_cat(m, *(const int *)(p + P_TEAMIDX) == (int)USER_TEAM()
             ? " for the local team (stock plays it unless it just did)"
             : " for another team (stock keeps it silent here)");
    logline(m);
    if (why == SHORT_OFF) {
        const void *race = *(void *const *)(team + T_RACE);
        SAY_EVENT(OFFICER_EVENT(race), p, race);
    } else {
        SAY_TOPIC(k_topic[why], p, NULLPTR);
    }
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
    else *(float *)(team + QOLRULES_F_T_CREW) += (float)c->crew;
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
    s_num(m, team ? f_round(*(const float *)(team + QOLRULES_F_T_DIL)) : 0);
    s_cat(m, " crew ");
    s_num(m, team ? f_round(*(const float *)(team + QOLRULES_F_T_CREW)) : 0);
    s_cat(m, " officers free ");
    s_num(m, team ? VACANCIES((void *)team) : 0);
    logline(m);
}

/* Give a class's cost back to the producer's owner; officers only with `off`. */
static void give_back(BYTE *p, void *cls, const char *why, int off)
{
    BYTE *team = *(BYTE **)(p + P_TEAM);
    Cost  c;
    if (!team || !cls) return;
    cost_of(p, cls, &c);
    if (!off) c.off = 0;
    charge(team, &c, +1);
    note(why, team, &c);
}

/* ---- the queue ---------------------------------------------------------- */

static int is_paid(const BYTE *it) { return it && (IT_ID(it) & PAID); }

/* Every marked item goes back, and the mark is cleared. */
static void drain(BYTE *p, const char *why)
{
    BYTE *it;
    for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it)) {
        if (!is_paid(it)) continue;
        give_back(p, IT_CLS(it), why, 1);
        IT_ID(it) &= ~PAID;
    }
}

/* Does stock ClearTeam give the officers of the item in progress back itself?
 * It calls mRemoveConstructionEffect (+0x170) when IsBusy (+0x138); Producer's
 * discharges only while the construction effect (+0x268) is live, Evolver's
 * always. */
#define VT_REMOVE_EFFECT a2s_qolrules_remove_effect   /* Producer::mRemoveConstructionEffect */
#define P_EFFECT         QOLRULES_F_P_EFFECT

static int stock_returns_officers(BYTE *p)
{
    const DWORD *vt = *(const DWORD **)p;
    if (!((BYTE (__thiscall *)(BYTE *))vt[QOLRULES_F_VT_IS_BUSY / 4])(p)) return 0;
    return vt[QOLRULES_F_VT_REMOVE_EFFECT / 4] != VT_REMOVE_EFFECT || *(const int *)(p + P_EFFECT) != -1;
}

/* ---- hooks -------------------------------------------------------------- */

typedef BYTE (__thiscall *BoolFn)(BYTE *);
typedef void (__thiscall *VoidFn)(BYTE *);
typedef void (__thiscall *IntFn)(BYTE *, int);
typedef BYTE (__thiscall *PtrFn)(BYTE *, void *);
typedef void (__thiscall *OpenFn)(BYTE *, void *);

static BoolFn t_start, t_cancel;
static VoidFn t_pop, t_clearq, t_clearteam;
static IntFn  t_actdel;
static PtrFn  t_save, t_load;
static OpenFn t_open;

/* in place of `call Producer::PushBuildQueueItem` in Handle_New_Command */
static const char *const k_short[] = {
    "", "refused (dilithium)", "refused (officers)", "refused (crew)",
    "refused (latinum)", "refused (metal)", "refused (biomatter)",
};

static void __thiscall h_push(BYTE *p, void *cls)
{
    BYTE *team = *(BYTE **)(p + P_TEAM);
    Cost  c;
    int   why;

    if (!cls || !team || !rules_on() || *(int *)(p + P_COUNT) >= Q_MAX) {
        ENGINE_PUSH(p, cls);
        return;
    }
    cost_of(p, cls, &c);
    why = short_of(team, &c);
    if (why != SHORT_NONE) {
        note(k_short[why], team, &c);
        advise(p, team, why);
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
    if (paid && *(BYTE **)(p + P_HEAD) != head) give_back(p, cls, "popped", 1);
}

static BYTE __thiscall h_cancel(BYTE *p)
{
    BYTE *head = *(BYTE **)(p + P_HEAD);
    BYTE *team = *(BYTE **)(p + P_TEAM);
    void *cls = head ? IT_CLS(head) : NULLPTR;
    void *cur = *(void **)(p + P_CUR);
    int   paid = !cur && is_paid(head);
    float crew0 = team ? *(float *)(team + QOLRULES_F_T_CREW) : 0.0f;
    BYTE  r;

    trace("CancelBuild", p);
    r = t_cancel(p);
    if (paid && *(BYTE **)(p + P_HEAD) != head) {
        give_back(p, cls, "cancelled", 1);
    } else if (cur && team && !*(void **)(p + P_CUR) && rules_on()) {
        /* the item in progress: stock gives it back in full except crew, which
         * AddCrew clamps to the cap -- what the cancel did not return is added */
        Cost  c;
        float got = *(float *)(team + QOLRULES_F_T_CREW) - crew0;
        cost_of(p, cur, &c);
        if (got < (float)c.crew) *(float *)(team + QOLRULES_F_T_CREW) += (float)c.crew - got;
        note("cancelled in progress", team, &c);
    }
    return r;
}

/* The order carries the id as it was when sent; StartBuild may have cleared the
 * mark since.  Ids are matched with the mark masked and passed on as they are now. */
static void __thiscall h_actdel(BYTE *p, int id)
{
    trace("ActDeleteBuildQueueItem", p);
    BYTE *it;
    void *cls = NULLPTR;
    int   paid = 0, bare = id & ~PAID;

    if (*(void **)(p + P_CUR) && bare == (*(int *)(p + P_CURID) & ~PAID)) {
        id = *(int *)(p + P_CURID);
    } else {
        for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it))
            if ((IT_ID(it) & ~PAID) == bare) {
                id = IT_ID(it); paid = is_paid(it); cls = IT_CLS(it); break;
            }
    }
    t_actdel(p, id);
    if (paid) {
        for (it = *(BYTE **)(p + P_HEAD); it; it = IT_NEXT(it))
            if (IT_ID(it) == id) return;      /* still queued: nothing left */
        give_back(p, cls, "deleted", 1);
    }
}

static void __thiscall h_clearq(BYTE *p)
{
    trace("ClearBuildQueue", p);
    drain(p, "cleared");
    t_clearq(p);
}

/* Capture, assimilation, and every producer's removal (destroyed, or the game
 * closing): stock zeroes the item in progress here, so it is given back once. */
static void __thiscall h_clearteam(BYTE *p)
{
    void *cur = *(void **)(p + P_CUR);

    trace("ClearTeam", p);
    drain(p, "lost");
    if (cur && rules_on()) {
        int off = !stock_returns_officers(p);
        give_back(p, cur, off ? "lost in progress" : "lost in progress (officers by stock)", off);
    }
    t_clearteam(p);
}

/* every game, new or loaded: the network answer is asked again */
static void __thiscall h_open(BYTE *prog, void *name)
{
    g_latched = 0;
    g_latch = 0;
    g_pay = g_want && g_patched;        /* a setting changed since is latched here */
    t_open(prog, name);
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

/* An entry hook: the profile's symbol and how many bytes (whole instructions) the jump
 * replaces. */
typedef struct { DWORD sym; int len; } Entry;

static void poke(BYTE *at, const BYTE *bytes, int n) { a2_poke((DWORD)at, bytes, n); }

/* Entry hooks.  The first `len` bytes of each (whole instructions, no relative
 * ones; the profile checks them) are copied behind a jump back, and the entry jumps
 * to the hook. */
static const Entry k_start    = { QOLRULES_S_ENTRY_START, 6 };
static const Entry k_pop      = { QOLRULES_S_ENTRY_POP, 6 };
static const Entry k_actdel   = { QOLRULES_S_ENTRY_ACTDEL, 5 };
static const Entry k_cancel   = { QOLRULES_S_ENTRY_CANCEL, 6 };
static const Entry k_clearq   = { QOLRULES_S_ENTRY_CLEARQ, 6 };
static const Entry k_clearteam= { QOLRULES_S_ENTRY_CLEARTEAM, 5 };
static const Entry k_save     = { QOLRULES_S_ENTRY_SAVE, 6 };
static const Entry k_load     = { QOLRULES_S_ENTRY_LOAD, 6 };
static const Entry k_open     = { QOLRULES_S_ENTRY_OPEN, 5 };  /* Program::GameOpenPreLoad */

static void *hook(const Entry *e, void *fn)
{
    BYTE *tr = g_tramp[g_ntramp++];
    BYTE  jmp[8];
    DWORD at = a2e_addr(e->sym);
    int   k;

    for (k = 0; k < e->len; k++) tr[k] = ((const BYTE *)at)[k];
    tr[e->len] = 0xE9;
    *(LONG *)(tr + e->len + 1) = (LONG)((at + (DWORD)e->len) - ((DWORD)tr + (DWORD)e->len + 5));
    jmp[0] = 0xE9;
    *(LONG *)(jmp + 1) = (LONG)((DWORD)fn - (at + 5));
    for (k = 5; k < e->len; k++) jmp[k] = 0x90;
    poke((BYTE *)at, jmp, e->len);
    return tr;
}

static int g_rdyPay, g_rdyVoice;   /* a2e_ready, asked once at start, before any write */

static int patch_rules(void)
{
    DWORD old, at;
    BYTE  call[5];

    if (!g_rdyPay) return 0;
    if (!VirtualProtect(g_tramp, sizeof g_tramp, PAGE_EXECUTE_READWRITE, &old)) return 0;

    t_start     = (BoolFn)hook(&k_start, (void *)h_start);
    t_pop       = (VoidFn)hook(&k_pop, (void *)h_pop);
    t_actdel    = (IntFn) hook(&k_actdel, (void *)h_actdel);
    t_cancel    = (BoolFn)hook(&k_cancel, (void *)h_cancel);
    t_clearq    = (VoidFn)hook(&k_clearq, (void *)h_clearq);
    t_clearteam = (VoidFn)hook(&k_clearteam, (void *)h_clearteam);
    t_save      = (PtrFn) hook(&k_save, (void *)h_save);
    t_load      = (PtrFn) hook(&k_load, (void *)h_load);
    t_open      = (OpenFn)hook(&k_open, (void *)h_open);

    /* the push call in Handle_New_Command (its target was checked by the profile) */
    at = a2e_addr(QOLRULES_S_PUSH_CALL);
    call[0] = 0xE8;
    *(LONG *)(call + 1) = (LONG)((DWORD)h_push - (at + 5));
    poke((BYTE *)at, call, 5);
    g_voice = g_rdyVoice;   /* StartBuild's own advisor calls, checked by qolrules.advisor */
    return 1;
}

/* ---- startup -------------------------------------------------------------- */

/* <Plugin>_SettingsNote (tools/settings/README.md, C5): why a row is not running, or NULL;
 * a NULL row asks about the plugin as a whole.  The engine's notes feed it. */
__declspec(dllexport) const char *__cdecl QOLRules_SettingsNote(const A2Setting *row)
{
    return row ? a2e_note_for(row->section, row->key) : a2e_note_plugin();
}

/* Called by the UI's settings hub on the game thread after RefitPlayer.ini changed.
 * PayOnQueue is latched at the next game's start (Program::GameOpenPreLoad), never
 * mid-game: a queue already paid for must be given back by the rules it was paid
 * under.  Online asks QOLRules_Wanted, which follows the new value at once. */
__declspec(dllexport) void __cdecl QOLRules_SettingsChanged(void)
{
    int on = a2_set_int(&g_ini, QOLRULES_ROW_PAYONQUEUE) != 0;
    char b[160];
    if (on == g_want) {
        logline("settings: re-read, nothing changed");
        return;
    }
    g_want = on;
    b[0] = 0;
    s_cat(b, "settings: applied QOLRules.PayOnQueue=");
    s_cat(b, on ? "1" : "0");
    s_cat(b, g_patched ? "  (takes hold at the next game)" : "  (not patched, cannot apply)");
    logline(b);
}

static void startup(void)
{
    char b[200];

    a2_ini_open(&g_ini, "QOLRules.ini", logline);
    if (!g_ini.path[0]) return;
    {
        char dir[320];
        a2_exe_dir(dir, sizeof dir);
        g_logpath[0] = 0; s_cat(g_logpath, dir); s_cat(g_logpath, "QOLRules.log");
        g_logging = (int)a2_ini_int(&g_ini, "Rules", "Log", 1);
        g_trace = g_logging > 1;
        if (g_logging && g_logpath[0])
            a2_log_where(g_logpath, a2_log_path(g_logpath, sizeof g_logpath, dir, "QOLRules.log"));
    }
    a2e_init(&QOLRULES_ENGINE, logline);
    g_rdyPay = a2e_ready(QOLRULES_H_PAY_ON_QUEUE);
    g_rdyVoice = a2e_ready(QOLRULES_H_ADVISOR);
    if (g_trace && g_rdyVoice) {      /* Log=2: where StartBuild's advisor calls were found */
        static const DWORD voice[4] = { QOLRULES_S_VOICE_TOPIC, QOLRULES_S_VOICE_SAY_TOPIC,
                                        QOLRULES_S_VOICE_OFFICER, QOLRULES_S_VOICE_SAY_EVENT };
        char m[60];
        int  i;
        for (i = 0; i < 4; i++) {
            m[0] = 0;
            s_cat(m, "  advisor site ");
            s_hex(m, a2e_addr(voice[i]));
            logline(m);
        }
    }

    g_want = a2_set_int(&g_ini, QOLRULES_ROW_PAYONQUEUE) != 0;
    g_patched = patch_rules();
    b[0] = 0;
    s_cat(b, "--- QOLRules PayOnQueue=");
    if (g_want) {
        s_cat(b, "1");
        if (g_patched) {
            g_pay = 1;
            s_cat(b, "  -> items are paid when queued, patched");
            s_cat(b, g_voice ? "; a refused order is announced as stock announces a stalled build"
                             : "; a refused order is NOT announced (qolrules.advisor stood down)");
        } else {
            s_cat(b, "  NOT PATCHED: ");
            s_cat(b, a2e_note(QOLRULES_H_PAY_ON_QUEUE) ? a2e_note(QOLRULES_H_PAY_ON_QUEUE) : "stood down");
        }
    } else {
        s_cat(b, "0  (items are paid as they start, as in stock)");
    }
    logline(b);
    if (!g_want) logline(g_patched ? "gate: PayOnQueue off (hooks installed, gated)"
                                   : "gate: PayOnQueue not patched, cannot apply");
}

BOOL __stdcall DllMain(HMODULE mod, DWORD reason, void *reserved)
{
    (void)mod; (void)reserved;
    if (reason == 1) startup();
    return TRUE;
}
