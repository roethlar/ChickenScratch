//! Links between documents in compiled output (PLAN_SCRIVENER_UPLOAD slice
//! 3, "Compile resolves links between documents, from the parse it already
//! does"; review record `.agents/review/findings/plan-4.md`).
//!
//! Compile concatenates the manuscript documents and hands Pandoc one text.
//! A link from one document to another (`/manuscript/two.md#S1`, the form
//! the converter and the splitter write, or `two.md#second` relative to the
//! document that holds it) came out of Pandoc as a hyperlink to a relative
//! path that no DOCX or PDF reader can follow. This module works on Pandoc's
//! own parse of the combined text (its JSON) and turns such links into
//! internal destinations, changing nothing about what Pandoc parses:
//!
//! 1. the combined text is marked with begin/end comment blocks per
//!    document so each parsed block has a known owner (never before the
//!    first document when no title page precedes it);
//! 2. a second parse of the same text with automatic identifiers off tells
//!    explicit identifiers from generated ones;
//! 3. a link's target is found by explicit identifier in the target
//!    document's section, else by the block path of the generated identifier
//!    in a solo parse of that document, never inside footnote contents;
//! 4. every identifier is renamed per element, except duplicates that a
//!    bare `#id` link already relies on, which keep their string so each
//!    writer resolves them exactly as today;
//! 5. a destination every writer emits is placed where the target is, and
//!    the links are rewritten.
//!
//! Anything that cannot be resolved exactly becomes its text with a named
//! warning in the [`LinkReport`]; nothing is ever guessed.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// One compiled document, in compile order.
#[derive(Debug, Clone)]
pub struct SectionDoc {
    /// Project-relative path (`manuscript/two.md`).
    pub path: String,
    /// The folder import's record of where the file came from, when any.
    pub source_path: Option<String>,
    pub content: String,
}

/// Every document of the project, included in the compile or not.
#[derive(Debug, Clone)]
pub struct KnownDoc {
    pub path: String,
    pub source_path: Option<String>,
}

/// Warnings and the fact of resolution, for the caller to surface.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LinkReport {
    pub warnings: Vec<String>,
}

const BLOCK_KINDS_WITH_ATTR: &[&str] = &["Header", "Div", "CodeBlock", "Table", "Figure"];

// ── Markers ──────────────────────────────────────────────────────────────────

pub fn begin_marker(index: usize, nonce: &str) -> String {
    format!("<!-- chikn:begin {index} {nonce} -->")
}

pub fn end_marker(index: usize, nonce: &str) -> String {
    format!("<!-- chikn:end {index} {nonce} -->")
}

/// Which marker a top-level block is, if any.
fn marker_of(block: &Value, nonce: &str) -> Option<(bool, usize)> {
    if block["t"].as_str()? != "RawBlock" {
        return None;
    }
    let text = block["c"][1].as_str()?.trim();
    let inner = text.strip_prefix("<!-- chikn:")?.strip_suffix(" -->")?;
    let mut parts = inner.split(' ');
    let kind = parts.next()?;
    let index: usize = parts.next()?.parse().ok()?;
    if parts.next()? != nonce || parts.next().is_some() {
        return None;
    }
    match kind {
        "begin" => Some((true, index)),
        "end" => Some((false, index)),
        _ => None,
    }
}

/// A document's run of top-level blocks in the combined parse.
#[derive(Debug, Clone)]
struct Section {
    doc: usize,
    /// Half-open range of top-level block indices, markers excluded.
    start: usize,
    end: usize,
}

/// Cut the top-level blocks into sections by the markers. `None` when the
/// markers are missing or out of order (a document's unclosed fence or Div
/// swallowed the next one, which already garbles today's output).
fn sections(
    blocks: &[Value],
    doc_count: usize,
    first_has_begin: bool,
    nonce: &str,
) -> Option<Vec<Section>> {
    let mut out = Vec::with_capacity(doc_count);
    let mut open: Option<(usize, usize)> = None; // (doc, start index)
    let mut next_doc = 0;
    if !first_has_begin && doc_count > 0 {
        open = Some((0, 0));
        next_doc = 1;
    }
    for (i, block) in blocks.iter().enumerate() {
        match marker_of(block, nonce) {
            Some((true, index)) => {
                if open.is_some() || index != next_doc {
                    return None;
                }
                open = Some((index, i + 1));
                next_doc += 1;
            }
            Some((false, index)) => {
                let (doc, start) = open.take()?;
                if doc != index {
                    return None;
                }
                out.push(Section { doc, start, end: i });
            }
            None => {}
        }
    }
    if open.is_some() || out.len() != doc_count {
        return None;
    }
    Some(out)
}

// ── Paths into the JSON tree ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Step {
    Key(&'static str),
    Idx(usize),
    /// A key of the document's `meta` map (or a nested MetaMap), whose names
    /// are the writer's own.
    MetaKey(String),
}

fn get<'a>(root: &'a Value, path: &[Step]) -> Option<&'a Value> {
    let mut cur = root;
    for step in path {
        cur = match step {
            Step::Key(k) => cur.get(*k)?,
            Step::Idx(i) => cur.get(*i)?,
            Step::MetaKey(k) => cur.get(k.as_str())?,
        };
    }
    Some(cur)
}

fn get_mut<'a>(root: &'a mut Value, path: &[Step]) -> Option<&'a mut Value> {
    let mut cur = root;
    for step in path {
        cur = match step {
            Step::Key(k) => cur.get_mut(*k)?,
            Step::Idx(i) => cur.get_mut(*i)?,
            Step::MetaKey(k) => cur.get_mut(k.as_str())?,
        };
    }
    Some(cur)
}

/// Where an element's attribute triple sits relative to its node.
fn attr_path(kind: &str) -> Option<&'static [Step]> {
    const HEADER: &[Step] = &[Step::Key("c"), Step::Idx(1)];
    const FIRST: &[Step] = &[Step::Key("c"), Step::Idx(0)];
    match kind {
        "Header" => Some(HEADER),
        "Div" | "CodeBlock" | "Table" | "Figure" | "Span" | "Image" | "Link" | "Code" => {
            Some(FIRST)
        }
        _ => None,
    }
}

// ── Analysis ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Element {
    kind: String,
    id: String,
    explicit: bool,
    in_note: bool,
    /// Section the element belongs to; None for generated blocks.
    section: Option<usize>,
    /// Full path from the document root to the node.
    path: Vec<Step>,
    /// Index path of array positions from the section's first block down
    /// to the node (block paths compare across parses); None inside notes.
    block_path: Option<Vec<usize>>,
    /// The heading whose inlines hold this element, when any.
    inside_header: Option<Vec<Step>>,
}

#[derive(Debug, Clone)]
struct LinkNode {
    path: Vec<Step>,
    target: String,
    section: Option<usize>,
    in_note: bool,
}

struct Scan {
    elements: Vec<Element>,
    links: Vec<LinkNode>,
}

struct Ctx {
    section: Option<usize>,
    in_note: bool,
    inside_header: Option<Vec<Step>>,
}

/// Walk the combined parse `c` and its diagnostic twin `d` in lockstep.
fn scan(root_c: &Value, root_d: &Value, secs: &[Section], nonce: &str) -> Scan {
    let mut out = Scan {
        elements: Vec::new(),
        links: Vec::new(),
    };
    let blocks_c = root_c["blocks"].as_array().cloned().unwrap_or_default();
    let blocks_d = root_d["blocks"].as_array().cloned().unwrap_or_default();
    let mut owner: HashMap<usize, (usize, usize)> = HashMap::new();
    for s in secs {
        for i in s.start..s.end {
            owner.insert(i, (s.doc, s.start));
        }
    }
    for (i, (bc, bd)) in blocks_c.iter().zip(blocks_d.iter()).enumerate() {
        if marker_of(bc, nonce).is_some() {
            continue;
        }
        let (section, section_start) = match owner.get(&i) {
            Some((doc, start)) => (Some(*doc), *start),
            None => (None, 0),
        };
        let mut ctx = Ctx {
            section,
            in_note: false,
            inside_header: None,
        };
        let mut path = vec![Step::Key("blocks"), Step::Idx(i)];
        // Block paths count from the section's first block, so they compare
        // with the document's solo parse.
        let mut bpath = Some(vec![i - section_start]);
        visit(bc, bd, &mut path, &mut bpath, &mut ctx, &mut out);
    }
    // Metadata (a `%` title block, YAML) holds inlines too: an identifier
    // there takes part in the duplicate count and a document link there
    // fires the gate. Generated content, outside every section (review
    // finding s3-8).
    let mut meta_ctx = Ctx {
        section: None,
        in_note: false,
        inside_header: None,
    };
    let mut meta_path = vec![Step::Key("meta")];
    visit_meta_map(
        &root_c["meta"],
        &root_d["meta"],
        &mut meta_path,
        &mut meta_ctx,
        &mut out,
    );
    out
}

/// A map of metadata entries (the document's `meta`, or a `MetaMap`'s
/// content): every value is a MetaValue, whatever its key. The map is known
/// from its position, never guessed from its keys: a writer's own key named
/// `t` once made the walk mistake the map for a tagged node and skip the
/// rest of the metadata (review finding s3-8, repair review).
fn visit_meta_map(c: &Value, d: &Value, path: &mut Vec<Step>, ctx: &mut Ctx, out: &mut Scan) {
    let Some(map) = c.as_object() else { return };
    for (key, value) in map {
        let d_value = d.get(key).unwrap_or(&Value::Null);
        path.push(Step::MetaKey(key.clone()));
        visit(value, d_value, path, &mut None, ctx, out);
        path.pop();
    }
}

fn visit(
    c: &Value,
    d: &Value,
    path: &mut Vec<Step>,
    bpath: &mut Option<Vec<usize>>,
    ctx: &mut Ctx,
    out: &mut Scan,
) {
    match c {
        Value::Array(items) => {
            let d_items = d.as_array();
            for (i, item) in items.iter().enumerate() {
                let d_item = d_items.and_then(|a| a.get(i)).unwrap_or(&Value::Null);
                path.push(Step::Idx(i));
                let mut child_bpath = bpath.as_ref().map(|b| {
                    let mut b = b.clone();
                    b.push(i);
                    b
                });
                visit(item, d_item, path, &mut child_bpath, ctx, out);
                path.pop();
            }
        }
        Value::Object(map) => {
            let kind = map.get("t").and_then(Value::as_str).unwrap_or("");
            if kind == "MetaMap" {
                path.push(Step::Key("c"));
                visit_meta_map(&c["c"], &d["c"], path, ctx, out);
                path.pop();
                return;
            }
            if let Some(ap) = attr_path(kind) {
                let id = get(c, ap)
                    .and_then(|a| a[0].as_str())
                    .unwrap_or("")
                    .to_string();
                let d_id = get(d, ap)
                    .and_then(|a| a[0].as_str())
                    .unwrap_or("")
                    .to_string();
                if !id.is_empty() {
                    out.elements.push(Element {
                        kind: kind.to_string(),
                        id,
                        explicit: !d_id.is_empty(),
                        in_note: ctx.in_note,
                        section: ctx.section,
                        path: path.clone(),
                        block_path: if ctx.in_note || !BLOCK_KINDS_WITH_ATTR.contains(&kind) {
                            None
                        } else {
                            bpath.clone()
                        },
                        inside_header: ctx.inside_header.clone(),
                    });
                }
                if kind == "Link" {
                    if let Some(target) = c["c"][2][0].as_str() {
                        out.links.push(LinkNode {
                            path: path.clone(),
                            target: target.to_string(),
                            section: ctx.section,
                            in_note: ctx.in_note,
                        });
                    }
                }
            }
            if kind == "Note" {
                let was = ctx.in_note;
                ctx.in_note = true;
                path.push(Step::Key("c"));
                visit(&c["c"], &d["c"], path, &mut None, ctx, out);
                path.pop();
                ctx.in_note = was;
                return;
            }
            if kind == "Header" {
                let was = ctx.inside_header.take();
                ctx.inside_header = Some(path.clone());
                path.push(Step::Key("c"));
                visit(&c["c"], &d["c"], path, &mut None, ctx, out);
                path.pop();
                ctx.inside_header = was;
                return;
            }
            if let Some(content) = map.get("c") {
                path.push(Step::Key("c"));
                // Only block-list positions extend a block path; inline
                // content of a block ends it.
                let mut inner = if BLOCK_KINDS_WITH_ATTR.contains(&kind)
                    || matches!(
                        kind,
                        "BlockQuote"
                            | "BulletList"
                            | "OrderedList"
                            | "DefinitionList"
                            | "LineBlock"
                    ) {
                    bpath.clone()
                } else {
                    None
                };
                visit(
                    content,
                    d.get("c").unwrap_or(&Value::Null),
                    path,
                    &mut inner,
                    ctx,
                    out,
                );
                path.pop();
            }
        }
        _ => {}
    }
}

// ── Targets ──────────────────────────────────────────────────────────────────

/// Normalise a slash path: collapse `.` and `..`, refuse escaping above the
/// root (returns None), strip a leading `./`.
fn normalise(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes.get(i + 1).copied()).and_then(hex),
                (bytes.get(i + 2).copied()).and_then(hex),
            ) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn dirname(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

fn has_scheme(target: &str) -> bool {
    let mut chars = target.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-')) {
            return false;
        }
    }
    false
}

/// Resolve `target` (path part, no fragment) from the folder of `origin`,
/// in source space when the origin carries a source path, else in project
/// space. Returns the project path of the document hit, if any.
fn resolve_from(origin: &KnownDoc, target: &str, docs: &[KnownDoc]) -> Option<String> {
    match &origin.source_path {
        Some(source) => {
            let joined = normalise(&format!("{}/{}", dirname(source), target))?;
            docs.iter()
                .find(|d| d.source_path.as_deref() == Some(joined.as_str()))
                .map(|d| d.path.clone())
        }
        None => {
            let joined = normalise(&format!("{}/{}", dirname(&origin.path), target))?;
            docs.iter()
                .find(|d| d.path == joined)
                .map(|d| d.path.clone())
        }
    }
}

fn resolve_root(target: &str, docs: &[KnownDoc]) -> Option<String> {
    let joined = normalise(target)?;
    docs.iter()
        .find(|d| d.path == joined)
        .map(|d| d.path.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TargetKind {
    /// Not a document link: a scheme, a non-document path, or unresolvable.
    Other,
    /// A bare `#id` link.
    Bare(String),
    /// A document link: project path and optional fragment.
    Document {
        path: String,
        fragment: Option<String>,
    },
    /// A document link that could not be read unambiguously (text + warning).
    Refused(String),
}

fn classify(link: &LinkNode, origin: Option<&KnownDoc>, docs: &[KnownDoc]) -> TargetKind {
    let target = link.target.as_str();
    if target.is_empty() || has_scheme(target) || target.starts_with("//") {
        return TargetKind::Other;
    }
    // The editor writes a non-ASCII fragment percent-encoded
    // (`b.md#caf%C3%A9`); Pandoc's identifier is `café`, so fragments are
    // decoded like paths before any lookup (review finding s3-1).
    if let Some(frag) = target.strip_prefix('#') {
        return TargetKind::Bare(percent_decode(frag));
    }
    let (path_part, fragment) = match target.find('#') {
        Some(i) => (&target[..i], Some(percent_decode(&target[i + 1..]))),
        None => (target, None),
    };
    let decoded = percent_decode(path_part);
    if let Some(root_rel) = decoded.strip_prefix('/') {
        return match resolve_root(root_rel, docs) {
            Some(path) => TargetKind::Document { path, fragment },
            None => TargetKind::Other,
        };
    }
    if link.in_note {
        // Every document is a possible origin. A target no origin resolves
        // to a document is not a document link at all (an attachment, a
        // relative resource) and stays untouched (review finding s3-4);
        // one that some origins resolve must be resolved by all of them to
        // the same document, else it is refused (plan-4 R18-1).
        let hits: Vec<Option<String>> = docs
            .iter()
            .map(|origin| resolve_from(origin, &decoded, docs))
            .collect();
        if hits.iter().all(|hit| hit.is_none()) {
            return TargetKind::Other;
        }
        if let Some(path) = hits.first().cloned().flatten() {
            if hits.iter().all(|hit| hit.as_deref() == Some(path.as_str())) {
                return TargetKind::Document { path, fragment };
            }
        }
        return TargetKind::Refused(format!(
            "footnote link {target} could be read from more than one document; anchor it as /<path>"
        ));
    }
    let Some(origin) = origin else {
        return TargetKind::Other;
    };
    let relative = resolve_from(origin, &decoded, docs);
    let root = resolve_root(&decoded, docs);
    match (relative, root) {
        (Some(path), _) => TargetKind::Document { path, fragment },
        (None, Some(path)) => TargetKind::Document { path, fragment },
        (None, None) => TargetKind::Other,
    }
}

/// A rootless target that reads as a document both relative to its own
/// document (the reading that won) and from the project root (the form
/// projects converted before this plan carry) names two different
/// documents: the writer is told, so the link can be anchored (plan-4
/// R16-2, review finding s3-10).
fn legacy_root_ambiguity(link: &LinkNode, resolved: &str, docs: &[KnownDoc]) -> Option<String> {
    let target = link.target.as_str();
    if link.in_note || target.starts_with('/') || target.starts_with('#') || has_scheme(target) {
        return None;
    }
    let path_part = target.split('#').next().unwrap_or(target);
    let root = resolve_root(&percent_decode(path_part), docs)?;
    if root == resolved {
        return None;
    }
    Some(format!(
        "{target} was read relative to its document as {resolved}, but also names {root} from the project root; anchor it as /{resolved} or /{root}"
    ))
}

// ── Public entry: does the combined parse hold document links? ──────────────

/// The gate (step 1): true when any link in the marked parse is a document
/// link. `secs` must come from [`sections`]; a `None` there means the
/// markers were swallowed and the caller falls back to today's path.
pub struct Analysis {
    root: Value,
    diagnostic: Option<Value>,
    secs: Vec<Section>,
    docs: Vec<KnownDoc>,
    sections: Vec<SectionDoc>,
    nonce: String,
}

/// Outcome of looking at the marked parse.
pub enum Gate {
    /// No link between documents: compile proceeds exactly as today.
    NoDocumentLinks,
    /// The markers were swallowed by an unclosed fence or Div.
    MarkersBroken { warning: String },
    /// Document links exist; resolution proceeds with this analysis.
    Resolve(Analysis),
}

pub fn gate(
    marked_json: Value,
    sections_in: Vec<SectionDoc>,
    all_docs: Vec<KnownDoc>,
    first_has_begin: bool,
    nonce: &str,
) -> Gate {
    let blocks = marked_json["blocks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let Some(secs) = sections(&blocks, sections_in.len(), first_has_begin, nonce) else {
        return Gate::MarkersBroken {
            warning: "a document runs into the next document (an unclosed code fence or block); links between documents were not resolved".to_string(),
        };
    };
    // A quick pass over the links with the combined parse alone (the
    // diagnostic parse is only needed once we know we resolve).
    let scanned = scan(&marked_json, &marked_json, &secs, nonce);
    let any = scanned.links.iter().any(|l| {
        let origin = l.section.map(|s| KnownDoc {
            path: sections_in[s].path.clone(),
            source_path: sections_in[s].source_path.clone(),
        });
        matches!(
            classify(l, origin.as_ref(), &all_docs),
            TargetKind::Document { .. } | TargetKind::Refused(_)
        )
    });
    if !any {
        return Gate::NoDocumentLinks;
    }
    Gate::Resolve(Analysis {
        root: marked_json,
        diagnostic: None,
        secs,
        docs: all_docs,
        sections: sections_in,
        nonce: nonce.to_string(),
    })
}

impl Analysis {
    /// The diagnostic parse (`-auto_identifiers`) of the same marked text.
    pub fn with_diagnostic(mut self, diagnostic_json: Value) -> Self {
        self.diagnostic = Some(diagnostic_json);
        self
    }

    /// Documents whose generated identifiers a link needs: their indices,
    /// for the caller to parse solo (at most a few at a time).
    pub fn documents_needing_solo_parse(&self) -> Vec<usize> {
        let diag = self.diagnostic.as_ref().unwrap_or(&self.root);
        let scanned = scan(&self.root, diag, &self.secs, &self.nonce);
        let mut needed = BTreeSet::new();
        for link in &scanned.links {
            let origin = link.section.map(|s| self.known(s));
            if let TargetKind::Document {
                path,
                fragment: Some(frag),
            } = classify(link, origin.as_ref(), &self.docs)
            {
                if let Some(sec) = self.section_of_path(&path) {
                    let explicit_hit = scanned.elements.iter().any(|e| {
                        e.section == Some(sec) && !e.in_note && e.explicit && e.id == frag
                    });
                    if !explicit_hit {
                        needed.insert(sec);
                    }
                }
            }
        }
        needed.into_iter().collect()
    }

    fn known(&self, section: usize) -> KnownDoc {
        KnownDoc {
            path: self.sections[section].path.clone(),
            source_path: self.sections[section].source_path.clone(),
        }
    }

    fn section_of_path(&self, path: &str) -> Option<usize> {
        self.sections.iter().position(|s| s.path == path)
    }

    /// Resolve, rename, materialise and rewrite. `solo` maps a section index
    /// to that document's solo parse (only the ones asked for). Returns the
    /// rewritten document (markers removed) and the warnings.
    pub fn resolve(mut self, solo: &HashMap<usize, Value>) -> (Value, LinkReport) {
        let mut report = LinkReport::default();
        let diag = self.diagnostic.clone().unwrap_or_else(|| self.root.clone());
        let scanned = scan(&self.root, &diag, &self.secs, &self.nonce);

        // ── Step 4: which element each link means ────────────────────────
        #[derive(Debug)]
        enum Resolved {
            Element(usize),      // index into scanned.elements
            SectionStart(usize), // whole-document link
            Text,                // becomes its text
            Leave,               // untouched (bare duplicate, other)
            BareUnique(usize),   // bare link to a unique element
        }
        let mut id_count: HashMap<&str, usize> = HashMap::new();
        for e in &scanned.elements {
            *id_count.entry(e.id.as_str()).or_insert(0) += 1;
        }
        // A bare fragment names an element either as written or decoded: the
        // editor percent-encodes `#café`, while a writer's quoted identifier
        // may itself contain a literal `%` (s3-1 repair reviews).
        let bare_targets: HashSet<String> = scanned
            .links
            .iter()
            .filter_map(|l| l.target.strip_prefix('#'))
            .flat_map(|frag| [frag.to_string(), percent_decode(frag)])
            .collect();

        // Identifiers a bare link names by more than one spelling at once
        // (one `{#café}`, one `{id="caf%C3%A9"}`): the link is left as
        // written, so every candidate must keep its string (s3-1 repair
        // review 3).
        let mut ambiguous_bare: HashSet<String> = HashSet::new();
        let mut resolutions: Vec<Resolved> = Vec::with_capacity(scanned.links.len());
        for link in &scanned.links {
            let origin = link.section.map(|s| self.known(s));
            let resolved = match classify(link, origin.as_ref(), &self.docs) {
                TargetKind::Other => Resolved::Leave,
                TargetKind::Refused(w) => {
                    report.warnings.push(w);
                    Resolved::Text
                }
                TargetKind::Bare(id) => {
                    let raw = link.target.strip_prefix('#').unwrap_or("");
                    let matches: Vec<usize> = scanned
                        .elements
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.id == id || e.id == raw)
                        .map(|(i, _)| i)
                        .collect();
                    if matches.len() == 1 {
                        Resolved::BareUnique(matches[0])
                    } else {
                        for &m in &matches {
                            ambiguous_bare.insert(scanned.elements[m].id.clone());
                        }
                        Resolved::Leave
                    }
                }
                TargetKind::Document { path, fragment } => {
                    if let Some(warning) = legacy_root_ambiguity(link, &path, &self.docs) {
                        report.warnings.push(warning);
                    }
                    let Some(sec) = self.section_of_path(&path) else {
                        // A document outside the compile: its text alone.
                        resolutions.push(Resolved::Text);
                        continue;
                    };
                    match fragment {
                        None => Resolved::SectionStart(sec),
                        Some(frag) => {
                            let explicit: Vec<usize> = scanned
                                .elements
                                .iter()
                                .enumerate()
                                .filter(|(_, e)| {
                                    e.section == Some(sec)
                                        && !e.in_note
                                        && e.explicit
                                        && e.id == frag
                                })
                                .map(|(i, _)| i)
                                .collect();
                            if explicit.len() == 1 {
                                Resolved::Element(explicit[0])
                            } else if explicit.len() > 1 {
                                report.warnings.push(format!(
                                    "{} is a duplicate explicit identifier in {}; the link {} became text",
                                    frag, path, link.target
                                ));
                                Resolved::Text
                            } else if let Some(idx) = self.by_block_path(&scanned, sec, &frag, solo)
                            {
                                Resolved::Element(idx)
                            } else if scanned
                                .elements
                                .iter()
                                .any(|e| e.section == Some(sec) && e.in_note && e.id == frag)
                            {
                                report.warnings.push(format!(
                                    "{} points into a footnote; footnotes cannot be linked to from another document",
                                    link.target
                                ));
                                Resolved::Text
                            } else {
                                report.warnings.push(format!(
                                    "{} was not found in {}; the link became text",
                                    link.target, path
                                ));
                                Resolved::Text
                            }
                        }
                    }
                }
            };
            resolutions.push(resolved);
        }

        // ── Step 5: identifiers ──────────────────────────────────────────
        // Kept: duplicated strings that a bare link relies on, and every
        // candidate of a bare link that matched by more than one spelling.
        let kept: HashSet<String> = scanned
            .elements
            .iter()
            .filter(|e| {
                (id_count.get(e.id.as_str()).copied().unwrap_or(0) >= 2
                    && bare_targets.contains(&e.id))
                    || ambiguous_bare.contains(&e.id)
            })
            .map(|e| e.id.clone())
            .collect();
        let mut in_use: HashSet<String> = kept.clone();
        let mut counters: HashMap<String, usize> = HashMap::new();
        let mut allocate = |prefix: &str, in_use: &mut HashSet<String>| -> String {
            let counter = counters.entry(prefix.to_string()).or_insert(0);
            loop {
                let candidate = format!("{prefix}_{}", *counter);
                *counter += 1;
                if !in_use.contains(&candidate) {
                    in_use.insert(candidate.clone());
                    return candidate;
                }
            }
        };
        let prefix_for = |section: Option<usize>| -> String {
            match section {
                Some(s) => format!("d{s}"),
                None => "g0".to_string(),
            }
        };
        let mut final_id: Vec<Option<String>> = vec![None; scanned.elements.len()];
        for (i, e) in scanned.elements.iter().enumerate() {
            if kept.contains(&e.id) {
                continue; // keeps its original string
            }
            final_id[i] = Some(allocate(&prefix_for(e.section), &mut in_use));
        }

        // ── Step 6: destinations ─────────────────────────────────────────
        // Insertions are collected and applied last, deepest/highest index
        // first, so earlier paths stay valid.
        struct Insert {
            parent: Vec<Step>, // path to the array
            index: usize,
            node: Value,
        }
        let mut inserts: Vec<Insert> = Vec::new();
        let mut set_ids: Vec<(Vec<Step>, String)> = Vec::new();
        // The id a resolved link will point at.
        let mut link_dest: Vec<Option<String>> = vec![None; scanned.links.len()];
        let mut section_dest: HashMap<usize, String> = HashMap::new();
        let mut element_dest: HashMap<usize, String> = HashMap::new();

        let header_own_or_alias =
            |idx: usize,
             scanned: &Scan,
             final_id: &[Option<String>],
             inserts: &mut Vec<Insert>,
             in_use: &mut HashSet<String>,
             allocate: &mut dyn FnMut(&str, &mut HashSet<String>) -> String|
             -> String {
                let e = &scanned.elements[idx];
                match &final_id[idx] {
                    Some(id) => id.clone(),
                    None => {
                        // Kept heading: alias as Plain+Span right after it.
                        let alias = allocate(&prefix_for(e.section), in_use);
                        let (parent, index) = split_last(&e.path);
                        inserts.push(Insert {
                            parent,
                            index: index + 1,
                            node: plain_span(&alias),
                        });
                        alias
                    }
                }
            };

        for (li, res) in resolutions.iter().enumerate() {
            match res {
                Resolved::Element(idx) => {
                    if let Some(d) = element_dest.get(idx) {
                        link_dest[li] = Some(d.clone());
                        continue;
                    }
                    let e = &scanned.elements[*idx];
                    let dest = if let Some(header_path) = &e.inside_header {
                        // Inside a heading: the heading's own destination.
                        let hidx = scanned
                            .elements
                            .iter()
                            .position(|h| &h.path == header_path)
                            .expect("heading of an inline element is an element");
                        header_own_or_alias(
                            hidx,
                            &scanned,
                            &final_id,
                            &mut inserts,
                            &mut in_use,
                            &mut allocate,
                        )
                    } else {
                        match e.kind.as_str() {
                            "Header" => header_own_or_alias(
                                *idx,
                                &scanned,
                                &final_id,
                                &mut inserts,
                                &mut in_use,
                                &mut allocate,
                            ),
                            "Span" => match &final_id[*idx] {
                                Some(id) => id.clone(),
                                None => {
                                    let alias = allocate(&prefix_for(e.section), &mut in_use);
                                    let (parent, index) = split_last(&e.path);
                                    inserts.push(Insert {
                                        parent,
                                        index,
                                        node: span(&alias),
                                    });
                                    alias
                                }
                            },
                            "Image" | "Link" | "Code" => {
                                let alias = allocate(&prefix_for(e.section), &mut in_use);
                                let (parent, index) = split_last(&e.path);
                                inserts.push(Insert {
                                    parent,
                                    index,
                                    node: span(&alias),
                                });
                                alias
                            }
                            "Div" => {
                                let alias = allocate(&prefix_for(e.section), &mut in_use);
                                let node = get(&self.root, &e.path).cloned().unwrap_or(Value::Null);
                                let blocks_path: Vec<Step> = e
                                    .path
                                    .iter()
                                    .cloned()
                                    .chain([Step::Key("c"), Step::Idx(1)])
                                    .collect();
                                match node["c"][1].as_array().and_then(|b| b.first()) {
                                    Some(first) if first["t"] == "Header" => {
                                        // Heading-first Div: the heading's own destination.
                                        let hpath: Vec<Step> = blocks_path
                                            .iter()
                                            .cloned()
                                            .chain([Step::Idx(0)])
                                            .collect();
                                        let hidx =
                                            scanned.elements.iter().position(|h| h.path == hpath);
                                        match hidx {
                                            Some(h) => header_own_or_alias(
                                                h,
                                                &scanned,
                                                &final_id,
                                                &mut inserts,
                                                &mut in_use,
                                                &mut allocate,
                                            ),
                                            None => {
                                                // Heading without an identifier: give it the alias.
                                                set_ids.push((hpath, alias.clone()));
                                                alias
                                            }
                                        }
                                    }
                                    Some(first) if has_direct_inlines(first) => {
                                        let inl: Vec<Step> = blocks_path
                                            .iter()
                                            .cloned()
                                            .chain([Step::Idx(0), Step::Key("c")])
                                            .collect();
                                        inserts.push(Insert {
                                            parent: inl,
                                            index: 0,
                                            node: span(&alias),
                                        });
                                        alias
                                    }
                                    _ => {
                                        inserts.push(Insert {
                                            parent: blocks_path,
                                            index: 0,
                                            node: plain_span(&alias),
                                        });
                                        alias
                                    }
                                }
                            }
                            _ => {
                                // CodeBlock, Table, Figure, RawBlock…: a Plain+Span before it.
                                let alias = allocate(&prefix_for(e.section), &mut in_use);
                                let (parent, index) = split_last(&e.path);
                                inserts.push(Insert {
                                    parent,
                                    index,
                                    node: plain_span(&alias),
                                });
                                alias
                            }
                        }
                    };
                    element_dest.insert(*idx, dest.clone());
                    link_dest[li] = Some(dest);
                }
                Resolved::SectionStart(sec) => {
                    if let Some(d) = section_dest.get(sec) {
                        link_dest[li] = Some(d.clone());
                        continue;
                    }
                    let s = &self.secs[*sec];
                    let blocks = self.root["blocks"].as_array().cloned().unwrap_or_default();
                    let dest = if s.start == s.end {
                        let alias = allocate(&format!("d{sec}"), &mut in_use);
                        inserts.push(Insert {
                            parent: vec![Step::Key("blocks")],
                            index: s.start,
                            node: plain_span(&alias),
                        });
                        alias
                    } else {
                        let first = &blocks[s.start];
                        let first_path = vec![Step::Key("blocks"), Step::Idx(s.start)];
                        if first["t"] == "Header" {
                            match scanned.elements.iter().position(|h| h.path == first_path) {
                                Some(h) => header_own_or_alias(
                                    h,
                                    &scanned,
                                    &final_id,
                                    &mut inserts,
                                    &mut in_use,
                                    &mut allocate,
                                ),
                                None => {
                                    let alias = allocate(&format!("d{sec}"), &mut in_use);
                                    set_ids.push((first_path, alias.clone()));
                                    alias
                                }
                            }
                        } else if has_direct_inlines(first) {
                            let alias = allocate(&format!("d{sec}"), &mut in_use);
                            inserts.push(Insert {
                                parent: vec![
                                    Step::Key("blocks"),
                                    Step::Idx(s.start),
                                    Step::Key("c"),
                                ],
                                index: 0,
                                node: span(&alias),
                            });
                            alias
                        } else {
                            let alias = allocate(&format!("d{sec}"), &mut in_use);
                            inserts.push(Insert {
                                parent: vec![Step::Key("blocks")],
                                index: s.start,
                                node: plain_span(&alias),
                            });
                            alias
                        }
                    };
                    section_dest.insert(*sec, dest.clone());
                    link_dest[li] = Some(dest);
                }
                Resolved::BareUnique(idx) => {
                    // The element's final name, renamed or kept: a link that
                    // reached a kept element by another spelling of its
                    // identifier (`#caf%25C3%25A9` to `{id="caf%C3%A9"}`) must
                    // still be rewritten to it (s3-1 repair review 4).
                    let dest = final_id[*idx]
                        .clone()
                        .unwrap_or_else(|| scanned.elements[*idx].id.clone());
                    link_dest[li] = Some(dest);
                }
                Resolved::Text | Resolved::Leave => {}
            }
        }

        // ── Apply: identifiers, links, insertions, marker removal ────────
        for (i, e) in scanned.elements.iter().enumerate() {
            if let Some(id) = &final_id[i] {
                if let Some(ap) = attr_path(&e.kind) {
                    let mut p = e.path.clone();
                    p.extend(ap.iter().cloned());
                    p.push(Step::Idx(0));
                    if let Some(slot) = get_mut(&mut self.root, &p) {
                        *slot = Value::String(id.clone());
                    }
                }
            }
        }
        for (path, id) in set_ids {
            let mut p = path.clone();
            p.extend([Step::Key("c"), Step::Idx(1), Step::Idx(0)]);
            if let Some(slot) = get_mut(&mut self.root, &p) {
                *slot = Value::String(id);
            }
        }
        for (li, link) in scanned.links.iter().enumerate() {
            match (&resolutions[li], &link_dest[li]) {
                (Resolved::Text, _) => {
                    // The link becomes its inlines: a Span holding them, with
                    // the Link's own attributes kept (its identifier, already
                    // renamed above, may be the destination of another link,
                    // review finding s3-7).
                    if let Some(node) = get_mut(&mut self.root, &link.path) {
                        let attr = node["c"][0].clone();
                        let inlines = node["c"][1].clone();
                        *node = json!({"t": "Span", "c": [attr, inlines]});
                    }
                }
                (_, Some(dest)) => {
                    let mut p = link.path.clone();
                    p.extend([Step::Key("c"), Step::Idx(2), Step::Idx(0)]);
                    if let Some(slot) = get_mut(&mut self.root, &p) {
                        *slot = Value::String(format!("#{dest}"));
                    }
                }
                _ => {}
            }
        }
        // Insertions: group by parent, apply highest index first.
        inserts.sort_by(|a, b| {
            b.parent
                .len()
                .cmp(&a.parent.len())
                .then(b.index.cmp(&a.index))
        });
        let mut by_parent: BTreeMap<Vec<Step>, Vec<(usize, Value)>> = BTreeMap::new();
        for ins in inserts {
            by_parent
                .entry(ins.parent)
                .or_default()
                .push((ins.index, ins.node));
        }
        // Deeper parents first so shallower insertions do not shift them.
        let mut parents: Vec<Vec<Step>> = by_parent.keys().cloned().collect();
        parents.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| b.cmp(a)));
        for parent in parents {
            let mut items = by_parent.remove(&parent).unwrap_or_default();
            items.sort_by_key(|item| std::cmp::Reverse(item.0));
            if let Some(Value::Array(arr)) = get_mut(&mut self.root, &parent) {
                for (index, node) in items {
                    let at = index.min(arr.len());
                    arr.insert(at, node);
                }
            }
        }
        // Markers out.
        if let Some(Value::Array(blocks)) = self.root.get_mut("blocks") {
            blocks.retain(|b| marker_of(b, &self.nonce).is_none());
        }
        (self.root, report)
    }

    /// Rule (b): the element in section `sec` at the block path of the solo
    /// parse's element carrying generated identifier `frag`, of the same kind.
    fn by_block_path(
        &self,
        scanned: &Scan,
        sec: usize,
        frag: &str,
        solo: &HashMap<usize, Value>,
    ) -> Option<usize> {
        let solo_doc = solo.get(&sec)?;
        let solo_blocks = solo_doc["blocks"].as_array()?;
        let mut found: Option<(Vec<usize>, String)> = None;
        for (i, block) in solo_blocks.iter().enumerate() {
            find_generated(block, frag, &mut vec![i], &mut found);
            if found.is_some() {
                break;
            }
        }
        let (bpath, kind) = found?;
        scanned.elements.iter().position(|e| {
            e.section == Some(sec)
                && !e.in_note
                && e.kind == kind
                && e.block_path.as_deref() == Some(bpath.as_slice())
        })
    }
}

/// Depth-first search for a block-level element with identifier `frag`,
/// never entering notes; records its index path and kind.
fn find_generated(
    v: &Value,
    frag: &str,
    path: &mut Vec<usize>,
    found: &mut Option<(Vec<usize>, String)>,
) {
    if found.is_some() {
        return;
    }
    match v {
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                path.push(i);
                find_generated(item, frag, path, found);
                path.pop();
                if found.is_some() {
                    return;
                }
            }
        }
        Value::Object(map) => {
            let kind = map.get("t").and_then(Value::as_str).unwrap_or("");
            if kind == "Note" {
                return;
            }
            if BLOCK_KINDS_WITH_ATTR.contains(&kind) {
                if let Some(ap) = attr_path(kind) {
                    if get(v, ap).and_then(|a| a[0].as_str()) == Some(frag) {
                        *found = Some((path.clone(), kind.to_string()));
                        return;
                    }
                }
            }
            // Descend only through block-list positions (inline content
            // never holds block-level targets).
            if BLOCK_KINDS_WITH_ATTR.contains(&kind)
                || matches!(
                    kind,
                    "BlockQuote" | "BulletList" | "OrderedList" | "DefinitionList" | "LineBlock"
                )
            {
                if let Some(c) = map.get("c") {
                    find_generated(c, frag, path, found);
                }
            }
        }
        _ => {}
    }
}

fn split_last(path: &[Step]) -> (Vec<Step>, usize) {
    let (last, parent) = path.split_last().expect("a node path is never empty");
    let index = match last {
        Step::Idx(i) => *i,
        Step::Key(_) | Step::MetaKey(_) => 0,
    };
    (parent.to_vec(), index)
}

fn has_direct_inlines(block: &Value) -> bool {
    matches!(
        block["t"].as_str(),
        Some("Para") | Some("Plain") | Some("Header")
    )
}

fn span(id: &str) -> Value {
    json!({"t": "Span", "c": [[id, [], []], []]})
}

fn plain_span(id: &str) -> Value {
    json!({"t": "Plain", "c": [span(id)]})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_collapses_and_refuses_escape() {
        assert_eq!(
            normalise("manuscript/./two.md"),
            Some("manuscript/two.md".into())
        );
        assert_eq!(
            normalise("manuscript/part/../two.md"),
            Some("manuscript/two.md".into())
        );
        assert_eq!(normalise("../etc/passwd.md"), None);
        assert_eq!(normalise("./a.md"), Some("a.md".into()));
    }

    #[test]
    fn schemes_and_root_anchors_are_told_apart() {
        assert!(has_scheme("https://example.com"));
        assert!(has_scheme("mailto:x@y"));
        assert!(!has_scheme("/manuscript/two.md"));
        assert!(!has_scheme("two.md#x"));
        assert!(!has_scheme("c/d:e")); // a slash before the colon
    }

    #[test]
    fn percent_decoding_handles_spaces_and_junk() {
        assert_eq!(percent_decode("two%20words.md"), "two words.md");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%2x"), "a%2x");
    }

    fn docs() -> Vec<KnownDoc> {
        vec![
            KnownDoc {
                path: "manuscript/one.md".into(),
                source_path: None,
            },
            KnownDoc {
                path: "manuscript/two.md".into(),
                source_path: None,
            },
            KnownDoc {
                path: "manuscript/part/ch.md".into(),
                source_path: None,
            },
            KnownDoc {
                path: "research/x.md".into(),
                source_path: None,
            },
        ]
    }

    fn link(target: &str, section: Option<usize>, in_note: bool) -> LinkNode {
        LinkNode {
            path: vec![],
            target: target.into(),
            section,
            in_note,
        }
    }

    #[test]
    fn relative_root_and_bare_targets_classify() {
        let d = docs();
        let one = &d[0];
        let ch = &d[2];
        assert_eq!(
            classify(&link("two.md#second", Some(0), false), Some(one), &d),
            TargetKind::Document {
                path: "manuscript/two.md".into(),
                fragment: Some("second".into())
            }
        );
        // One level up from manuscript/part/ is manuscript/, not the root.
        assert_eq!(
            classify(&link("../research/x.md", Some(2), false), Some(ch), &d),
            TargetKind::Other
        );
        assert_eq!(
            classify(&link("../../research/x.md", Some(2), false), Some(ch), &d),
            TargetKind::Document {
                path: "research/x.md".into(),
                fragment: None
            }
        );
        assert_eq!(
            classify(&link("/manuscript/two.md#S1", Some(2), false), Some(ch), &d),
            TargetKind::Document {
                path: "manuscript/two.md".into(),
                fragment: Some("S1".into())
            }
        );
        // Legacy rootless project path, tried from the root when relative fails.
        assert_eq!(
            classify(&link("manuscript/two.md", Some(2), false), Some(ch), &d),
            TargetKind::Document {
                path: "manuscript/two.md".into(),
                fragment: None
            }
        );
        assert_eq!(
            classify(&link("research/notes.pdf", Some(0), false), Some(one), &d),
            TargetKind::Other
        );
        assert_eq!(
            classify(
                &link("https://example.com/two.md", Some(0), false),
                Some(one),
                &d
            ),
            TargetKind::Other
        );
        assert_eq!(
            classify(&link("#introduction", Some(0), false), Some(one), &d),
            TargetKind::Bare("introduction".into())
        );
        assert_eq!(
            classify(&link("../../etc/passwd.md", Some(0), false), Some(one), &d),
            TargetKind::Other
        );
    }

    #[test]
    fn source_space_resolution_uses_source_paths() {
        let d = vec![
            KnownDoc {
                path: "manuscript/one.md".into(),
                source_path: Some("one.txt".into()),
            },
            KnownDoc {
                path: "manuscript/two.md".into(),
                source_path: Some("two.txt".into()),
            },
            KnownDoc {
                path: "manuscript/part/Ch.md".into(),
                source_path: Some("part/Ch.markdown".into()),
            },
        ];
        assert_eq!(
            classify(&link("two.txt#second", Some(0), false), Some(&d[0]), &d),
            TargetKind::Document {
                path: "manuscript/two.md".into(),
                fragment: Some("second".into())
            }
        );
        assert_eq!(
            classify(&link("../one.txt", Some(2), false), Some(&d[2]), &d),
            TargetKind::Document {
                path: "manuscript/one.md".into(),
                fragment: None
            }
        );
    }

    #[test]
    fn a_relative_link_inside_a_note_needs_every_origin_to_agree() {
        let d = docs();
        // `two.md` from manuscript/part/ does not resolve → refused.
        assert!(matches!(
            classify(&link("two.md#x", Some(0), true), Some(&d[0]), &d),
            TargetKind::Refused(_)
        ));
        // A target every folder agrees on.
        let same = vec![
            KnownDoc {
                path: "manuscript/one.md".into(),
                source_path: None,
            },
            KnownDoc {
                path: "manuscript/two.md".into(),
                source_path: None,
            },
        ];
        assert_eq!(
            classify(&link("two.md", Some(0), true), Some(&same[0]), &same),
            TargetKind::Document {
                path: "manuscript/two.md".into(),
                fragment: None
            }
        );
        // Root-anchored needs no provenance.
        assert_eq!(
            classify(&link("/research/x.md", Some(0), true), Some(&d[0]), &d),
            TargetKind::Document {
                path: "research/x.md".into(),
                fragment: None
            }
        );
    }

    #[test]
    fn markers_cut_sections_and_detect_swallowing() {
        let nonce = "n1";
        let raw = |text: &str| json!({"t": "RawBlock", "c": ["html", text]});
        let para = json!({"t": "Para", "c": []});
        let blocks = vec![
            para.clone(),
            raw(&end_marker(0, nonce)),
            raw(&begin_marker(1, nonce)),
            para.clone(),
            para.clone(),
            raw(&end_marker(1, nonce)),
        ];
        let secs = sections(&blocks, 2, false, nonce).unwrap();
        assert_eq!((secs[0].start, secs[0].end), (0, 1));
        assert_eq!((secs[1].start, secs[1].end), (3, 5));
        // A swallowed end marker: out of order.
        let broken = vec![
            para.clone(),
            raw(&begin_marker(1, nonce)),
            para.clone(),
            raw(&end_marker(1, nonce)),
        ];
        assert!(sections(&broken, 2, false, nonce).is_none());
        // Another nonce cannot impersonate a marker.
        let foreign = vec![para.clone(), raw("<!-- chikn:end 0 other -->")];
        assert!(sections(&foreign, 1, false, nonce).is_none());
    }
}
