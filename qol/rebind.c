/*
 * rebind.c -- the key rebind table: an action of the game's key map moved from one key to
 * another, in memory, while the grid's position keys are on.  Included by qol.c.
 *
 *   [BarKeys]
 *   Rebind = select_next_ship:W>M
 *
 * Each row is `action:FROM>TO` (rows apart by commas): in every chord of `action` (its
 * name as Input.map spells it) the held key FROM becomes TO, the chord's other keys (the
 * modifiers it wants held or up) kept.  Applied while the position keys are the bar's
 * (BarKeys on, the UI's bar drawn: bar_active), put back when they stop; Input.map and
 * RTS_CFG.h are never touched.  A target already held by another chord whose other keys
 * do not exclude ours is refused (logged), and so is an action or key not found.  Key
 * names: A-Z, 0-9, F1-F12, LeftBracket, RightBracket (the log names a chord's keys when
 * FROM is not in it).
 *
 * The engine's side (qol/README.md, "Key rebinds"): the parsed key map is two arrays the
 * input thread tests each frame, the named controls (a name and its state cell) and the
 * discrete maps (a state cell and a chord of up to eight keys).  A key of a chord is the
 * word of its device's state it reads, the bit in that word, and held or up; the keyboard's
 * state is a bit array where a key's bit is its set-1 scancode less one (measured on Roots:
 * W, scancode 0x11, is bit 16 of the first word; M, 0x32, bit 17 of the second).  A rebind
 * writes one key's word and bit in one map, and puts them back.
 */

#define RB_MAX_ROWS 8
#define RB_MAX_HITS 16

typedef struct { char action[40]; DWORD from, to; } RbRow;
typedef struct { int map, key; DWORD word, bit, toWord, toBit; } RbHit;

static int    g_rdyRebind;
static RbRow  g_rbRows[RB_MAX_ROWS];
static int    g_rbN;
static char   g_rbText[256];
static RbHit  g_rbHits[RB_MAX_HITS];
static int    g_rbNHits;
static int    g_rbOn = -1;          /* the state applied: 1 in, 0 out, -1 not yet */

static const struct { const char *name; BYTE code; } k_rbKeys[] = {
    {"Q",0x10},{"W",0x11},{"E",0x12},{"R",0x13},{"T",0x14},{"Y",0x15},{"U",0x16},{"I",0x17},
    {"O",0x18},{"P",0x19},{"A",0x1e},{"S",0x1f},{"D",0x20},{"F",0x21},{"G",0x22},{"H",0x23},
    {"J",0x24},{"K",0x25},{"L",0x26},{"Z",0x2c},{"X",0x2d},{"C",0x2e},{"V",0x2f},{"B",0x30},
    {"N",0x31},{"M",0x32},{"1",0x02},{"2",0x03},{"3",0x04},{"4",0x05},{"5",0x06},{"6",0x07},
    {"7",0x08},{"8",0x09},{"9",0x0a},{"0",0x0b},{"F1",0x3b},{"F2",0x3c},{"F3",0x3d},
    {"F4",0x3e},{"F5",0x3f},{"F6",0x40},{"F7",0x41},{"F8",0x42},{"F9",0x43},{"F10",0x44},
    {"F11",0x57},{"F12",0x58},{"LeftBracket",0x1a},{"RightBracket",0x1b},
};

static char rb_up(char c) { return (c >= 'a' && c <= 'z') ? (char)(c - 32) : c; }

static int rb_same(const char *a, const char *b, int n)    /* case-blind, n chars of a */
{
    int i;
    for (i = 0; i < n; i++) if (rb_up(a[i]) != rb_up(b[i]) || !b[i]) return 0;
    return b[n] == 0;
}

static int rb_key(const char *s, int n)
{
    int i;
    for (i = 0; i < (int)(sizeof k_rbKeys / sizeof k_rbKeys[0]); i++)
        if (rb_same(s, k_rbKeys[i].name, n)) return k_rbKeys[i].code;
    return -1;
}

static const char *rb_name(DWORD code)
{
    int i;
    for (i = 0; i < (int)(sizeof k_rbKeys / sizeof k_rbKeys[0]); i++)
        if (k_rbKeys[i].code == code) return k_rbKeys[i].name;
    return "?";
}

/* `select_next_ship:W>M, ...` into the table; a bad row is logged and skipped. */
static void rb_parse(const char *s)
{
    g_rbN = 0;
    while (*s) {
        const char *a, *f, *t, *e;
        int na, nf, nt, from, to;
        char m[160];
        while (*s == ' ' || *s == ',' || *s == '\t') s++;
        if (!*s || *s == ';') break;
        a = s;
        while (*s && *s != ',' && *s != ';') s++;
        e = s;
        while (e > a && (e[-1] == ' ' || e[-1] == '\t')) e--;
        f = a; while (f < e && *f != ':') f++;
        t = f; while (t < e && *t != '>') t++;
        na = (int)(f - a);
        nf = (int)(t - f) - 1;
        nt = (int)(e - t) - 1;
        from = (f < e && t < e && nf > 0) ? rb_key(f + 1, nf) : -1;
        to = (t < e && nt > 0) ? rb_key(t + 1, nt) : -1;
        if (na <= 0 || na >= 40 || from < 0 || to < 0 || g_rbN >= RB_MAX_ROWS) {
            m[0] = 0;
            s_cat(m, "rebind: a row not understood (action:FROM>TO, keys A-Z 0-9 F1-F12), skipped");
            logline(m);
            continue;
        }
        {
            RbRow *r = &g_rbRows[g_rbN++];
            int i;
            for (i = 0; i < na; i++) r->action[i] = a[i];
            r->action[na] = 0;
            r->from = (DWORD)from;
            r->to = (DWORD)to;
        }
    }
}

/* The parsed key map, read where the profile says. */
static BYTE *rb_map(int i)
{
    BYTE *base = *(BYTE **)a2e_addr(QOL_S_RB_MAPS);
    return base ? base + i * QOL_V_RB_MAP_STRIDE : NULLPTR;
}
static int   rb_nmaps(void) { return *(int *)a2e_addr(QOL_S_RB_N_MAPS); }
static BYTE *rb_keyp(BYTE *m, int k) { return m + QOL_F_RB_MAP_KEYS + k * QOL_V_RB_KEY_STRIDE; }
static int   rb_nkeys(BYTE *m)
{
    int n = *(int *)(m + QOL_F_RB_MAP_N);
    return n < 0 ? 0 : (n > QOL_V_RB_MAX_KEYS ? QOL_V_RB_MAX_KEYS : n);
}

/* The state cell of the control named `name`, or NULL. */
static void *rb_cell(const char *name)
{
    BYTE *base = *(BYTE **)a2e_addr(QOL_S_RB_FIELDS);
    int   n = *(int *)a2e_addr(QOL_S_RB_N_FIELDS), i;
    if (!base) return NULLPTR;
    for (i = 0; i < n; i++) {
        BYTE *d = base + i * QOL_V_RB_FIELD_STRIDE;
        if (rb_same(name, (const char *)d, s_len(name))) return *(void **)(d + QOL_F_RB_FIELD_CELL);
    }
    return NULLPTR;
}

/* Whether map `j`'s chord would fire with the key (`word`, `bit`) held under map `m`'s chord:
 * it holds that key and wants no key held that `m` wants up. */
static int rb_clash(BYTE *m, int j, DWORD word, DWORD bit)
{
    BYTE *o = rb_map(j);
    int k, l, has = 0;
    for (k = 0; k < rb_nkeys(o); k++) {
        BYTE *ok = rb_keyp(o, k);
        if (!ok[8]) continue;
        if (*(DWORD *)ok == word && *(DWORD *)(ok + 4) == bit) { has = 1; continue; }
        for (l = 0; l < rb_nkeys(m); l++) {
            BYTE *mk = rb_keyp(m, l);
            if (!mk[8] && *(DWORD *)mk == *(DWORD *)ok && (*(DWORD *)(mk + 4) & *(DWORD *)(ok + 4))) return 0;
        }
    }
    return has;
}

static void rb_log_chord(char *m, BYTE *map)
{
    int k;
    for (k = 0; k < rb_nkeys(map); k++) {
        BYTE *kp = rb_keyp(map, k);
        s_cat(m, kp[8] ? " +" : " -");
        s_hex(m, *(DWORD *)kp);
        s_cat(m, ":");
        s_hex(m, *(DWORD *)(kp + 4));
    }
}

static void rb_apply(void)
{
    int r, i, k, n = rb_nmaps();
    g_rbNHits = 0;
    for (r = 0; r < g_rbN; r++) {
        RbRow *row = &g_rbRows[r];
        void  *cell = rb_cell(row->action);
        int    done = 0;
        char   m[300];
        m[0] = 0;
        if (!cell) {
            s_cat(m, "rebind: no control named "); s_cat(m, row->action); s_cat(m, " in the key map, row skipped");
            logline(m);
            continue;
        }
        for (i = 0; i < n; i++) {
            BYTE *map = rb_map(i);
            if (*(void **)(map + QOL_F_RB_MAP_CELL) != cell) continue;
            for (k = 0; k < rb_nkeys(map); k++) {
                BYTE *kp = rb_keyp(map, k);
                DWORD word = *(DWORD *)kp, bit = *(DWORD *)(kp + 4);
                DWORD f = row->from - 1, t = row->to - 1;
                DWORD base = word - (f / 32) * 4;
                DWORD tWord = base + (t / 32) * 4, tBit = 1u << (t % 32);
                int   j, clash = -1;
                if (!kp[8] || bit != (1u << (f % 32)) || word < (f / 32) * 4) continue;
                for (j = 0; j < n && clash < 0; j++)
                    if (j != i && rb_clash(map, j, tWord, tBit)) clash = j;
                m[0] = 0;
                if (clash >= 0) {
                    BYTE *o = rb_map(clash);
                    s_cat(m, "rebind: "); s_cat(m, row->action); s_cat(m, " ");
                    s_cat(m, rb_name(row->from)); s_cat(m, " -> "); s_cat(m, rb_name(row->to));
                    s_cat(m, " refused: another chord already fires on it (map ");
                    s_num(m, clash); s_cat(m, ","); rb_log_chord(m, o); s_cat(m, ")");
                    logline(m);
                    done = -1;
                    continue;
                }
                if (g_rbNHits >= RB_MAX_HITS) continue;
                *(volatile DWORD *)kp = tWord;
                *(volatile DWORD *)(kp + 4) = tBit;
                g_rbHits[g_rbNHits].map = i;
                g_rbHits[g_rbNHits].key = k;
                g_rbHits[g_rbNHits].word = word;
                g_rbHits[g_rbNHits].bit = bit;
                g_rbHits[g_rbNHits].toWord = tWord;
                g_rbHits[g_rbNHits].toBit = tBit;
                g_rbNHits++;
                s_cat(m, "rebind: "); s_cat(m, row->action); s_cat(m, " ");
                s_cat(m, rb_name(row->from)); s_cat(m, " -> "); s_cat(m, rb_name(row->to));
                s_cat(m, " (map "); s_num(m, i); s_cat(m, ","); rb_log_chord(m, map); s_cat(m, ")");
                logline(m);
                if (!done) done = 1;
            }
            if (!done) {
                m[0] = 0;
                s_cat(m, "rebind: "); s_cat(m, row->action); s_cat(m, " has no held ");
                s_cat(m, rb_name(row->from)); s_cat(m, " in map "); s_num(m, i); s_cat(m, " (");
                rb_log_chord(m, map); s_cat(m, " )");
                logline(m);
            }
        }
    }
}

static void rb_revert(void)
{
    int h, n = rb_nmaps();
    for (h = 0; h < g_rbNHits; h++) {
        RbHit *x = &g_rbHits[h];
        if (x->map < n) {
            BYTE *kp = rb_keyp(rb_map(x->map), x->key);
            if (*(DWORD *)kp == x->toWord && *(DWORD *)(kp + 4) == x->toBit) {
                *(volatile DWORD *)kp = x->word;
                *(volatile DWORD *)(kp + 4) = x->bit;
            }
        }
    }
    if (g_rbNHits) logline("rebind: the key map's own keys back");
    g_rbNHits = 0;
}

/* Reads [BarKeys] Rebind again (QOL_SettingsChanged); a changed table is put back and
 * re-applied at the next step. */
static void rebind_settings(void)
{
    char v[256];
    a2_ini_str(&g_ini, "BarKeys", "Rebind", "", v, sizeof v);
    if (!s_eq(v, g_rbText)) {
        if (g_rbOn == 1) rb_revert();
        s_cpy(g_rbText, v);
        rb_parse(v);
        g_rbOn = -1;
    }
}

/* The bar's step: the table in while the position keys are on, out when they stop.  The key
 * map is a game's for its whole run (InputInit), so it is applied once per change. */
static void rebind_follow(int on)
{
    if (!g_rdyRebind || !g_rbN) return;
    on = on ? 1 : 0;
    if (on == g_rbOn) return;
    if (on) rb_apply(); else rb_revert();
    g_rbOn = on;
}
