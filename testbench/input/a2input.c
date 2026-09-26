/* a2input -- a virtual pointer and keyboard on a wlroots compositor's seat.
 *
 * WHY NOT xdotool
 *
 * The bench's headless sway has no input devices, so its seat has no pointer and no
 * keyboard.  XTest moves Xwayland's core pointer anyway, and Wine even logs the
 * ButtonPress -- but with no keyboard on the seat nothing ever holds focus, and the
 * click never becomes a Win32 click.  Virtual devices are real wl_pointer/wl_keyboard
 * input: Xwayland receives them exactly as it would a mouse and keyboard, which is
 * what the game sees on the desktop.  (Measured: XTest left the frame pixel-identical;
 * this opens the Single Player screen.)
 *
 * Both devices live as long as the process.  Destroying one drops the seat's
 * capability, which Xwayland sees as the device being unplugged.
 *
 *   a2input WIDTH HEIGHT          reads one command per line on stdin, answers "ok" or
 *                                 "err <why>" on stdout for each:
 *     move X Y                    absolute, in output pixels
 *     down B | up B               B = 1 left, 2 middle, 3 right
 *     wheel N                     N notches, positive = down
 *     key COMBO                   tap, e.g. Escape, Return, F10, a, ctrl+s, alt+F4
 *     keydown COMBO | keyup COMBO
 *     type TEXT                   the rest of the line, one character at a time
 *     sync                        round-trip only
 *
 * Key names are xkbcommon keysym names; the keymap is xkbcommon's default (us).
 */
#define _GNU_SOURCE
#include <linux/input-event-codes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <wayland-client.h>
#include <xkbcommon/xkbcommon.h>
#include "wlr-virtual-pointer-unstable-v1-client-protocol.h"
#include "virtual-keyboard-unstable-v1-client-protocol.h"

static struct wl_seat *seat;
static struct wl_output *output;
static struct zwlr_virtual_pointer_manager_v1 *pmanager;
static uint32_t pmanager_version;
static struct zwp_virtual_keyboard_manager_v1 *kmanager;

static void global(void *data, struct wl_registry *reg, uint32_t name,
                   const char *iface, uint32_t version) {
    (void)data;
    if (!strcmp(iface, wl_seat_interface.name) && !seat)
        seat = wl_registry_bind(reg, name, &wl_seat_interface, 1);
    else if (!strcmp(iface, wl_output_interface.name) && !output)
        output = wl_registry_bind(reg, name, &wl_output_interface, 1);
    else if (!strcmp(iface, zwlr_virtual_pointer_manager_v1_interface.name)) {
        pmanager_version = version < 2 ? version : 2;
        pmanager = wl_registry_bind(reg, name, &zwlr_virtual_pointer_manager_v1_interface,
                                    pmanager_version);
    } else if (!strcmp(iface, zwp_virtual_keyboard_manager_v1_interface.name))
        kmanager = wl_registry_bind(reg, name, &zwp_virtual_keyboard_manager_v1_interface, 1);
}
static void global_remove(void *d, struct wl_registry *r, uint32_t n) { (void)d; (void)r; (void)n; }
static const struct wl_registry_listener reg_listener = { global, global_remove };

static uint32_t now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

/* ---- keyboard ------------------------------------------------------------------ */

static struct xkb_keymap *keymap;
static struct zwp_virtual_keyboard_v1 *kbd;
static uint32_t mask_shift, mask_ctrl, mask_alt, mask_logo, mods;

/* The evdev code for a keysym, and whether it sits on the shifted level. */
static int find_key(xkb_keysym_t sym, uint32_t *code, int *shifted) {
    xkb_keycode_t min = xkb_keymap_min_keycode(keymap), max = xkb_keymap_max_keycode(keymap);
    for (int level = 0; level < 2; level++)
        for (xkb_keycode_t kc = min; kc <= max; kc++) {
            const xkb_keysym_t *syms;
            int n = xkb_keymap_key_get_syms_by_level(keymap, kc, 0, level, &syms);
            for (int i = 0; i < n; i++)
                if (syms[i] == sym) { *code = kc - 8; *shifted = level; return 1; }
        }
    return 0;
}

static void send_mods(void) { zwp_virtual_keyboard_v1_modifiers(kbd, mods, 0, 0, 0); }

static void key_event(uint32_t code, int down) {
    zwp_virtual_keyboard_v1_key(kbd, now_ms(), code,
                                down ? WL_KEYBOARD_KEY_STATE_PRESSED : WL_KEYBOARD_KEY_STATE_RELEASED);
}

/* Press (down=1) or release (down=0) a combo such as "ctrl+shift+s".  Modifiers go
 * down first and come up last; each is sent both as a key and in the modifier mask,
 * because wlroots forwards a virtual keyboard's mask as given and does not derive it. */
static const char *combo(const char *spec, int down) {
    char buf[128];
    snprintf(buf, sizeof buf, "%s", spec);
    char *parts[8];
    int n = 0;
    for (char *tok = strtok(buf, "+"); tok && n < 8; tok = strtok(NULL, "+")) parts[n++] = tok;
    if (!n) return "empty key";
    uint32_t codes[8], masks[8];
    for (int i = 0; i < n; i++) {
        const char *p = parts[i];
        const char *alias = NULL;
        masks[i] = 0;
        if (i < n - 1 || n == 1) {
            if (!strcasecmp(p, "ctrl") || !strcasecmp(p, "control")) { alias = "Control_L"; masks[i] = mask_ctrl; }
            else if (!strcasecmp(p, "shift")) { alias = "Shift_L"; masks[i] = mask_shift; }
            else if (!strcasecmp(p, "alt")) { alias = "Alt_L"; masks[i] = mask_alt; }
            else if (!strcasecmp(p, "super") || !strcasecmp(p, "logo")) { alias = "Super_L"; masks[i] = mask_logo; }
        }
        if (!strcasecmp(p, "enter")) alias = "Return";
        if (!strcasecmp(p, "esc")) alias = "Escape";
        if (!strcasecmp(p, "space")) alias = "space";
        xkb_keysym_t sym = xkb_keysym_from_name(alias ? alias : p, XKB_KEYSYM_NO_FLAGS);
        if (sym == XKB_KEY_NoSymbol) sym = xkb_keysym_from_name(alias ? alias : p, XKB_KEYSYM_CASE_INSENSITIVE);
        int shifted;
        if (sym == XKB_KEY_NoSymbol || !find_key(sym, &codes[i], &shifted)) return "unknown key";
    }
    if (down)
        for (int i = 0; i < n; i++) {
            if (masks[i]) { mods |= masks[i]; send_mods(); }
            key_event(codes[i], 1);
        }
    else
        for (int i = n - 1; i >= 0; i--) {
            key_event(codes[i], 0);
            if (masks[i]) { mods &= ~masks[i]; send_mods(); }
        }
    return NULL;
}

static const char *type_text(struct wl_display *dpy, const char *s) {
    while (*s && *s != '\n') {
        /* decode one UTF-8 code point */
        uint32_t cp = (unsigned char)*s;
        int len = cp < 0x80 ? 1 : cp < 0xE0 ? 2 : cp < 0xF0 ? 3 : 4;
        if (len > 1) {
            cp &= 0xFF >> (len + 1);
            for (int i = 1; i < len; i++) cp = (cp << 6) | ((unsigned char)s[i] & 0x3F);
        }
        s += len;
        xkb_keysym_t sym = xkb_utf32_to_keysym(cp);
        uint32_t code;
        int shifted;
        if (!find_key(sym, &code, &shifted)) return "a character has no key in the us keymap";
        if (shifted) { mods |= mask_shift; send_mods(); }
        key_event(code, 1);
        key_event(code, 0);
        if (shifted) { mods &= ~mask_shift; send_mods(); }
        wl_display_roundtrip(dpy);
        usleep(20000);
    }
    return NULL;
}

static int setup_keyboard(void) {
    struct xkb_context *ctx = xkb_context_new(XKB_CONTEXT_NO_FLAGS);
    struct xkb_rule_names names = { 0 };
    keymap = xkb_keymap_new_from_names(ctx, &names, XKB_KEYMAP_COMPILE_NO_FLAGS);
    if (!keymap) return 0;
    char *text = xkb_keymap_get_as_string(keymap, XKB_KEYMAP_FORMAT_TEXT_V1);
    size_t size = strlen(text) + 1;
    int fd = memfd_create("a2input-keymap", MFD_CLOEXEC);
    if (fd < 0 || write(fd, text, size) != (ssize_t)size) return 0;
    free(text);
    kbd = zwp_virtual_keyboard_manager_v1_create_virtual_keyboard(kmanager, seat);
    zwp_virtual_keyboard_v1_keymap(kbd, WL_KEYBOARD_KEYMAP_FORMAT_XKB_V1, fd, size);
    close(fd);
    mask_shift = 1u << xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_SHIFT);
    mask_ctrl  = 1u << xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_CTRL);
    mask_alt   = 1u << xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_ALT);
    mask_logo  = 1u << xkb_keymap_mod_get_index(keymap, XKB_MOD_NAME_LOGO);
    return 1;
}

/* ---- main loop ----------------------------------------------------------------- */

int main(int argc, char **argv) {
    if (argc != 3) { fprintf(stderr, "usage: a2input WIDTH HEIGHT\n"); return 2; }
    setvbuf(stdout, NULL, _IOLBF, 0);
    uint32_t w = (uint32_t)atoi(argv[1]), h = (uint32_t)atoi(argv[2]);
    struct wl_display *dpy = wl_display_connect(NULL);
    if (!dpy) { fprintf(stderr, "a2input: cannot connect to WAYLAND_DISPLAY\n"); return 1; }
    struct wl_registry *reg = wl_display_get_registry(dpy);
    wl_registry_add_listener(reg, &reg_listener, NULL);
    wl_display_roundtrip(dpy);
    if (!pmanager || !kmanager || !seat) {
        fprintf(stderr, "a2input: compositor lacks virtual pointer/keyboard support\n");
        return 1;
    }
    struct zwlr_virtual_pointer_v1 *ptr = pmanager_version >= 2 && output
        ? zwlr_virtual_pointer_manager_v1_create_virtual_pointer_with_output(pmanager, seat, output)
        : zwlr_virtual_pointer_manager_v1_create_virtual_pointer(pmanager, seat);
    if (!setup_keyboard()) { fprintf(stderr, "a2input: could not build a keymap\n"); return 1; }
    wl_display_roundtrip(dpy);
    printf("ready\n");

    static const uint32_t buttons[] = { 0, BTN_LEFT, BTN_MIDDLE, BTN_RIGHT };
    char line[1024];
    while (fgets(line, sizeof line, stdin)) {
        int a, b;
        char arg[256];
        const char *err = NULL;
        if (sscanf(line, "move %d %d", &a, &b) == 2) {
            if (a < 0) a = 0;
            if (b < 0) b = 0;
            zwlr_virtual_pointer_v1_motion_absolute(ptr, now_ms(), (uint32_t)a, (uint32_t)b, w, h);
            zwlr_virtual_pointer_v1_frame(ptr);
        } else if (sscanf(line, "down %d", &a) == 1 && a >= 1 && a <= 3) {
            zwlr_virtual_pointer_v1_button(ptr, now_ms(), buttons[a], WL_POINTER_BUTTON_STATE_PRESSED);
            zwlr_virtual_pointer_v1_frame(ptr);
        } else if (sscanf(line, "up %d", &a) == 1 && a >= 1 && a <= 3) {
            zwlr_virtual_pointer_v1_button(ptr, now_ms(), buttons[a], WL_POINTER_BUTTON_STATE_RELEASED);
            zwlr_virtual_pointer_v1_frame(ptr);
        } else if (sscanf(line, "wheel %d", &a) == 1) {
            zwlr_virtual_pointer_v1_axis_source(ptr, WL_POINTER_AXIS_SOURCE_WHEEL);
            zwlr_virtual_pointer_v1_axis_discrete(ptr, now_ms(), WL_POINTER_AXIS_VERTICAL_SCROLL,
                                                  wl_fixed_from_int(15 * a), a);
            zwlr_virtual_pointer_v1_frame(ptr);
        } else if (sscanf(line, "key %255s", arg) == 1) {
            if (!(err = combo(arg, 1))) {
                wl_display_roundtrip(dpy);
                usleep(40000);
                combo(arg, 0);
            }
        } else if (sscanf(line, "keydown %255s", arg) == 1) {
            err = combo(arg, 1);
        } else if (sscanf(line, "keyup %255s", arg) == 1) {
            err = combo(arg, 0);
        } else if (!strncmp(line, "type ", 5)) {
            err = type_text(dpy, line + 5);
        } else if (strncmp(line, "sync", 4)) {
            err = "bad command";
        }
        if (wl_display_roundtrip(dpy) < 0) { printf("err compositor gone\n"); return 1; }
        if (err) printf("err %s\n", err);
        else printf("ok\n");
    }
    zwlr_virtual_pointer_v1_destroy(ptr);
    zwp_virtual_keyboard_v1_destroy(kbd);
    wl_display_roundtrip(dpy);
    return 0;
}
