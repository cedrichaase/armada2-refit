/* STA64 M1 presentation spike: the shared mapping's header, the same layout for the
 * 32-bit ASI and the 64-bit host (fixed-width fields only, 64-bit fields first).
 * Throwaway: nothing here ships. */
#ifndef M1_H
#define M1_H

typedef unsigned int u32;
typedef unsigned long long u64;

#define M1_MAGIC 0x31364d41u /* "AM61" */
#define M1_VERSION 1u

/* message counters, by kind */
enum { MK_MOVE, MK_LDOWN, MK_LUP, MK_RDOWN, MK_KEYDOWN, MK_CHAR, MK_ACTIVATE, MK_OTHER, MK_N };

typedef struct M1Header {
    u64 qpf;                    /* QueryPerformanceFrequency, written by the game */
    u64 heartbeat_qpc;          /* QPC at the last engine BeginScene */
    u64 spawn_qpc;              /* QPC just before CreateProcess of the host */
    u64 spawned_qpc;            /* QPC just after CreateProcess returned */
    u64 host_open_qpc;          /* QPC when the host had the mapping open */
    u64 host_window_qpc;        /* QPC when the host's window existed */
    u64 host_first_present_qpc; /* QPC after the host's first vkQueuePresentKHR */
    u64 host_last_present_qpc;
    u64 stall_begin_qpc, stall_end_qpc;
    u32 magic, version, size;
    u32 game_pid, game_tid, game_hwnd; /* HWNDs fit in 32 bits, also for a 64-bit reader */
    u32 counter;                /* engine BeginScenes */
    u32 stall_ms;               /* request: block the game thread this long at the next BeginScene */
    u32 stalled;                /* 1 while the game thread sleeps */
    u32 stall_count;
    u32 host_pid, host_hwnd, host_frames;
    u32 host_mode;              /* 'A', 'T', 'B', 'D' */
    u32 host_hide;              /* request: the host hides its window */
    u32 game_msgs[MK_N];        /* messages the game's thread took off its queue (WH_GETMESSAGE) */
    u32 host_msgs[MK_N];        /* messages the host's window got */
    u32 game_focus_hwnd;        /* GetForegroundWindow seen from the game, each BeginScene */
    u32 game_active;            /* GetActiveWindow on the game thread */
    u32 reserved[16];
} M1Header;

#endif
