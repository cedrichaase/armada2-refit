/* reads every key through its row */
#include "../../../../../csdk/a2asi.h"
#include "settings_table.h"
#include "rules_settings_table.h"

static void logline(const char *s) { (void)s; }

UINT good_read(void)
{
    A2Ini ini;
    char  theme[16];
    a2_ini_open(&ini, "Good.ini", logline);
    a2_set_str(&ini, GOOD_ROW_GOOD_THEME, theme, sizeof theme);
    return a2_set_int(&ini, GOOD_ROW_GOOD_ENABLE) + a2_set_int(&ini, GOOD_ROW_GOOD_MODE)
         + a2_set_int(&ini, GOOD_ROW_WORLD_SURFACES) + a2_set_int(&ini, GOOD_ROW_HUD_BARHIDDEN)
         + a2_ini_int(&ini, "Other", "Free", 0) + (UINT)theme[0];
}
