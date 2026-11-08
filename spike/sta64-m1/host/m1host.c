/* STA64 M1 presentation spike, the 64-bit side: M1Host.exe (x86_64-pc-windows).
 *
 *   M1Host.exe <mapping name> <mode> [cover%] [nofwd]
 *
 * mode:
 *   A  option A: a top-level popup of its own over the game's window, topmost and
 *      WS_EX_NOACTIVATE (the game keeps the focus and so the keyboard); the mouse it
 *      gets is forwarded to the game's window with PostMessage
 *   a  option A, activating: an ordinary top-level popup that takes the focus; mouse and
 *      keys forwarded with PostMessage
 *   T  option A, click-through: WS_EX_LAYERED | WS_EX_TRANSPARENT | NOACTIVATE, topmost,
 *      nothing forwarded (input should reach the game's window under it)
 *   B  option B: a child window of the game's window (another process); mouse and keys
 *      forwarded to the parent with PostMessage
 *   D  option B, the child disabled (WS_DISABLED): input should go to the parent
 * cover%: the share of the game's client height the host covers, from the top (100)
 *
 * It presents a Vulkan swap chain (FIFO) at the display's rate, drawing with clears
 * only: a bar that moves every host frame, the game's frame counter in bits (green),
 * the host's own in bits (white), and a background that turns red while the game's
 * heartbeat is older than 100 ms. Every present is logged with its time.
 * Throwaway: nothing here ships. */
#include "../m1.h"

typedef unsigned long long size_t_;
typedef void *HANDLE;
typedef void *HWND;
typedef void *HINSTANCE;
typedef int BOOL;
typedef unsigned long DWORD;
typedef long LONG;
typedef unsigned int UINT;
typedef unsigned long long WPARAM;
typedef long long LPARAM;
typedef long long LRESULT;
typedef struct { LONG left, top, right, bottom; } RECT;
typedef struct { LONG x, y; } POINT;
typedef struct { HWND hwnd; UINT message; WPARAM wParam; LPARAM lParam; DWORD time; POINT pt; DWORD priv; } MSG;
typedef struct { long long QuadPart; } LARGE_INTEGER;
typedef LRESULT (*WNDPROC)(HWND, UINT, WPARAM, LPARAM);
typedef struct {
    UINT cbSize, style; WNDPROC lpfnWndProc; int cbClsExtra, cbWndExtra; HINSTANCE hInstance;
    HANDLE hIcon, hCursor, hbrBackground; const char *lpszMenuName, *lpszClassName; HANDLE hIconSm;
} WNDCLASSEXA;

#define IMPORT __declspec(dllimport)
IMPORT HANDLE OpenFileMappingA(DWORD, BOOL, const char *);
IMPORT void *MapViewOfFile(HANDLE, DWORD, DWORD, DWORD, size_t_);
IMPORT HANDLE OpenProcess(DWORD, BOOL, DWORD);
IMPORT DWORD WaitForSingleObject(HANDLE, DWORD);
IMPORT void Sleep(DWORD);
IMPORT HINSTANCE GetModuleHandleA(const char *);
IMPORT HINSTANCE LoadLibraryA(const char *);
IMPORT void *GetProcAddress(HINSTANCE, const char *);
IMPORT BOOL QueryPerformanceCounter(LARGE_INTEGER *);
IMPORT BOOL QueryPerformanceFrequency(LARGE_INTEGER *);
IMPORT DWORD GetCurrentProcessId(void);
IMPORT HANDLE CreateFileA(const char *, DWORD, DWORD, void *, DWORD, DWORD, HANDLE);
IMPORT BOOL WriteFile(HANDLE, const void *, DWORD, DWORD *, void *);
IMPORT BOOL FlushFileBuffers(HANDLE);
IMPORT char *GetCommandLineA(void);
IMPORT void ExitProcess(UINT);
IMPORT unsigned short RegisterClassExA(const WNDCLASSEXA *);
IMPORT HWND CreateWindowExA(DWORD, const char *, const char *, DWORD, int, int, int, int, HWND, HANDLE, HINSTANCE, void *);
IMPORT LRESULT DefWindowProcA(HWND, UINT, WPARAM, LPARAM);
IMPORT BOOL PeekMessageA(MSG *, HWND, UINT, UINT, UINT);
IMPORT BOOL TranslateMessage(const MSG *);
IMPORT LRESULT DispatchMessageA(const MSG *);
IMPORT BOOL ShowWindow(HWND, int);
IMPORT BOOL SetWindowPos(HWND, HWND, int, int, int, int, UINT);
IMPORT BOOL GetClientRect(HWND, RECT *);
IMPORT BOOL ClientToScreen(HWND, POINT *);
IMPORT BOOL PostMessageA(HWND, UINT, WPARAM, LPARAM);
IMPORT BOOL SetLayeredWindowAttributes(HWND, DWORD, unsigned char, DWORD);
IMPORT BOOL IsWindow(HWND);
IMPORT HWND SetParent(HWND, HWND);
IMPORT HWND GetForegroundWindow(void);
IMPORT int wsprintfA(char *, const char *, ...);
IMPORT HANDLE LoadCursorA(HINSTANCE, const char *);

void *memset(void *d, int c, size_t_ n) { unsigned char *p = d; while (n--) *p++ = (unsigned char)c; return d; }
void *memcpy(void *d, const void *s, size_t_ n) { unsigned char *p = d; const unsigned char *q = s; while (n--) *p++ = *q++; return d; }
int _fltused;

/* ---- the Vulkan subset, declared by hand (no SDK headers here) ---- */
typedef void *VkH;
typedef int VkResult;
typedef u32 VkBool32;
typedef struct { u32 width, height; } VkExtent2D;
typedef struct { int x, y; } VkOffset2D;
typedef struct { VkOffset2D offset; VkExtent2D extent; } VkRect2D;
typedef struct { u32 sType; const void *pNext; const char *app; u32 appv; const char *eng; u32 engv; u32 api; } VkApplicationInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; const VkApplicationInfo *app; u32 nl; const char *const *layers; u32 ne; const char *const *exts; } VkInstanceCreateInfo;
typedef struct { u32 flags, count, tsbits, gx, gy, gz; } VkQueueFamilyProperties;
typedef struct { u32 sType; const void *pNext; u32 flags; u32 family; u32 count; const float *prio; } VkDeviceQueueCreateInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; u32 nq; const VkDeviceQueueCreateInfo *q; u32 nl; const char *const *layers; u32 ne; const char *const *exts; const void *features; } VkDeviceCreateInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; HINSTANCE hinst; HWND hwnd; } VkWin32SurfaceCreateInfoKHR;
typedef struct { u32 minImageCount, maxImageCount; VkExtent2D cur, minE, maxE; u32 maxLayers, transforms, curTransform, compositeAlpha, usage; } VkSurfaceCapabilitiesKHR;
typedef struct { u32 format, colorSpace; } VkSurfaceFormatKHR;
typedef struct { u32 sType; const void *pNext; u32 flags; VkH surface; u32 minImageCount, format, colorSpace; VkExtent2D extent; u32 layers, usage, sharing, nqf; const u32 *qf; u32 transform, alpha, presentMode; VkBool32 clipped; VkH old; } VkSwapchainCreateInfoKHR;
typedef struct { u32 sType; const void *pNext; u32 flags; VkH image; u32 viewType, format; u32 r, g, b, a; u32 aspect, baseMip, levels, baseLayer, nlayers; } VkImageViewCreateInfo;
typedef struct { u32 flags, format, samples, loadOp, storeOp, sLoadOp, sStoreOp, initialLayout, finalLayout; } VkAttachmentDescription;
typedef struct { u32 attachment, layout; } VkAttachmentReference;
typedef struct { u32 flags, bindPoint; u32 nin; const VkAttachmentReference *in; u32 ncol; const VkAttachmentReference *col; const VkAttachmentReference *resolve; const VkAttachmentReference *ds; u32 npres; const u32 *pres; } VkSubpassDescription;
typedef struct { u32 src, dst, srcStage, dstStage, srcAccess, dstAccess, flags; } VkSubpassDependency;
typedef struct { u32 sType; const void *pNext; u32 flags; u32 na; const VkAttachmentDescription *a; u32 ns; const VkSubpassDescription *s; u32 nd; const VkSubpassDependency *d; } VkRenderPassCreateInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; VkH rp; u32 na; const VkH *views; u32 w, h, layers; } VkFramebufferCreateInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; u32 family; } VkCommandPoolCreateInfo;
typedef struct { u32 sType; const void *pNext; VkH pool; u32 level; u32 count; } VkCommandBufferAllocateInfo;
typedef struct { u32 sType; const void *pNext; u32 flags; const void *inh; } VkCommandBufferBeginInfo;
typedef struct { float c[4]; } VkClearValue;
typedef struct { u32 sType; const void *pNext; VkH rp; VkH fb; VkRect2D area; u32 nclear; const VkClearValue *clear; } VkRenderPassBeginInfo;
typedef struct { u32 aspect; u32 att; VkClearValue v; } VkClearAttachment;
typedef struct { VkRect2D rect; u32 baseLayer, nlayers; } VkClearRect;
typedef struct { u32 sType; const void *pNext; u32 nw; const VkH *wait; const u32 *stages; u32 ncb; const VkH *cbs; u32 ns; const VkH *sig; } VkSubmitInfo;
typedef struct { u32 sType; const void *pNext; u32 nw; const VkH *wait; u32 nsc; const VkH *sc; const u32 *idx; VkResult *res; } VkPresentInfoKHR;
typedef struct { u32 sType; const void *pNext; u32 flags; } VkSimpleCreateInfo;

typedef void (*PFN)(void);
typedef PFN (*PFN_gipa)(VkH, const char *);
static PFN_gipa gipa;
#define VKF(ret, name, args) static ret (*name) args
VKF(VkResult, vkCreateInstance, (const VkInstanceCreateInfo *, const void *, VkH *));
VKF(VkResult, vkEnumeratePhysicalDevices, (VkH, u32 *, VkH *));
VKF(void, vkGetPhysicalDeviceProperties, (VkH, void *));
VKF(void, vkGetPhysicalDeviceQueueFamilyProperties, (VkH, u32 *, VkQueueFamilyProperties *));
VKF(VkResult, vkCreateWin32SurfaceKHR, (VkH, const VkWin32SurfaceCreateInfoKHR *, const void *, VkH *));
VKF(VkResult, vkGetPhysicalDeviceSurfaceSupportKHR, (VkH, u32, VkH, VkBool32 *));
VKF(VkResult, vkGetPhysicalDeviceSurfaceCapabilitiesKHR, (VkH, VkH, VkSurfaceCapabilitiesKHR *));
VKF(VkResult, vkGetPhysicalDeviceSurfaceFormatsKHR, (VkH, VkH, u32 *, VkSurfaceFormatKHR *));
VKF(VkResult, vkCreateDevice, (VkH, const VkDeviceCreateInfo *, const void *, VkH *));
VKF(void, vkGetDeviceQueue, (VkH, u32, u32, VkH *));
VKF(VkResult, vkCreateSwapchainKHR, (VkH, const VkSwapchainCreateInfoKHR *, const void *, VkH *));
VKF(void, vkDestroySwapchainKHR, (VkH, VkH, const void *));
VKF(VkResult, vkGetSwapchainImagesKHR, (VkH, VkH, u32 *, VkH *));
VKF(VkResult, vkCreateImageView, (VkH, const VkImageViewCreateInfo *, const void *, VkH *));
VKF(void, vkDestroyImageView, (VkH, VkH, const void *));
VKF(VkResult, vkCreateRenderPass, (VkH, const VkRenderPassCreateInfo *, const void *, VkH *));
VKF(VkResult, vkCreateFramebuffer, (VkH, const VkFramebufferCreateInfo *, const void *, VkH *));
VKF(void, vkDestroyFramebuffer, (VkH, VkH, const void *));
VKF(VkResult, vkCreateCommandPool, (VkH, const VkCommandPoolCreateInfo *, const void *, VkH *));
VKF(VkResult, vkAllocateCommandBuffers, (VkH, const VkCommandBufferAllocateInfo *, VkH *));
VKF(VkResult, vkBeginCommandBuffer, (VkH, const VkCommandBufferBeginInfo *));
VKF(VkResult, vkEndCommandBuffer, (VkH));
VKF(void, vkCmdBeginRenderPass, (VkH, const VkRenderPassBeginInfo *, u32));
VKF(void, vkCmdEndRenderPass, (VkH));
VKF(void, vkCmdClearAttachments, (VkH, u32, const VkClearAttachment *, u32, const VkClearRect *));
VKF(VkResult, vkCreateSemaphore, (VkH, const VkSimpleCreateInfo *, const void *, VkH *));
VKF(VkResult, vkCreateFence, (VkH, const VkSimpleCreateInfo *, const void *, VkH *));
VKF(VkResult, vkWaitForFences, (VkH, u32, const VkH *, VkBool32, u64));
VKF(VkResult, vkResetFences, (VkH, u32, const VkH *));
VKF(VkResult, vkAcquireNextImageKHR, (VkH, VkH, u64, VkH, VkH, u32 *));
VKF(VkResult, vkQueueSubmit, (VkH, u32, const VkSubmitInfo *, VkH));
VKF(VkResult, vkQueuePresentKHR, (VkH, const VkPresentInfoKHR *));
VKF(VkResult, vkDeviceWaitIdle, (VkH));

/* ---- state ---- */
static volatile M1Header *H;
static HANDLE g_log;
static HWND g_win, g_game;
static char g_mode = 'A';
static int g_cover = 100, g_forward = 1;
static VkH inst, phys, dev, queue, surface, swapchain, rpass, cpool, cmd, sem_acq, sem_done, fence;
static u32 family, nimages, sc_w, sc_h, sc_format;
static VkH images[8], views[8], fbs[8];

static u64 qpc(void) { LARGE_INTEGER t; QueryPerformanceCounter(&t); return (u64)t.QuadPart; }
static u64 us(u64 dt) { return dt * 1000000ull / H->qpf; }

static void logs(const char *s)
{
    DWORD n = 0, w;
    while (s[n]) n++;
    if (g_log) WriteFile(g_log, s, n, &w, 0);
}
#define LOGF(...) do { char b_[600]; int n_ = wsprintfA(b_, __VA_ARGS__); b_[n_] = 0; logs(b_); } while (0)
static void die(const char *why) { LOGF("fatal: %s\r\n", why); FlushFileBuffers(g_log); ExitProcess(1); }
#define CHECK(x) do { VkResult r_ = (x); if (r_ < 0) { LOGF("%s -> %d\r\n", #x, r_); die("vulkan"); } } while (0)

static LRESULT wndproc(HWND w, UINT m, WPARAM wp, LPARAM lp)
{
    int k = -1;
    if (m == 0x200) k = MK_MOVE;
    else if (m == 0x201) k = MK_LDOWN;
    else if (m == 0x202) k = MK_LUP;
    else if (m == 0x204) k = MK_RDOWN;
    else if (m == 0x100 || m == 0x104) k = MK_KEYDOWN;
    else if (m == 0x102) k = MK_CHAR;
    else if (m == 0x06 || m == 0x1c || m == 0x07 || m == 0x08 || m == 0x21) k = MK_ACTIVATE;
    if (k >= 0 && H) {
        H->host_msgs[k]++;
        if (k != MK_MOVE) LOGF("host msg 0x%x wp %x lp %x at %u us; game stalled %u\r\n", m, (u32)wp, (u32)lp, (u32)us(qpc() - H->spawn_qpc), H->stalled);
        if (H->stalled && k == MK_MOVE && (H->host_msgs[k] & 15) == 0) LOGF("host mousemove during the stall (%u)\r\n", H->host_msgs[k]);
    }
    if (g_forward && g_game && ((m >= 0x200 && m <= 0x20e) || (m >= 0x100 && m <= 0x109))) {
        PostMessageA(g_game, m, wp, lp);
        return 0;
    }
    if (m == 0x21 /* WM_MOUSEACTIVATE */) return 3; /* MA_NOACTIVATE; DefWindowProc would ask the parent (B) */
    if (m == 0x20 /* WM_SETCURSOR */ && (g_mode == 'B' || g_mode == 'D')) return 1; /* DefWindowProc asks the parent first: a cross-thread send */
    if (m == 0x10 || m == 0x14) return m == 0x14; /* no close, no erase */
    return DefWindowProcA(w, m, wp, lp);
}

static void load_vulkan(void)
{
    HINSTANCE lib = LoadLibraryA("vulkan-1.dll");
    if (!lib) die("no vulkan-1.dll");
    gipa = (PFN_gipa)GetProcAddress(lib, "vkGetInstanceProcAddr");
    if (!gipa) die("no vkGetInstanceProcAddr");
    vkCreateInstance = (void *)gipa(0, "vkCreateInstance");
}
#define L(name) do { name = (void *)gipa(inst, #name); if (!name) die(#name); } while (0)

static void make_swapchain(void)
{
    VkSurfaceCapabilitiesKHR caps;
    VkSwapchainCreateInfoKHR ci;
    VkH old = swapchain;
    u32 i;
    CHECK(vkGetPhysicalDeviceSurfaceCapabilitiesKHR(phys, surface, &caps));
    sc_w = caps.cur.width; sc_h = caps.cur.height;
    if (sc_w == 0xffffffff || sc_w == 0) {
        RECT r; GetClientRect(g_win, &r);
        sc_w = (u32)(r.right - r.left); sc_h = (u32)(r.bottom - r.top);
    }
    if (!sc_w || !sc_h) { sc_w = sc_h = 0; return; }
    for (i = 0; i < nimages; i++) { vkDestroyFramebuffer(dev, fbs[i], 0); vkDestroyImageView(dev, views[i], 0); }
    memset(&ci, 0, sizeof ci);
    ci.sType = 1000001000; ci.surface = surface;
    ci.minImageCount = caps.minImageCount < 2 ? 2 : caps.minImageCount;
    if (caps.maxImageCount && ci.minImageCount > caps.maxImageCount) ci.minImageCount = caps.maxImageCount;
    ci.format = sc_format; ci.colorSpace = 0; ci.extent.width = sc_w; ci.extent.height = sc_h;
    ci.layers = 1; ci.usage = 0x10; ci.sharing = 0; ci.transform = caps.curTransform;
    ci.alpha = (caps.compositeAlpha & 1) ? 1 : caps.compositeAlpha & (u32)-(int)caps.compositeAlpha;
    ci.presentMode = 2; /* FIFO */ ci.clipped = 1; ci.old = old;
    CHECK(vkCreateSwapchainKHR(dev, &ci, 0, &swapchain));
    if (old) vkDestroySwapchainKHR(dev, old, 0);
    nimages = 8;
    CHECK(vkGetSwapchainImagesKHR(dev, swapchain, &nimages, images));
    for (i = 0; i < nimages; i++) {
        VkImageViewCreateInfo v; VkFramebufferCreateInfo f;
        memset(&v, 0, sizeof v);
        v.sType = 15; v.image = images[i]; v.viewType = 1; v.format = sc_format; v.aspect = 1; v.levels = 1; v.nlayers = 1;
        CHECK(vkCreateImageView(dev, &v, 0, &views[i]));
        memset(&f, 0, sizeof f);
        f.sType = 37; f.rp = rpass; f.na = 1; f.views = &views[i]; f.w = sc_w; f.h = sc_h; f.layers = 1;
        CHECK(vkCreateFramebuffer(dev, &f, 0, &fbs[i]));
    }
    LOGF("swap chain %ux%u, %u images, alpha mode %u\r\n", sc_w, sc_h, nimages, ci.alpha);
}

static void init_vulkan(void)
{
    static const char *iext[] = { "VK_KHR_surface", "VK_KHR_win32_surface" };
    static const char *dext[] = { "VK_KHR_swapchain" };
    VkApplicationInfo app = { 0, 0, "M1Host", 1, "spike", 1, (1u << 22) | (1u << 12) };
    VkInstanceCreateInfo ici = { 1, 0, 0, &app, 0, 0, 2, iext };
    VkH devs[8]; u32 ndev = 8, i;
    VkQueueFamilyProperties qf[16]; u32 nqf = 16;
    VkWin32SurfaceCreateInfoKHR sci;
    float prio = 1.0f;
    VkDeviceQueueCreateInfo qci;
    VkDeviceCreateInfo dci;
    VkSurfaceFormatKHR fmts[32]; u32 nfmt = 32;
    static unsigned char props[2048];
    VkAttachmentDescription att = { 0, 0, 1, 1 /* clear */, 0 /* store */, 2, 1, 0, 1000001002 };
    VkAttachmentReference ref = { 0, 2 };
    VkSubpassDescription sub;
    VkSubpassDependency dep = { 0xffffffffu, 0, 0x400, 0x400, 0, 0x100, 0 };
    VkRenderPassCreateInfo rpci;
    VkCommandPoolCreateInfo pci;
    VkCommandBufferAllocateInfo cai;
    VkSimpleCreateInfo sem = { 9, 0, 0 }, fen = { 8, 0, 1 };

    CHECK(vkCreateInstance(&ici, 0, &inst));
    L(vkEnumeratePhysicalDevices); L(vkGetPhysicalDeviceProperties); L(vkGetPhysicalDeviceQueueFamilyProperties);
    L(vkCreateWin32SurfaceKHR); L(vkGetPhysicalDeviceSurfaceSupportKHR); L(vkGetPhysicalDeviceSurfaceCapabilitiesKHR);
    L(vkGetPhysicalDeviceSurfaceFormatsKHR); L(vkCreateDevice); L(vkGetDeviceQueue); L(vkCreateSwapchainKHR);
    L(vkDestroySwapchainKHR); L(vkGetSwapchainImagesKHR); L(vkCreateImageView); L(vkDestroyImageView);
    L(vkCreateRenderPass); L(vkCreateFramebuffer); L(vkDestroyFramebuffer); L(vkCreateCommandPool);
    L(vkAllocateCommandBuffers); L(vkBeginCommandBuffer); L(vkEndCommandBuffer); L(vkCmdBeginRenderPass);
    L(vkCmdEndRenderPass); L(vkCmdClearAttachments); L(vkCreateSemaphore); L(vkCreateFence); L(vkWaitForFences);
    L(vkResetFences); L(vkAcquireNextImageKHR); L(vkQueueSubmit); L(vkQueuePresentKHR); L(vkDeviceWaitIdle);

    memset(&sci, 0, sizeof sci);
    sci.sType = 1000009000; sci.hinst = GetModuleHandleA(0); sci.hwnd = g_win;
    CHECK(vkCreateWin32SurfaceKHR(inst, &sci, 0, &surface));
    CHECK(vkEnumeratePhysicalDevices(inst, &ndev, devs));
    phys = 0;
    for (i = 0; i < ndev && !phys; i++) {
        u32 q;
        u32 type;
        vkGetPhysicalDeviceProperties(devs[i], props);
        type = *(u32 *)(props + 16);
        nqf = 16;
        vkGetPhysicalDeviceQueueFamilyProperties(devs[i], &nqf, qf);
        LOGF("device %u: %s (type %u)\r\n", i, (const char *)(props + 20), type);
        if (type == 4 && ndev > 1) continue; /* a CPU device only if alone */
        for (q = 0; q < nqf; q++) {
            VkBool32 ok = 0;
            vkGetPhysicalDeviceSurfaceSupportKHR(devs[i], q, surface, &ok);
            if ((qf[q].flags & 1) && ok) { phys = devs[i]; family = q; break; }
        }
    }
    if (!phys) die("no device presents to this window");
    vkGetPhysicalDeviceProperties(phys, props);
    LOGF("chose %s, family %u\r\n", (const char *)(props + 20), family);
    memset(&qci, 0, sizeof qci);
    qci.sType = 2; qci.family = family; qci.count = 1; qci.prio = &prio;
    memset(&dci, 0, sizeof dci);
    dci.sType = 3; dci.nq = 1; dci.q = &qci; dci.ne = 1; dci.exts = dext;
    CHECK(vkCreateDevice(phys, &dci, 0, &dev));
    vkGetDeviceQueue(dev, family, 0, &queue);
    CHECK(vkGetPhysicalDeviceSurfaceFormatsKHR(phys, surface, &nfmt, fmts));
    sc_format = fmts[0].format;
    for (i = 0; i < nfmt; i++) if (fmts[i].format == 44 /* B8G8R8A8_UNORM */) sc_format = 44;
    att.format = sc_format;
    memset(&sub, 0, sizeof sub);
    sub.ncol = 1; sub.col = &ref;
    memset(&rpci, 0, sizeof rpci);
    rpci.sType = 38; rpci.na = 1; rpci.a = &att; rpci.ns = 1; rpci.s = &sub; rpci.nd = 1; rpci.d = &dep;
    CHECK(vkCreateRenderPass(dev, &rpci, 0, &rpass));
    memset(&pci, 0, sizeof pci);
    pci.sType = 39; pci.flags = 2 /* reset */; pci.family = family;
    CHECK(vkCreateCommandPool(dev, &pci, 0, &cpool));
    memset(&cai, 0, sizeof cai);
    cai.sType = 40; cai.pool = cpool; cai.count = 1;
    CHECK(vkAllocateCommandBuffers(dev, &cai, &cmd));
    CHECK(vkCreateSemaphore(dev, &sem, 0, &sem_acq));
    CHECK(vkCreateSemaphore(dev, &sem, 0, &sem_done));
    CHECK(vkCreateFence(dev, &fen, 0, &fence));
    make_swapchain();
}

static VkClearRect rects[96];
static VkClearAttachment atts[96];
static u32 nr;
static void box(int x, int y, int w, int h, float r, float g, float b)
{
    if (nr >= 96 || w <= 0 || h <= 0) return;
    if (x < 0) { w += x; x = 0; }
    if (y < 0) { h += y; y = 0; }
    if (x + w > (int)sc_w) w = (int)sc_w - x;
    if (y + h > (int)sc_h) h = (int)sc_h - y;
    if (w <= 0 || h <= 0) return;
    rects[nr].rect.offset.x = x; rects[nr].rect.offset.y = y;
    rects[nr].rect.extent.width = (u32)w; rects[nr].rect.extent.height = (u32)h;
    rects[nr].baseLayer = 0; rects[nr].nlayers = 1;
    atts[nr].aspect = 1; atts[nr].att = 0;
    atts[nr].v.c[0] = r; atts[nr].v.c[1] = g; atts[nr].v.c[2] = b; atts[nr].v.c[3] = 1;
    nr++;
}
static void bits(u32 v, int y, int h, float r, float g, float b)
{
    int i, cw = (int)sc_w / 26;
    for (i = 0; i < 24; i++) {
        int on = (v >> (23 - i)) & 1;
        box(cw + i * cw + 2, y, cw - 4, h, on ? r : r * 0.15f, on ? g : g * 0.15f, on ? b : b * 0.15f);
    }
}

/* returns 0 when nothing was presented (no swap chain, or out of date) */
static int frame(u32 n, int stale)
{
    u32 idx;
    VkResult r;
    VkCommandBufferBeginInfo bi = { 42, 0, 1, 0 };
    VkClearValue bg;
    VkRenderPassBeginInfo rbi;
    u32 stage = 0x400, i;
    VkSubmitInfo si;
    VkPresentInfoKHR pi;
    int bw = (int)sc_w / 40 + 4, x;

    if (!sc_w) { make_swapchain(); if (!sc_w) return 0; }
    CHECK(vkWaitForFences(dev, 1, &fence, 1, ~0ull));
    r = vkAcquireNextImageKHR(dev, swapchain, ~0ull, sem_acq, 0, &idx);
    if (r == -1000001004) { make_swapchain(); return 0; }
    CHECK(r);
    CHECK(vkResetFences(dev, 1, &fence));
    CHECK(vkBeginCommandBuffer(cmd, &bi));
    bg.c[0] = stale ? 0.35f : 0.02f; bg.c[1] = 0.03f; bg.c[2] = stale ? 0.05f : 0.20f; bg.c[3] = 1;
    memset(&rbi, 0, sizeof rbi);
    rbi.sType = 43; rbi.rp = rpass; rbi.fb = fbs[idx]; rbi.area.extent.width = sc_w; rbi.area.extent.height = sc_h;
    rbi.nclear = 1; rbi.clear = &bg;
    vkCmdBeginRenderPass(cmd, &rbi, 0);
    nr = 0;
    x = (int)((n * 8u) % (sc_w + (u32)bw)) - bw;
    box(x, 0, bw, (int)sc_h, 0.9f, 0.9f, 0.9f);                                 /* the host's moving bar */
    box(0, (int)sc_h / 8, (int)((H->counter % 256u) * sc_w / 256u), (int)sc_h / 16, 0.1f, 0.9f, 0.2f); /* game counter */
    bits(H->counter, (int)sc_h * 3 / 8, (int)sc_h / 10, 0.2f, 1.0f, 0.3f);
    bits(n, (int)sc_h * 5 / 8, (int)sc_h / 10, 1.0f, 1.0f, 1.0f);
    if (H->stalled) box(0, (int)sc_h - (int)sc_h / 12, (int)sc_w, (int)sc_h / 12, 1.0f, 0.2f, 0.1f);
    /* one call per rect: every attachment of a call clears every rect of it */
    for (i = 0; i < nr; i++) vkCmdClearAttachments(cmd, 1, &atts[i], 1, &rects[i]);
    vkCmdEndRenderPass(cmd);
    CHECK(vkEndCommandBuffer(cmd));
    memset(&si, 0, sizeof si);
    si.sType = 4; si.nw = 1; si.wait = &sem_acq; si.stages = &stage; si.ncb = 1; si.cbs = &cmd; si.ns = 1; si.sig = &sem_done;
    CHECK(vkQueueSubmit(queue, 1, &si, fence));
    memset(&pi, 0, sizeof pi);
    pi.sType = 1000001001; pi.nw = 1; pi.wait = &sem_done; pi.nsc = 1; pi.sc = &swapchain; pi.idx = &idx;
    r = vkQueuePresentKHR(queue, &pi);
    if (r == -1000001004 || r == 1000001003) { make_swapchain(); return r == 1000001003; }
    CHECK(r);
    return 1;
}

static char *next_tok(char **p)
{
    char *s = *p, *t;
    while (*s == ' ') s++;
    if (!*s) return 0;
    if (*s == '"') { t = ++s; while (*s && *s != '"') s++; }
    else { t = s; while (*s && *s != ' ') s++; }
    if (*s) *s++ = 0;
    *p = s;
    return t;
}

static void place(int create)
{
    RECT r; POINT o = { 0, 0 };
    int w, h, hh;
    GetClientRect(g_game, &r);
    w = r.right - r.left; h = r.bottom - r.top;
    hh = h * g_cover / 100;
    if (g_mode == 'B' || g_mode == 'D') {
        if (create) return;
        SetWindowPos(g_win, 0, 0, 0, w, hh, 0x14 /* NOZORDER|NOACTIVATE */);
    } else {
        ClientToScreen(g_game, &o);
        if (create) return;
        SetWindowPos(g_win, (HWND)(long long)-1 /* TOPMOST */, o.x, o.y, w, hh, 0x10 /* NOACTIVATE */);
    }
}

void start(void)
{
    char *cl = GetCommandLineA(), *name, *tok, logname[64];
    HANDLE map, game;
    WNDCLASSEXA wc;
    RECT r; POINT o = { 0, 0 };
    MSG m;
    u32 n = 0, last_counter = 0, i;
    u64 t_prev = 0, t_follow = 0, max_dt = 0, stall_max_dt = 0, stall_frames = 0;
    u32 hidden = 0, last_hwnd;
    DWORD style, ex;

    next_tok(&cl);
    name = next_tok(&cl);
    tok = next_tok(&cl);
    if (tok) g_mode = tok[0];
    while ((tok = next_tok(&cl))) {
        if (tok[0] >= '0' && tok[0] <= '9') { g_cover = 0; while (*tok >= '0' && *tok <= '9') g_cover = g_cover * 10 + (*tok++ - '0'); }
        else if (tok[0] == 'n') g_forward = 0;
    }
    if (g_mode == 'T' || g_mode == 'D') g_forward = 0;
    wsprintfA(logname, "M1Host-%c.log", g_mode == 'a' ? 'X' : g_mode);
    g_log = CreateFileA(logname, 0x40000000, 1, 0, 2, 0, 0);
    if (!name) die("no mapping name");
    map = OpenFileMappingA(0xf001f, 0, name);
    if (!map) die("OpenFileMapping failed");
    H = (volatile M1Header *)MapViewOfFile(map, 0xf001f, 0, 0, 0);
    if (!H || H->magic != M1_MAGIC) die("bad mapping");
    H->host_open_qpc = qpc();
    H->host_pid = GetCurrentProcessId();
    H->host_mode = (u32)g_mode;
    LOGF("M1Host pid %u, mapping %s, mode %c, cover %d%%, forward %d; opened %u us after spawn\r\n",
         H->host_pid, name, g_mode, g_cover, g_forward, (u32)us(H->host_open_qpc - H->spawn_qpc));
    game = OpenProcess(0x100000 /* SYNCHRONIZE */, 0, H->game_pid);
    while (!H->game_hwnd) Sleep(10);
    g_game = (HWND)(unsigned long long)H->game_hwnd;
    last_hwnd = H->game_hwnd;

    memset(&wc, 0, sizeof wc);
    wc.cbSize = sizeof wc; wc.lpfnWndProc = wndproc; wc.hInstance = GetModuleHandleA(0);
    wc.lpszClassName = "M1Host"; wc.hCursor = LoadCursorA(0, (const char *)32512);
    RegisterClassExA(&wc);
    GetClientRect(g_game, &r);
    if (g_mode == 'B' || g_mode == 'D') {
        style = 0x40000000 /* CHILD */ | 0x04000000 /* CLIPSIBLINGS */ | (g_mode == 'D' ? 0x08000000 : 0);
        ex = 0x08000000 /* NOACTIVATE */;
        g_win = CreateWindowExA(ex, "M1Host", "M1Host", style, 0, 0, r.right, r.bottom * g_cover / 100, g_game, 0, wc.hInstance, 0);
    } else {
        ClientToScreen(g_game, &o);
        style = 0x80000000 /* POPUP */;
        ex = g_mode == 'a' ? 0 : 0x08000000 /* NOACTIVATE */ | 0x8 /* TOPMOST */;
        if (g_mode == 'T') ex |= 0x80000 /* LAYERED */ | 0x20 /* TRANSPARENT */;
        g_win = CreateWindowExA(ex, "M1Host", "Armada II (M1 host)", style, o.x, o.y, r.right, r.bottom * g_cover / 100, 0, 0, wc.hInstance, 0);
        if (g_mode == 'T') SetLayeredWindowAttributes(g_win, 0, 255, 2 /* LWA_ALPHA */);
    }
    if (!g_win) die("CreateWindowEx failed");
    ShowWindow(g_win, g_mode == 'a' ? 5 /* SHOW */ : 4 /* SHOWNOACTIVATE */);
    H->host_window_qpc = qpc();
    H->host_hwnd = (u32)(unsigned long long)g_win;
    LOGF("window %x (game %x %dx%d) style %x ex %x, at %u us after spawn\r\n", H->host_hwnd, H->game_hwnd,
         (int)r.right, (int)r.bottom, (u32)style, (u32)ex, (u32)us(H->host_window_qpc - H->spawn_qpc));

    load_vulkan();
    init_vulkan();

    for (;;) {
        u64 now, dt;
        int stale, presented;
        while (PeekMessageA(&m, 0, 0, 0, 1)) { TranslateMessage(&m); DispatchMessageA(&m); }
        if (game && WaitForSingleObject(game, 0) == 0) { LOGF("game exited\r\n"); break; }
        now = qpc();
        if (us(now - t_follow) > 250000) {
            t_follow = now;
            if (H->game_hwnd != last_hwnd && IsWindow((HWND)(unsigned long long)H->game_hwnd)) {
                last_hwnd = H->game_hwnd;
                g_game = (HWND)(unsigned long long)H->game_hwnd;
                LOGF("the game's window is now %x\r\n", last_hwnd);
                if (g_mode == 'B' || g_mode == 'D') SetParent(g_win, g_game);
            }
            if (!hidden) place(0);
            if (H->host_hide != hidden) {
                hidden = H->host_hide;
                ShowWindow(g_win, hidden ? 0 : 4);
                LOGF("%s\r\n", hidden ? "hidden" : "shown");
            }
        }
        stale = H->heartbeat_qpc && us(now - H->heartbeat_qpc) > 100000;
        presented = hidden ? 0 : frame(n, stale);
        if (hidden) Sleep(16);
        if (!presented) continue;
        now = qpc();
        dt = t_prev ? us(now - t_prev) : 0;
        t_prev = now;
        if (!H->host_first_present_qpc) {
            H->host_first_present_qpc = now;
            LOGF("first present at %u us after spawn\r\n", (u32)us(now - H->spawn_qpc));
        }
        H->host_last_present_qpc = now;
        H->host_frames = ++n;
        if (dt > max_dt) max_dt = dt;
        if (H->stalled) { stall_frames++; if (dt > stall_max_dt) stall_max_dt = dt; }
        LOGF("present %u t %u dt %u game %u stalled %u fg %x\r\n", n, (u32)us(now - H->spawn_qpc), (u32)dt,
             H->counter, H->stalled, (u32)(unsigned long long)GetForegroundWindow());
        if (n % 300 == 0) {
            LOGF("summary: %u presents, max dt %u us, during stalls %u presents with max dt %u us; game counter +%u\r\n",
                 n, (u32)max_dt, (u32)stall_frames, (u32)stall_max_dt, H->counter - last_counter);
            last_counter = H->counter;
            FlushFileBuffers(g_log);
        }
    }
    (void)i;
    ExitProcess(0);
}
