/* sym use: TOYC_S_COMMENTED is ignored here */
void f(char *p) {
    int z = TOYC_S_NOPE; int a = TOYC_S_PAN_FN, h = TOYC_H_PAN, k = TOYC_V_PAN_K;
    char *q = p + TOYC_F_PAN_OFF;
    char *r = p + 0x10;
    __asm__("mov _a2s_toyc_pan_patch, %eax");
}
