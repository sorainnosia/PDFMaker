use std::any::Any;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use wasmtime::{
    Caller, Config, Engine, Extern, ExternRef, Linker, Module, Ref, Rooted, Store, Val,
};

const WASM_URL: &str = "https://pdfmaker.ink/web/pkg/pdfmaker_bg.wasm";
const WASM_CACHE_NAME: &str = "pdfmaker_bg.wasm";

/// Host-side stand-ins for the JS values the engine passes around as `externref`.
/// Only the variants the synchronous `html_to_pdf` path needs are modelled.
#[derive(Clone)]
enum HostRef {
    Undefined,
    Null,
    Bool(bool),
    /// `globalThis` — an object, so `is_object` is true.
    Global,
    /// `globalThis.crypto` — an object with `getRandomValues`.
    Crypto,
    /// The product of a `(ptr, len) -> externref` cast intrinsic. The same wasm-bindgen
    /// signature is used both for `Ref(String) -> Externref` (consumed by `console.log`,
    /// read as `text`) and `Ref(Slice<u8>) -> Uint8Array` (consumed by `getRandomValues`,
    /// which writes into wasm memory at `ptr..ptr+len`). We can't tell them apart by
    /// signature, so we keep BOTH the live ptr/len (for writing) and an eager UTF-8 snapshot
    /// (for reading — safe even after the producing call returns).
    Mem { ptr: u32, len: u32, text: String },
}

/// Per-store host state: a tiny PRNG to back `getRandomValues`. The engine only uses
/// randomness for HashMap DoS seeds and the PDF document id, so a time-seeded splitmix64
/// is entirely sufficient (and avoids an OS-rng dependency).
struct HostState {
    rng: u64,
}

impl HostState {
    fn next_u64(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ERROR: {e:#}");
        std::process::exit(1);
    }
}

struct Args {
    input: Option<PathBuf>,
    css: Option<PathBuf>,
    output: Option<PathBuf>,
    paper: Option<(f32, f32)>,
    /// `--upgrade`: delete the cached Engine and re-download the latest from the URL.
    upgrade: bool,
}

fn run() -> anyhow::Result<()> {
    let args = parse_args()?;

    // 1. Ensure the Engine is cached locally. `--upgrade` deletes any cached copy and
    //    re-downloads the latest from the URL first.
    let wasm_path = ensure_wasm(args.upgrade)?;

    let input = match args.input {
        Some(p) => p,
        None => {
            // `--upgrade` with no input file: just refresh the engine and exit.
            println!("Upgraded Engine");
            return Ok(());
        }
    };

    // 2. Read inputs.
    let html = std::fs::read_to_string(&input)
        .map_err(|e| anyhow::anyhow!("reading input {}: {e}", input.display()))?;
    let css = match &args.css {
        Some(p) => Some(
            std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("reading css {}: {e}", p.display()))?,
        ),
        None => None,
    };

    // 3. Run html_to_pdf in the wasm.
    let pdf = convert(&wasm_path, &html, css.as_deref(), args.paper)?;

    // 4. Pick output path (auto Document.pdf / Document-2.pdf / ... when not given).
    let out = match args.output {
        Some(p) => p,
        None => next_document_path(),
    };
    std::fs::write(&out, &pdf)
        .map_err(|e| anyhow::anyhow!("writing {}: {e}", out.display()))?;
    println!("SUCCESS: wrote {} ({} bytes)", out.display(), pdf.len());
    Ok(())
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn parse_args() -> anyhow::Result<Args> {
    let mut input = None;
    let mut css = None;
    let mut output = None;
    let mut paper = None;
    let mut upgrade = false;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-i" | "--input" => input = Some(PathBuf::from(need(&mut it, "-i")?)),
            "-c" | "--css" => css = Some(PathBuf::from(need(&mut it, "-c")?)),
            "-o" | "--output" => output = Some(PathBuf::from(need(&mut it, "-o")?)),
            "-p" | "--paper" | "--size" => paper = Some(parse_paper(&need(&mut it, "-p")?)?),
            "-u" | "--upgrade" => upgrade = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {other} (use --help)"),
        }
    }
    // An input is required, except when only refreshing the engine with --upgrade.
    if input.is_none() && !upgrade {
        anyhow::bail!("missing required -i <input.html> (use --help)");
    }
    Ok(Args { input, css, output, paper, upgrade })
}

fn need(it: &mut impl Iterator<Item = String>, flag: &str) -> anyhow::Result<String> {
    it.next().ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "pdfmaker — HTML to PDF via built-in Engine\n\n\
         USAGE:\n  pdfmaker -i <input.html> [-c <style.css>] [-p <paper>] [-o <output.pdf>]\n\n\
         OPTIONS:\n  \
         -i, --input   Input HTML file (required)\n  \
         -c, --css     Extra CSS file (optional)\n  \
         -p, --paper   Paper size: A3 A4 A5 Letter Legal, or WIDTHxHEIGHT in points\n                \
         (optional; default uses the @page size in the document's CSS)\n  \
         -o, --output  Output PDF (optional; defaults to Document.pdf, Document-2.pdf, ...)\n  \
         -u, --upgrade Delete the cached Engine and re-download the latest from the URL\n                \
         (can be used alone to just refresh, or together with a conversion)\n"
    );
}

/// Resolve a paper-size argument to (width, height) in PDF points.
fn parse_paper(s: &str) -> anyhow::Result<(f32, f32)> {
    let dims = match s.to_ascii_lowercase().as_str() {
        "a3" => (841.8898, 1190.5512),
        "a4" => (595.2756, 841.8898),
        "a5" => (419.5276, 595.2756),
        "letter" => (612.0, 792.0),
        "legal" => (612.0, 1008.0),
        "tabloid" => (792.0, 1224.0),
        custom => {
            // WIDTHxHEIGHT in points, e.g. "595x842".
            let (w, h) = custom
                .split_once('x')
                .ok_or_else(|| anyhow::anyhow!("unknown paper size: {s} (try A4, Letter, or WxH)"))?;
            (
                w.trim().parse::<f32>().map_err(|_| anyhow::anyhow!("bad width in {s}"))?,
                h.trim().parse::<f32>().map_err(|_| anyhow::anyhow!("bad height in {s}"))?,
            )
        }
    };
    Ok(dims)
}

/// First non-existing Document.pdf / Document-2.pdf / Document-3.pdf in the cwd.
fn next_document_path() -> PathBuf {
    let first = PathBuf::from("Document.pdf");
    if !first.exists() {
        return first;
    }
    for n in 2.. {
        let p = PathBuf::from(format!("Document-{n}.pdf"));
        if !p.exists() {
            return p;
        }
    }
    unreachable!()
}

// ---------------------------------------------------------------------------
// Engine download / cache
// ---------------------------------------------------------------------------

/// Path the Engine is cached at: `<temp>/pdfmaker/pdfmaker_bg.wasm`.
fn wasm_cache_path() -> PathBuf {
    std::env::temp_dir().join("pdfmaker").join(WASM_CACHE_NAME)
}

fn ensure_wasm(force: bool) -> anyhow::Result<PathBuf> {
    let path = wasm_cache_path();
    if force {
        // --upgrade: drop any cached copy so the latest is re-downloaded below.
        let _ = std::fs::remove_file(&path);
    } else if path.exists() {
        // If a cached Engine exists, use it as-is — no size or freshness check. A truncated
        // or corrupt file is dealt with at load time: the loader deletes it so the next run
        // re-downloads (see `convert`).
        return Ok(path);
    }

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("creating cache dir {}: {e}", dir.display()))?;
    }
    // `identity` so we store the real Engine bytes even if the server can gzip.
    let resp = ureq::get(WASM_URL)
        .set("Accept-Encoding", "identity")
        .call()
        .map_err(|e| anyhow::anyhow!("downloading Engine: {e}"))?;
    let mut bytes = Vec::new();
    std::io::copy(&mut resp.into_reader(), &mut bytes)
        .map_err(|e| anyhow::anyhow!("reading Engine body: {e}"))?;
    if bytes.len() < 1024 {
        anyhow::bail!("downloaded Engine is suspiciously small ({} bytes)", bytes.len());
    }
    // Write atomically (tmp + rename) so a half-download never poisons the cache.
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &bytes)
        .map_err(|e| anyhow::anyhow!("writing Engine cache: {e}"))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| anyhow::anyhow!("finalizing Engine cache: {e}"))?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// wasmtime host
// ---------------------------------------------------------------------------

/// Read a HostRef out of an externref argument.
fn host_ref(caller: &Caller<'_, HostState>, r: Option<Rooted<ExternRef>>) -> Option<HostRef> {
    let r = r?;
    let data: &(dyn Any + Send + Sync) = r.data(caller).ok().flatten()?;
    data.downcast_ref::<HostRef>().cloned()
}

fn convert(
    wasm_path: &Path,
    html: &str,
    css: Option<&str>,
    paper: Option<(f32, f32)>,
) -> anyhow::Result<Vec<u8>> {
    let mut config = Config::new();
    config.wasm_reference_types(true);
    let engine = Engine::new(&config)?;
    // A truncated or corrupt cached file fails to parse/validate here. Delete it so the
    // next run downloads a fresh copy.
    let module = match Module::from_file(&engine, wasm_path) {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_file(wasm_path);
            anyhow::bail!("loading program failed (re-run to fix): {e}");
        }
    };

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678)
        | 1;
    let mut store = Store::new(&engine, HostState { rng: seed });

    let mut linker: Linker<HostState> = Linker::new(&engine);
    // Bind imports by matching the Engine's actual import list (auto-adapting to whatever
    // hash suffixes the downloaded wasm-bindgen build used); everything else traps.
    define_imports(&mut linker, &module)?;

    let instance = linker.instantiate(&mut store, &module)?;

    // wasm-bindgen runs its initialisation (externref table setup, ctor side effects)
    // from this export rather than a Engine start section.
    if let Ok(start) = instance.get_typed_func::<(), ()>(&mut store, "__wbindgen_start") {
        start.call(&mut store, ())?;
    }

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| anyhow::anyhow!("Engine has no `memory` export"))?;
    let malloc = instance.get_typed_func::<(i32, i32), i32>(&mut store, "__wbindgen_malloc")?;
    let free = instance.get_typed_func::<(i32, i32, i32), ()>(&mut store, "__wbindgen_free")?;
    let html_to_pdf = instance
        .get_func(&mut store, "html_to_pdf")
        .ok_or_else(|| anyhow::anyhow!("Engine has no `html_to_pdf` export"))?;

    // Marshal the &str arguments into Engine memory the way wasm-bindgen's
    // `passStringToWasm0` does: malloc(len, 1) then copy the UTF-8 bytes.
    let (html_ptr, html_len) = pass_string(&mut store, &malloc, &memory, html)?;
    let (css_ptr, css_len) = match css {
        Some(c) => pass_string(&mut store, &malloc, &memory, c)?,
        None => (0, 0), // wasm-bindgen encodes `None` for an Option<String> as ptr 0.
    };

    // Option<f32> is passed as f64: the value when Some, or the sentinel 0x1_0000_0001
    // (4294967297.0) when None. fround() to mirror the JS Math.fround.
    let (width, height) = match paper {
        Some((w, h)) => (opt_f32(Some(w)), opt_f32(Some(h))),
        None => (opt_f32(None), opt_f32(None)),
    };

    let params = [
        Val::I32(html_ptr),
        Val::I32(html_len),
        Val::I32(css_ptr),
        Val::I32(css_len),
        Val::F64(width.to_bits()),
        Val::F64(height.to_bits()),
    ];
    // Returns (ptr, len, err_ref, is_err) via multi-value.
    let mut results = [Val::I32(0), Val::I32(0), Val::I32(0), Val::I32(0)];
    html_to_pdf
        .call(&mut store, &params, &mut results)
        .map_err(|e| anyhow::anyhow!("html_to_pdf trapped: {e}"))?;

    let ptr = results[0].i32().unwrap();
    let len = results[1].i32().unwrap();
    let err_idx = results[2].i32().unwrap();
    let is_err = results[3].i32().unwrap();
    if is_err != 0 {
        // The Err(JsValue) is parked in the externref table at `err_idx`; pull the engine's
        // own message out of it for a clear error report.
        let msg = take_error_message(&mut store, &instance, err_idx)
            .unwrap_or_else(|| "engine rejected the input".to_string());
        anyhow::bail!("engine error: {msg}");
    }

    let mut pdf = vec![0u8; len as usize];
    memory
        .read(&store, ptr as usize, &mut pdf)
        .map_err(|e| anyhow::anyhow!("reading PDF bytes from Engine memory: {e}"))?;
    // Free the wasm-side allocation (align 1, element size 1), matching the JS glue.
    free.call(&mut store, (ptr, len, 1))?;
    Ok(pdf)
}

/// Pull the string out of the error externref the Engine parked at table index `idx`
/// (its `Err(JsValue)`), then release the slot.
fn take_error_message(
    store: &mut Store<HostState>,
    instance: &wasmtime::Instance,
    idx: i32,
) -> Option<String> {
    let table = instance.get_table(&mut *store, "__wbindgen_externrefs")?;
    let msg = match table.get(&mut *store, idx as u64) {
        Some(Ref::Extern(Some(r))) => {
            let data: &(dyn Any + Send + Sync) = r.data(&*store).ok().flatten()?;
            match data.downcast_ref::<HostRef>() {
                Some(HostRef::Mem { text, .. }) => Some(text.clone()),
                _ => None,
            }
        }
        _ => None,
    };
    // Return the slot to the Engine allocator (matches the JS `takeFromExternrefTable0`).
    if let Some(dealloc) = instance.get_typed_func::<i32, ()>(&mut *store, "__externref_table_dealloc").ok() {
        let _ = dealloc.call(&mut *store, idx);
    }
    msg
}

/// Encode an `Option<f32>` the way wasm-bindgen expects it on the f64 wire slot.
fn opt_f32(v: Option<f32>) -> f64 {
    match v {
        Some(x) => x as f64,           // already f32-precise
        None => 4_294_967_297.0_f64,   // 0x1_0000_0001 sentinel
    }
}

fn pass_string(
    store: &mut Store<HostState>,
    malloc: &wasmtime::TypedFunc<(i32, i32), i32>,
    memory: &wasmtime::Memory,
    s: &str,
) -> anyhow::Result<(i32, i32)> {
    let bytes = s.as_bytes();
    let len = bytes.len() as i32;
    let ptr = malloc.call(&mut *store, (len, 1))?;
    memory
        .write(&mut *store, ptr as usize, bytes)
        .map_err(|e| anyhow::anyhow!("writing string to Engine memory: {e}"))?;
    Ok((ptr, len))
}

/// Read a UTF-8 (lossy) string out of Engine memory.
fn read_string(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> String {
    let mem = match caller.get_export("memory").and_then(Extern::into_memory) {
        Some(m) => m,
        None => return String::new(),
    };
    let mut buf = vec![0u8; len.max(0) as usize];
    if mem.read(&*caller, ptr as usize, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Strip wasm-bindgen's trailing `_<hash>` from an import name so we can match on the
/// stable semantic part (e.g. `__wbg_log_8cec76766b8c0e33` -> `__wbg_log`). The hash is a
/// run of lowercase hex digits (typically 16); names without one are returned unchanged
/// (e.g. `__wbindgen_init_externref_table`).
fn strip_bindgen_hash(name: &str) -> &str {
    if let Some(i) = name.rfind('_') {
        let suffix = &name[i + 1..];
        if suffix.len() >= 8
            && suffix.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return &name[..i];
        }
    }
    name
}

// --- import handlers (shared by whatever hashed names the Engine actually uses) ---

/// `(ptr, len) -> externref` cast intrinsic. See `HostRef::Mem`.
fn h_cast(mut caller: Caller<'_, HostState>, ptr: i32, len: i32) -> anyhow::Result<Option<Rooted<ExternRef>>> {
    let text = read_string(&mut caller, ptr, len);
    let r = ExternRef::new(&mut caller, HostRef::Mem { ptr: ptr as u32, len: len as u32, text })?;
    Ok(Some(r))
}

/// console.log / console.warn — print the cast string verbatim.
fn h_log(caller: Caller<'_, HostState>, arg: Option<Rooted<ExternRef>>) {
    if let Some(HostRef::Mem { text, .. }) = host_ref(&caller, arg) {
        eprintln!("{text}");
    }
}

/// `globalThis.crypto` -> a crypto object (so the browser-crypto path is taken).
fn h_crypto(mut caller: Caller<'_, HostState>, _g: Option<Rooted<ExternRef>>) -> anyhow::Result<Option<Rooted<ExternRef>>> {
    Ok(Some(ExternRef::new(&mut caller, HostRef::Crypto)?))
}

/// `globalThis.process` / `.versions` / `.node` / `.msCrypto` -> undefined, so getrandom
/// skips the Node.js path.
fn h_undefined1(mut caller: Caller<'_, HostState>, _a: Option<Rooted<ExternRef>>) -> anyhow::Result<Option<Rooted<ExternRef>>> {
    Ok(Some(ExternRef::new(&mut caller, HostRef::Undefined)?))
}

/// `static_accessor_*` -> table index of a fresh `globalThis` externref.
fn h_global(mut caller: Caller<'_, HostState>) -> anyhow::Result<i32> {
    let g = ExternRef::new(&mut caller, HostRef::Global)?;
    externref_table_add(&mut caller, Val::ExternRef(Some(g)))
}

/// `crypto.getRandomValues(view)` / Node `randomFillSync` — fill the buffer at `ptr..len`.
fn h_get_random(mut caller: Caller<'_, HostState>, _c: Option<Rooted<ExternRef>>, view: Option<Rooted<ExternRef>>) -> anyhow::Result<()> {
    let (ptr, len) = match host_ref(&caller, view) {
        Some(HostRef::Mem { ptr, len, .. }) => (ptr as usize, len as usize),
        _ => anyhow::bail!("getRandomValues called with a non-buffer argument"),
    };
    let mut buf = vec![0u8; len];
    {
        let st = caller.data_mut();
        let mut i = 0;
        while i < len {
            let r = st.next_u64().to_le_bytes();
            let n = (len - i).min(8);
            buf[i..i + n].copy_from_slice(&r[..n]);
            i += n;
        }
    }
    let mem = caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| anyhow::anyhow!("no memory export for getRandomValues"))?;
    mem.write(&mut caller, ptr, &buf)
        .map_err(|e| anyhow::anyhow!("getRandomValues write: {e}"))?;
    Ok(())
}

fn h_is_object(caller: Caller<'_, HostState>, arg: Option<Rooted<ExternRef>>) -> i32 {
    matches!(host_ref(&caller, arg), Some(HostRef::Global | HostRef::Crypto)) as i32
}
fn h_is_string(caller: Caller<'_, HostState>, arg: Option<Rooted<ExternRef>>) -> i32 {
    matches!(host_ref(&caller, arg), Some(HostRef::Mem { .. })) as i32
}
fn h_is_function(_c: Caller<'_, HostState>, _a: Option<Rooted<ExternRef>>) -> i32 { 0 }
fn h_is_undefined(caller: Caller<'_, HostState>, arg: Option<Rooted<ExternRef>>) -> i32 {
    matches!(host_ref(&caller, arg), None | Some(HostRef::Undefined)) as i32
}

fn h_throw(mut caller: Caller<'_, HostState>, ptr: i32, len: i32) -> anyhow::Result<()> {
    let s = read_string(&mut caller, ptr, len);
    anyhow::bail!("Engine threw: {s}")
}

/// `__wbindgen_init_externref_table` — grow the externref table by 4 and seed the canonical
/// [undefined, null, true, false] slots.
fn h_init_table(mut caller: Caller<'_, HostState>) -> anyhow::Result<()> {
    let table = caller
        .get_export("__wbindgen_externrefs")
        .and_then(Extern::into_table)
        .ok_or_else(|| anyhow::anyhow!("missing __wbindgen_externrefs table"))?;
    let undef = ExternRef::new(&mut caller, HostRef::Undefined)?;
    let offset = table.grow(&mut caller, 4, Ref::Extern(Some(undef)))?;
    let undef0 = ExternRef::new(&mut caller, HostRef::Undefined)?;
    table.set(&mut caller, 0, Ref::Extern(Some(undef0)))?;
    let null = ExternRef::new(&mut caller, HostRef::Null)?;
    table.set(&mut caller, offset + 1, Ref::Extern(Some(null)))?;
    let t = ExternRef::new(&mut caller, HostRef::Bool(true))?;
    table.set(&mut caller, offset + 2, Ref::Extern(Some(t)))?;
    let f = ExternRef::new(&mut caller, HostRef::Bool(false))?;
    table.set(&mut caller, offset + 3, Ref::Extern(Some(f)))?;
    Ok(())
}

/// Bind exactly the imports the synchronous `html_to_pdf` path uses, identifying each by
/// its hash-stripped name and type signature (so it adapts to whatever wasm-bindgen build
/// was downloaded). Everything else is trapped — those are only reached by the async /
/// fetch / DOM variants we never call.
fn define_imports(linker: &mut Linker<HostState>, module: &Module) -> anyhow::Result<()> {
    use wasmtime::{ExternType, ValType};

    // Signature predicate: chars are 'i'=i32, 'f'=f64, 'e'=externref.
    fn val_is(t: &ValType, k: char) -> bool {
        match k {
            'i' => matches!(t, ValType::I32),
            'f' => matches!(t, ValType::F64),
            'e' => matches!(t, ValType::Ref(r) if r.heap_type().is_extern()),
            _ => false,
        }
    }
    fn sig(ft: &wasmtime::FuncType, params: &str, results: &str) -> bool {
        ft.params().len() == params.len()
            && ft.results().len() == results.len()
            && ft.params().zip(params.chars()).all(|(t, k)| val_is(&t, k))
            && ft.results().zip(results.chars()).all(|(t, k)| val_is(&t, k))
    }

    for imp in module.imports() {
        let ft = match imp.ty() {
            ExternType::Func(ft) => ft,
            _ => continue,
        };
        let modname = imp.module().to_string();
        let name = imp.name().to_string();
        match strip_bindgen_hash(&name) {
            "__wbindgen_init_externref_table" if sig(&ft, "", "") => {
                linker.func_wrap(&modname, &name, h_init_table)?;
            }
            "__wbg_log" | "__wbg_warn" if sig(&ft, "e", "") => {
                linker.func_wrap(&modname, &name, h_log)?;
            }
            "__wbg_crypto" if sig(&ft, "e", "e") => {
                linker.func_wrap(&modname, &name, h_crypto)?;
            }
            "__wbg_process" | "__wbg_versions" | "__wbg_node" | "__wbg_msCrypto"
                if sig(&ft, "e", "e") =>
            {
                linker.func_wrap(&modname, &name, h_undefined1)?;
            }
            "__wbg_getRandomValues" | "__wbg_randomFillSync" if sig(&ft, "ee", "") => {
                linker.func_wrap(&modname, &name, h_get_random)?;
            }
            "__wbg___wbindgen_is_object" if sig(&ft, "e", "i") => {
                linker.func_wrap(&modname, &name, h_is_object)?;
            }
            "__wbg___wbindgen_is_string" if sig(&ft, "e", "i") => {
                linker.func_wrap(&modname, &name, h_is_string)?;
            }
            "__wbg___wbindgen_is_function" if sig(&ft, "e", "i") => {
                linker.func_wrap(&modname, &name, h_is_function)?;
            }
            "__wbg___wbindgen_is_undefined" if sig(&ft, "e", "i") => {
                linker.func_wrap(&modname, &name, h_is_undefined)?;
            }
            "__wbg___wbindgen_throw" if sig(&ft, "ii", "") => {
                linker.func_wrap(&modname, &name, h_throw)?;
            }
            "__wbindgen_cast" if sig(&ft, "ii", "e") => {
                linker.func_wrap(&modname, &name, h_cast)?;
            }
            b if b.starts_with("__wbg_static_accessor_") && sig(&ft, "", "i") => {
                linker.func_wrap(&modname, &name, h_global)?;
            }
            _ => {} // left for define_unknown_imports_as_traps below
        }
    }

    // Everything we didn't bind (fetch / Promise / Response / DOM accessors, only used by
    // the async variants) becomes a trap — if the sync path ever hit one we'd see it.
    linker.define_unknown_imports_as_traps(module)?;
    Ok(())
}

/// Mirror wasm-bindgen's `addToExternrefTable0`: allocate a slot via the module's
/// `__externref_table_alloc` export, store `val` there, and return the index.
fn externref_table_add(
    caller: &mut Caller<'_, HostState>,
    val: Val,
) -> anyhow::Result<i32> {
    let alloc = caller
        .get_export("__externref_table_alloc")
        .and_then(Extern::into_func)
        .ok_or_else(|| anyhow::anyhow!("missing __externref_table_alloc export"))?;
    let mut out = [Val::I32(0)];
    alloc.call(&mut *caller, &[], &mut out)?;
    let idx = out[0].i32().unwrap();
    let table = caller
        .get_export("__wbindgen_externrefs")
        .and_then(Extern::into_table)
        .ok_or_else(|| anyhow::anyhow!("missing __wbindgen_externrefs table"))?;
    let r = match val {
        Val::ExternRef(r) => r,
        _ => None,
    };
    table.set(&mut *caller, idx as u64, Ref::Extern(r))?;
    Ok(idx)
}
