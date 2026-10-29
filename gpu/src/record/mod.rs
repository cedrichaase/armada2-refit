//! Record one frame of `gpu` calls to files, for `replay` to draw again with no game
//! (README.md, "Record and replay").
//!
//! **Asked for** with a trigger file beside the exe, `Gpu.record`, whose text is the
//! recording's name on its first line (and anything after, which makes a request
//! unique). Each plugin's recorder looks for it twice a second; the first to see it
//! picks the frame, a little ahead so the other plugins see it too, and writes it into
//! a page every recorder in the process shares (`sys::shared_page`), with the counter
//! that orders the calls of all of them. When that frame of Display8's comes, every
//! recorder writes what its plugin calls, until the frame is over:
//!
//! - `Dumps\<name>\<module>.a2s`: the stream (`wire`), and `<module>.txt` beside it,
//!   what was recorded and what could not be;
//! - `Dumps\<name>\blobs\`: the contents of textures and buffers, by hash.
//!
//! **What a frame finds made** (a texture, a buffer, a storage buffer made in an earlier
//! frame, the engine's back buffer, a mirror) is recorded at its first use in the frame:
//! its description and its contents as they are then (a snapshot), read back from the
//! device. So the replay starts where the frame did. Reading back flushes the frame so
//! far, which changes nothing it draws, only how long the frame takes.
//!
//! **Off, it costs** two flags per call and a read of the clock per `Cmd` begun; the
//! trigger file is looked for once half a second has passed since the last look.
//!
//! The recorder needs the Vulkan backend: it reads back what the frame finds made
//! through it. On another it says so once and records nothing.
pub mod wire;
pub mod sys;

use crate::types::*;
use crate::vulkan;
use crate::{Buffer, Frame, Span, Storage, Texture};
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::fmt::Write;
use wire::*;

pub type Log = fn(core::fmt::Arguments);

/// A `String` of the parts (`alloc::format!` does not link in the plugins).
macro_rules! cat {
    ($($t:tt)*) => {{
        let mut s = String::new();
        let _ = s.write_fmt(format_args!($($t)*));
        s
    }};
}

/// The trigger file's name, beside the exe.
pub const TRIGGER: &str = "Gpu.record";

const PAGE_MAGIC: u32 = 0x5245_4741; // "AGER"
const TOKEN: usize = 128;
/// In `mirror_levels`: the engine's texture is X8R8G8B8.
const X8: u32 = 0x8000_0000;

/// What every recorder in the process shares.
#[repr(C)]
struct Page {
    magic: u32,
    /// Calls recorded so far, by every plugin: the next record's `seq`.
    seq: u32,
    /// Display8's frame to record (0: none asked for).
    frame: u64,
    /// The request's trigger text, which tells one request from the next.
    token: [u8; TOKEN],
    /// Streams written for it.
    done: u32,
}

/// The `Gpu` whose recorder is recording now (Texture::write and Frame::next have no
/// `Gpu` to ask): one per plugin, as each plugin links this crate.
static mut ACTIVE: usize = 0;

pub(crate) fn active() -> usize {
    unsafe { ACTIVE }
}

/// A span written in the frame (`vertices`, `indices`, `span_of`), to tell which one a
/// draw reads.
struct Alloc {
    vk: u64,
    buf: usize,
    stride: u32,
    first: u32,
    count: u32,
    id: u32,
}

struct State {
    page: *mut Page,
    name: String,
    dir: String,
    module: String,
    frame: u64,
    out: Vec<u8>,
    next: u32,
    tex: BTreeMap<usize, u32>,
    bufs: BTreeMap<usize, u32>,
    stos: BTreeMap<usize, u32>,
    pipes: BTreeMap<usize, u32>,
    cpipes: BTreeMap<usize, u32>,
    codes: BTreeMap<(usize, usize), u32>,
    frames: BTreeMap<usize, u32>,
    allocs: Vec<Alloc>,
    mirror_levels: BTreeMap<usize, u32>,
    blobs: BTreeSet<u64>,
    notes: String,
    records: u32,
    snaps: u32,
    snap_bytes: u64,
    missed_spans: u32,
}

pub struct Recorder {
    log: Log,
    /// Recording Display8's frame `State::frame` now.
    on: Cell<bool>,
    /// A frame is chosen and not come yet.
    armed: Cell<bool>,
    next_poll: Cell<u32>,
    told: Cell<bool>,
    /// The last look's tick and Display8's frame then (the frame rate, to choose how far
    /// ahead to record).
    last: Cell<(u32, u64)>,
    /// The trigger text last acted on.
    handled: RefCell<Vec<u8>>,
    st: RefCell<Option<Box<State>>>,
}

fn key<T>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as usize
}

impl Recorder {
    pub const fn new(log: Log) -> Recorder {
        Recorder {
            log,
            on: Cell::new(false),
            armed: Cell::new(false),
            next_poll: Cell::new(0),
            told: Cell::new(false),
            last: Cell::new((0, 0)),
            handled: RefCell::new(Vec::new()),
            st: RefCell::new(None),
        }
    }

    /// Recording now: every call is to be written.
    #[inline]
    pub fn on(&self) -> bool {
        self.on.get()
    }

    /// A `Cmd` has begun on `vk` (`on_vulkan`: the backend is Vulkan), into `frame`; its
    /// id in the stream, 0 when not recording. Looks for the trigger now and then, and
    /// starts or ends the recording at Display8's frame boundaries. `me` is the `Gpu`
    /// this recorder is in.
    pub fn begin(&self, vk: &vulkan::Gpu, on_vulkan: bool, me: usize, frame: *const Frame) -> u32 {
        if !self.on.get() && !self.armed.get() {
            // Every `Cmd` asks the clock (a read of shared memory): a plugin that begins a
            // few a frame must still look twice a second, whatever the frame rate.
            let t = sys::ticks();
            if (t.wrapping_sub(self.next_poll.get()) as i32) < 0 {
                return 0;
            }
            self.next_poll.set(t.wrapping_add(500));
            self.poll(vk, on_vulkan, me);
            if !self.armed.get() {
                return 0;
            }
        }
        if !self.live(vk, me) {
            return 0;
        }
        let fid = self.frame_id(frame as usize);
        // A `Cmd` starts with the engine's viewport (Display8's record of it), which no
        // call of the plugin's says: recorded with it.
        let vp = vulkan::engine_viewport(vk);
        let id = self.with(|s| {
            let id = s.id();
            let mut w = W::default();
            w.u32(id).u32(fid);
            match vp {
                Some(v) => w.u8(1).u32(v.x).u32(v.y).u32(v.w).u32(v.h).f32(v.min_z).f32(v.max_z),
                None => w.u8(0),
            };
            (id, w)
        });
        let Some((id, w)) = id else { return 0 };
        self.emit(OP_BEGIN, &w);
        id
    }

    /// Whether the frame is being recorded now, the recording started or ended at
    /// Display8's frame boundaries: asked of Display8 (the frame now, which begins it
    /// after a `Present`) at every call while a recording is asked for or under way, so
    /// what a plugin does before its first `Cmd` of the frame is recorded too. Two flags
    /// otherwise.
    #[inline]
    pub fn live(&self, vk: &vulkan::Gpu, me: usize) -> bool {
        if !self.on.get() && !self.armed.get() {
            return false;
        }
        self.at_frame(vk, me)
    }

    fn at_frame(&self, vk: &vulkan::Gpu, me: usize) -> bool {
        let Some(now) = vk.rec_frame_live() else { return self.on.get() };
        let f = self.st.borrow().as_ref().map_or(0, |s| s.frame);
        if self.on.get() {
            if now != f {
                self.finish("its frame ended");
                return false;
            }
            return true;
        }
        if now < f {
            return false;
        }
        if now > f {
            (self.log)(format_args!("gpu: record: frame {} was asked for and this plugin first drew in frame {}: nothing recorded", f, now));
            self.drop_state();
            return false;
        }
        self.start(vk, me);
        true
    }

    /// The plugin is at the engine's `Present`: the frame is over. The recording's name
    /// when this plugin recorded it (its streams are written now), for the plugin to dump
    /// the frame as presented.
    pub fn present(&self, vk: &vulkan::Gpu) -> Option<String> {
        if !self.on.get() {
            return None;
        }
        let (f, name) = self.st.borrow().as_ref().map(|s| (s.frame, s.name.clone()))?;
        if vk.rec_frame_live() != Some(f) {
            self.finish("its frame ended");
            return None;
        }
        self.finish("the engine presented it");
        Some(name)
    }

    fn poll(&self, vk: &vulkan::Gpu, on_vulkan: bool, me: usize) {
        let dir = sys::exe_dir();
        let trigger = cat!("{}{}", dir, TRIGGER);
        let now = vk.rec_frame();
        let t = sys::ticks();
        let (lt, lf) = self.last.get();
        if let Some(n) = now {
            self.last.set((t, n));
        }
        if !sys::exists(&trigger) {
            return;
        }
        let Some(text) = sys::read(&trigger) else { return };
        let text: Vec<u8> = text.into_iter().take(TOKEN - 1).collect();
        if *self.handled.borrow() == text {
            return;
        }
        if !on_vulkan {
            if !self.told.replace(true) {
                (self.log)(format_args!("gpu: record: {} asks for a recording, which needs Display8's Vulkan backend (Mode=stub, Backend=vulkan): nothing recorded", TRIGGER));
            }
            *self.handled.borrow_mut() = text;
            return;
        }
        let Some(now) = vk.rec_frame_live().or(now) else { return };
        let page = sys::shared_page(core::mem::size_of::<Page>()) as *mut Page;
        if page.is_null() {
            (self.log)(format_args!("gpu: record: the page the plugins share could not be made: nothing recorded"));
            *self.handled.borrow_mut() = text;
            return;
        }
        let p = unsafe { &mut *page };
        let same = p.magic == PAGE_MAGIC && p.token[..text.len()] == text[..] && p.token[text.len()] == 0;
        let frame = if same {
            if p.frame <= now {
                // Asked for before this plugin looked, and gone.
                (self.log)(format_args!("gpu: record: frame {} was asked for and has begun or is past (now {}): nothing recorded", p.frame, now));
                *self.handled.borrow_mut() = text;
                return;
            }
            p.frame
        } else {
            // Far enough ahead that every other plugin looks before it comes: they look
            // every half second, so a second and a half of frames, at the rate of the
            // last half second.
            let dt = t.wrapping_sub(lt).max(1) as u64;
            let fps = if lf > 0 && now >= lf { (now - lf) * 1000 / dt } else { 30 };
            let ahead = (fps * 3 / 2 + 2).clamp(4, 600);
            p.magic = PAGE_MAGIC;
            p.token = [0; TOKEN];
            p.token[..text.len()].copy_from_slice(&text);
            p.frame = now + ahead;
            p.done = 0;
            p.frame
        };
        let first = text.split(|&c| c == b'\n' || c == b'\r').next().unwrap_or(&[]);
        let mut name = String::new();
        for &c in first.iter().take(64) {
            let c = c as char;
            name.push(if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' { c } else { '-' });
        }
        if name.is_empty() {
            name.push_str("record");
        }
        let rdir = cat!("{}Dumps\\{}\\", dir, name);
        if !sys::mkdir(&cat!("{}Dumps", dir)) || !sys::mkdir(&rdir) || !sys::mkdir(&cat!("{}blobs", rdir)) {
            (self.log)(format_args!("gpu: record: {} could not be made: nothing recorded", rdir));
            *self.handled.borrow_mut() = text;
            return;
        }
        let (_, module) = sys::module_of(core::ptr::addr_of!(ACTIVE) as usize);
        (self.log)(format_args!("gpu: record: \"{}\" asked for: Display8's frame {} (now {}) into {}", name, frame, now, rdir));
        *self.handled.borrow_mut() = text;
        *self.st.borrow_mut() = Some(Box::new(State {
            page,
            name,
            dir: rdir,
            module,
            frame,
            out: Vec::new(),
            next: 1,
            tex: BTreeMap::new(),
            bufs: BTreeMap::new(),
            stos: BTreeMap::new(),
            pipes: BTreeMap::new(),
            cpipes: BTreeMap::new(),
            codes: BTreeMap::new(),
            frames: BTreeMap::new(),
            allocs: Vec::new(),
            mirror_levels: BTreeMap::new(),
            blobs: BTreeSet::new(),
            notes: String::new(),
            records: 0,
            snaps: 0,
            snap_bytes: 0,
            missed_spans: 0,
        }));
        self.armed.set(true);
        unsafe { ACTIVE = me };
    }

    fn start(&self, vk: &vulkan::Gpu, _me: usize) {
        self.on.set(true);
        self.armed.set(false);
        let mut h = W::default();
        let ct = vk.engine_target().map(|t| t.desc());
        let dt = vk.engine_depth().map(|t| t.desc());
        self.with(|s| {
            h.u32(VERSION).str(&s.module).str("vulkan").str(&s.name).u64(s.frame);
            let c = ct.unwrap_or(TextureDesc::d2(0, 0, Format::Unknown, Usage(0)));
            h.u32(c.width).u32(c.height).u32(c.format as u32);
            h.u32(dt.map_or(0, |d| d.format as u32));
            h.bool(vk.d3d9_centres());
            s.out.extend_from_slice(MAGIC);
            s.out.extend_from_slice(&(h.buf.len() as u32).to_le_bytes());
            s.out.extend_from_slice(&h.buf);
        });
        (self.log)(format_args!("gpu: record: recording Display8's frame {}", self.st.borrow().as_ref().map_or(0, |s| s.frame)));
    }

    fn finish(&self, why: &str) {
        self.on.set(false);
        self.armed.set(false);
        unsafe { ACTIVE = 0 };
        let Some(mut s) = self.st.borrow_mut().take() else { return };
        let seq = s.seq();
        put(&mut s.out, seq, OP_END_STREAM, &[]);
        s.records += 1;
        let file = cat!("{}{}.a2s", s.dir, s.module);
        let ok = sys::write(&file, &s.out);
        let mut side = String::new();
        let _ = write!(
            side,
            "module {}\r\nname {}\r\nframe {}\r\nended: {}\r\nrecords {}\r\nstream bytes {}\r\nsnapshots {} ({} bytes)\r\nblobs {}\r\ndraws whose span was not found: {}\r\n{}",
            s.module, s.name, s.frame, why, s.records, s.out.len(), s.snaps, s.snap_bytes, s.blobs.len(), s.missed_spans, s.notes
        );
        sys::write(&cat!("{}{}.txt", s.dir, s.module), side.as_bytes());
        unsafe { (*s.page).done += 1 };
        (self.log)(format_args!(
            "gpu: record: frame {} ({}): {} records, {} snapshots ({} MB) into {}{}",
            s.frame, why, s.records, s.snaps, s.snap_bytes >> 20, file, if ok { "" } else { ": could not be written" }
        ));
    }

    fn drop_state(&self) {
        self.on.set(false);
        self.armed.set(false);
        unsafe { ACTIVE = 0 };
        *self.st.borrow_mut() = None;
    }

    fn with<R>(&self, f: impl FnOnce(&mut State) -> R) -> Option<R> {
        let mut st = self.st.borrow_mut();
        Some(f(st.as_mut()?))
    }

    fn emit(&self, op: u16, w: &W) {
        self.with(|s| {
            let seq = s.seq();
            put(&mut s.out, seq, op, &w.buf);
            s.records += 1;
        });
    }

    fn note(&self, line: core::fmt::Arguments) {
        self.with(|s| {
            let _ = s.notes.write_fmt(line);
            s.notes.push_str("\r\n");
        });
    }

    fn frame_id(&self, ptr: usize) -> u32 {
        self.with(|s| {
            if let Some(&id) = s.frames.get(&ptr) {
                return id;
            }
            let id = s.id();
            s.frames.insert(ptr, id);
            id
        })
        .unwrap_or(0)
    }

    /// Writes `data` as a blob (once per hash).
    pub fn blob(&self, data: &[u8]) -> Blob {
        if data.is_empty() {
            return Blob::default();
        }
        let hash = fnv(data);
        let b = Blob { hash, len: data.len() as u32 };
        let path = self.with(|s| {
            if s.blobs.insert(hash) {
                Some(cat!("{}blobs\\{}", s.dir, blob_name(hash, b.len)))
            } else {
                None
            }
        });
        if let Some(Some(p)) = path {
            if !sys::exists(&p) && !sys::write(&p, data) {
                self.note(format_args!("blob {} could not be written", p));
            }
        }
        b
    }

    // -- objects --

    /// A texture made in the frame (`Gpu::texture`), with what it was made with.
    pub fn texture_new(&self, t: &Texture, desc: &TextureDesc, b: Blob) {
        let Texture::Vk(vt) = t else { return };
        let id = self.with(|s| {
            let id = s.id();
            s.tex.insert(key(&vt.0), id);
            id
        });
        let Some(id) = id else { return };
        let mut w = W::default();
        w.u32(id).tex_desc(desc).blob(b);
        self.emit(OP_TEX_NEW, &w);
    }

    pub fn buffer_new(&self, b: &Buffer, desc: &BufferDesc, data: &[u8]) {
        let Buffer::Vk(vb) = b else { return };
        let bl = self.blob(data);
        let id = self.with(|s| {
            let id = s.id();
            s.bufs.insert(key(&vb.0), id);
            id
        });
        let Some(id) = id else { return };
        let mut w = W::default();
        w.u32(id).buf_desc(desc).blob(bl);
        self.emit(OP_BUF_NEW, &w);
    }

    pub fn storage_new(&self, st: &Storage, desc: &StorageDesc) {
        let Storage::Vk(vs) = st else { return };
        let id = self.with(|s| {
            let id = s.id();
            s.stos.insert(key(&vs.0), id);
            id
        });
        let Some(id) = id else { return };
        let mut w = W::default();
        w.u32(id).u32(desc.size).u32(desc.stride);
        self.emit(OP_STO_NEW, &w);
    }

    /// An engine texture's mirror, its levels and whether it is X8R8G8B8 (its alpha
    /// read as one: Display8's view says so, the image does not) (`Gpu::engine_texture`).
    pub fn mirror(&self, t: &Texture, levels: u32, x8: bool) {
        let Texture::Vk(vt) = t else { return };
        self.with(|s| s.mirror_levels.insert(key(&vt.0), levels | if x8 { X8 } else { 0 }));
    }

    /// `t`'s id, recording it with its contents (`contents`) the first time the frame
    /// uses it, when it was made before (0: none).
    pub fn tex(&self, vk: &vulkan::Gpu, t: Option<&Texture>, contents: bool) -> u32 {
        let Some(Texture::Vk(vt)) = t else { return 0 };
        let k = key(&vt.0);
        let (id, levels) = match self.with(|s| match s.tex.get(&k) {
            Some(&id) => (id, None),
            None => {
                let id = s.id();
                s.tex.insert(k, id);
                (id, Some(s.mirror_levels.get(&k).copied().unwrap_or(0)))
            }
        }) {
            Some(x) => x,
            None => return 0,
        };
        let Some(levels) = levels else { return id };
        let role = vk.rec_role(vt);
        let mut desc = vt.desc();
        if role == ROLE_MIRROR && levels & X8 != 0 {
            desc.format = Format::Bgrx8;
        }
        let levels = if role == ROLE_MIRROR { (levels & !X8).max(1) } else { 0 };
        if levels > 0 {
            desc.levels = levels;
        }
        let blob = if contents {
            match vk.rec_texture(vt, levels) {
                Ok(v) => {
                    self.with(|s| {
                        s.snaps += 1;
                        s.snap_bytes += v.len() as u64;
                    });
                    self.blob(&v)
                }
                Err(why) => {
                    self.note(format_args!(
                        "texture {} ({}x{} {} {:?}, {} samples, role {}): no contents: {}",
                        id, desc.width, desc.height, desc.format.name(), desc.kind, desc.samples, role, why
                    ));
                    Blob::default()
                }
            }
        } else {
            Blob::default()
        };
        let mut w = W::default();
        w.u32(id).tex_desc(&desc).u32(role).blob(blob);
        self.emit(OP_TEX_SNAP, &w);
        id
    }

    pub fn buf(&self, vk: &vulkan::Gpu, b: &Buffer) -> u32 {
        let Buffer::Vk(vb) = b else { return 0 };
        let k = key(&vb.0);
        let (id, new) = match self.with(|s| match s.bufs.get(&k) {
            Some(&id) => (id, false),
            None => {
                let id = s.id();
                s.bufs.insert(k, id);
                (id, true)
            }
        }) {
            Some(x) => x,
            None => return 0,
        };
        if !new {
            return id;
        }
        let blob = match vk.rec_buffer(vb) {
            Ok(v) => {
                self.with(|s| {
                    s.snaps += 1;
                    s.snap_bytes += v.len() as u64;
                });
                self.blob(&v)
            }
            Err(why) => {
                self.note(format_args!("buffer {}: no contents: {}", id, why));
                Blob::default()
            }
        };
        let mut w = W::default();
        w.u32(id).buf_desc(&vb.desc()).blob(blob);
        self.emit(OP_BUF_SNAP, &w);
        id
    }

    pub fn sto(&self, vk: &vulkan::Gpu, st: Option<&Storage>) -> u32 {
        let Some(Storage::Vk(vs)) = st else { return 0 };
        let k = key(&vs.0);
        let (id, new) = match self.with(|s| match s.stos.get(&k) {
            Some(&id) => (id, false),
            None => {
                let id = s.id();
                s.stos.insert(k, id);
                (id, true)
            }
        }) {
            Some(x) => x,
            None => return 0,
        };
        if !new {
            return id;
        }
        let blob = match vk.rec_storage(vs) {
            Ok(v) => self.blob(&v),
            Err(why) => {
                self.note(format_args!("storage {}: no contents: {}", id, why));
                Blob::default()
            }
        };
        let d = vs.desc();
        let mut w = W::default();
        w.u32(id).u32(d.size).u32(d.stride).blob(blob);
        self.emit(OP_STO_SNAP, &w);
        id
    }

    fn code(&self, words: &'static [u32]) -> u32 {
        if words.is_empty() {
            return 0;
        }
        let k = (words.as_ptr() as usize, words.len());
        let r = self.with(|s| match s.codes.get(&k) {
            Some(&id) => (id, false),
            None => {
                let id = s.id();
                s.codes.insert(k, id);
                (id, true)
            }
        });
        let Some((id, new)) = r else { return 0 };
        if new {
            let mut w = W::default();
            w.u32(id).u32(words.len() as u32);
            for x in words {
                w.u32(*x);
            }
            self.emit(OP_CODE, &w);
        }
        id
    }

    fn codes(&self, c: &Code) -> [u32; 4] {
        [self.code(c.d3d9), self.code(c.spirv), self.code(c.dxil), self.code(c.dxil_at)]
    }

    pub fn pipe(&self, p: &vulkan::Pipeline) -> u32 {
        let k = key(&p.0);
        let r = self.with(|s| match s.pipes.get(&k) {
            Some(&id) => (id, false),
            None => {
                let id = s.id();
                s.pipes.insert(k, id);
                (id, true)
            }
        });
        let Some((id, new)) = r else { return 0 };
        if new {
            let d = p.desc();
            let (vs, ps) = (self.codes(&d.vs), self.codes(&d.ps));
            let mut w = W::default();
            w.u32(id).pipe_desc(d, vs, ps);
            self.emit(OP_PIPE, &w);
        }
        id
    }

    pub fn cpipe(&self, p: &vulkan::ComputePipeline) -> u32 {
        let k = key(&p.0);
        let r = self.with(|s| match s.cpipes.get(&k) {
            Some(&id) => (id, false),
            None => {
                let id = s.id();
                s.cpipes.insert(k, id);
                (id, true)
            }
        });
        let Some((id, new)) = r else { return 0 };
        if new {
            let cs = self.codes(&p.0.desc.cs);
            let mut w = W::default();
            w.u32(id);
            for c in cs {
                w.u32(c);
            }
            self.emit(OP_CPIPE, &w);
        }
        id
    }

    // -- commands --

    pub fn op(&self, op: u16, f: impl FnOnce(&mut W)) {
        let mut w = W::default();
        f(&mut w);
        self.emit(op, &w);
    }

    pub fn end(&self, cmd: u32) {
        if self.on.get() {
            self.op(OP_END, |w| {
                w.u32(cmd);
            });
        }
    }

    pub fn frame_next(&self, frame: *const Frame) {
        let id = self.frame_id(frame as usize);
        self.op(OP_FRAME_NEXT, |w| {
            w.u32(id);
        });
    }

    pub fn begin_pass(&self, vk: &vulkan::Gpu, cmd: u32, pass: &Pass) {
        let mut ids = [0u32; MAX_COLORS];
        for (i, c) in pass.colors.iter().enumerate().take(MAX_COLORS) {
            let cleared = matches!(c.load, Load::Clear(_)) && pass.area.is_none();
            ids[i] = self.tex(vk, Some(c.texture), !cleared);
        }
        let did = pass.depth.as_ref().map(|d| self.tex(vk, Some(d.texture), false));
        self.op(OP_BEGIN_PASS, |w| {
            w.u32(cmd).u32(pass.colors.len().min(MAX_COLORS) as u32);
            for (i, c) in pass.colors.iter().enumerate().take(MAX_COLORS) {
                w.u32(ids[i]).u32(c.layer);
                match c.load {
                    Load::Load => w.u8(0).floats(&[]),
                    Load::Clear(v) => w.u8(1).floats(&v),
                    Load::DontCare => w.u8(2).floats(&[]),
                };
            }
            match (&pass.depth, did) {
                (Some(d), Some(id)) => {
                    w.u8(1).u32(id);
                    match d.load {
                        Load::Load => w.u8(0).f32(0.0),
                        Load::Clear(v) => w.u8(1).f32(v),
                        Load::DontCare => w.u8(2).f32(0.0),
                    };
                }
                _ => {
                    w.u8(0);
                }
            }
            w.opt_rect(pass.area);
        });
    }

    /// A span written into the frame, as the backend gave it.
    pub fn span(&self, op: u16, cmd: u32, data: &[u8], stride: u32, buffer: u32, got: Option<&Span>) {
        let id = self.with(|s| {
            let id = s.id();
            if let Some(sp) = got {
                s.allocs.push(Alloc { vk: sp.vk, buf: sp.buf as usize, stride: sp.stride, first: sp.first, count: sp.count, id });
            }
            id
        });
        let Some(id) = id else { return };
        self.op(op, |w| {
            w.u32(cmd).u32(id).u32(stride);
            match op {
                OP_SPAN_OF => w.u32(buffer),
                _ => w.bytes(data),
            };
            w.bool(got.is_some());
        });
    }

    /// A span a draw reads, as the span written it is part of: its id, its first
    /// element from that one's, its count and stride.
    pub fn span_ref(&self, w: &mut W, sp: &Span) {
        let r = self.with(|s| {
            let found = s
                .allocs
                .iter()
                .rev()
                .find(|a| a.vk == sp.vk && a.buf == sp.buf as usize && a.stride == sp.stride && sp.first >= a.first && sp.first + sp.count <= a.first + a.count)
                .map(|a| (a.id, sp.first - a.first));
            if found.is_none() {
                s.missed_spans += 1;
            }
            found
        });
        let (id, delta) = r.flatten().unwrap_or((0, 0));
        w.u32(id).u32(delta).u32(sp.count).u32(sp.stride);
    }

    /// A texture's `write` (rows of `w` texels `pitch` bytes apart), packed.
    pub fn write(&self, vk: &vulkan::Gpu, t: &Texture, src: *const u8, pitch: usize, wid: u32, rows: u32) {
        let id = self.tex(vk, Some(t), true);
        let d = t.desc();
        let bpp = d.format.bytes().unwrap_or(4);
        let (wid, rows) = (wid.min(d.width) as usize, rows.min(d.height) as usize);
        let mut data = Vec::with_capacity(wid * bpp * rows);
        for y in 0..rows {
            data.extend_from_slice(unsafe { core::slice::from_raw_parts(src.add(y * pitch), wid * bpp) });
        }
        self.op(OP_TEX_WRITE, |w| {
            w.u32(id).u32((wid * bpp) as u32).u32(wid as u32).u32(rows as u32).bytes(&data);
        });
    }
}

impl State {
    fn id(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        id
    }

    fn seq(&mut self) -> u32 {
        let p = unsafe { &mut *self.page };
        let s = p.seq;
        p.seq = s.wrapping_add(1);
        s
    }
}

fn put(out: &mut Vec<u8>, seq: u32, op: u16, payload: &[u8]) {
    out.extend_from_slice(&seq.to_le_bytes());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
}
