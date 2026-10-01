/* OmniBundleScope report shell — report.
 *
 * Squarified treemap (Bruls, Huizing & van Wijk) on a 2D canvas, drawn with
 * label culling so 10k nodes stay responsive: anything that cannot fit a legible
 * label is painted as a block only. No dependencies, no network, no framework.
 */
(function () {
  "use strict";

  var DATA = readPayload();
  var DETAIL = readDetail();
  var state = {
    dim: "package", // grouping dimension
    measure: "parsed", // size dimension
    filter: "",
    focus: null, // { group, name } for zoom
    selected: null, // node key in the current level
  };

  var el = {
    target: document.getElementById("target"),
    summary: document.getElementById("summary"),
    canvas: document.getElementById("map"),
    tip: document.getElementById("tip"),
    crumbs: document.getElementById("crumbs"),
    results: document.getElementById("results"),
    search: document.getElementById("search"),
    clear: document.getElementById("clear"),
    inspect: document.getElementById("inspectBody"),
    inspectEmpty: document.getElementById("inspectEmpty"),
    empty: document.getElementById("empty"),
    diag: document.getElementById("diag"),
    dimSeg: document.getElementById("dimension"),
    dims: Array.prototype.slice.call(document.querySelectorAll(".dims__item")),
  };

  var ctx = el.canvas.getContext("2d");
  var nodes = []; // laid-out rectangles for the current level
  var hitboxes = [];

  /* ---------------------------------------------------------------- utils */

  function readPayload() {
    var raw = document.getElementById("payload").textContent.trim();
    if (!raw) return { trees: [], totals: {}, diagnostics: [] };
    try {
      return JSON.parse(raw);
    } catch (e) {
      return { trees: [], totals: {}, diagnostics: [], parseError: String(e) };
    }
  }

  // Per-module detail. For small graphs it is an inline JSON island; for large
  // ones it arrives as `window.__OB_DETAIL__` from a companion `<script>`
  // (a `<script src>` works from `file://`, `fetch` does not).
  function readDetail() {
    if (window.__OB_DETAIL__) return window.__OB_DETAIL__;
    var island = document.getElementById("detail");
    if (!island) return null;
    var text = island.textContent.trim();
    if (!text) return null;
    try {
      return JSON.parse(text);
    } catch (e) {
      return null;
    }
  }

  function bytes(n) {
    if (n == null) return "—";
    if (n < 1024) return n + " B";
    if (n < 1048576) return (n / 1024).toFixed(n < 10240 ? 1 : 0) + " KB";
    if (n < 1073741824) return (n / 1048576).toFixed(1) + " MB";
    return (n / 1073741824).toFixed(2) + " GB";
  }

  function esc(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c];
    });
  }

  // Deterministic hue per group: a curated, desaturated set, so that the same
  // as data rather than as a rainbow, which is the usual way a treemap fails.
  var HUES = [16, 42, 96, 150, 186, 208, 258, 292, 320, 348];

  function colourFor(key, depth) {
    var h = 0;
    for (var i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) >>> 0;
    var hue = HUES[h % HUES.length];
    var sat = 46 - depth * 7;
    var light = 52 + depth * 6;
    var dark = isDark();
    return "hsl(" + hue + " " + sat + "% " + (dark ? light : 100 - light) + "%)";
  }

  function isDark() {
    var t = document.documentElement.getAttribute("data-theme");
    if (t === "dark") return true;
    if (t === "light") return false;
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  /* ------------------------------------------------------------ tree build */

  // The treemap itself is built server-side (see treeFor). What is left here is
  // the flat index the filter, the sidebar and the inspector walk.
  var indexCache = { dim: null, rows: [] };

  function indexFor(dim) {
    if (indexCache.dim === dim) return indexCache.rows;
    var rows = flatten(treeFor(dim), [], []);
    rows.sort(function (a, b) { return b.node.size - a.node.size; });
    indexCache = { dim: dim, rows: rows };
    return rows;
  }

  function matchesFilter(text) {
    if (!state.filter) return true;
    return fuzzy(state.filter, text.toLowerCase());
  }

  // Subsequence match, cheap and good enough for a filter box: every query
  // character must appear in order.
  function fuzzy(needle, hay) {
    var i = 0;
    for (var j = 0; j < hay.length && i < needle.length; j++) {
      if (hay.charAt(j) === needle.charAt(i)) i++;
    }
    return i === needle.length;
  }

  /* --------------------------------------------------------------- squarify */

  // Squarified treemap: Bruls/Huizing/van Wijk. Lays out `items` (already
  // sorted by size desc) into the rect, and returns laid-out leaves.
  function squarify(items, x, y, w, h) {
    var out = [];
    var total = items.reduce(function (a, b) { return a + b.size; }, 0);
    if (total <= 0 || w <= 0 || h <= 0) return out;

    var scale = (w * h) / total;
    var rest = items.slice();
    var rect = { x: x, y: y, w: w, h: h };

    function worst(row, side) {
      var s = row.reduce(function (a, b) { return a + b.size * scale; }, 0);
      if (s <= 0) return Infinity;
      var mx = row.reduce(function (a, b) { return Math.max(a, b.size * scale); }, 0);
      var mn = row.reduce(function (a, b) { return Math.min(a, b.size * scale); }, Infinity);
      return Math.max((side * side * mx) / (s * s), (s * s) / (side * side * mn));
    }

    while (rest.length) {
      var vertical = rect.w >= rect.h; // lay the row along the shorter side
      var side = vertical ? rect.h : rect.w;
      var row = [];
      var best = Infinity;
      while (rest.length) {
        var candidate = worst(row.concat([rest[0]]), side);
        if (row.length && candidate > best) break;
        row.push(rest.shift());
        best = candidate;
      }

      var rowArea = row.reduce(function (a, b) { return a + b.size * scale; }, 0);
      var thickness = rowArea / side;

      if (vertical) {
        var cy = rect.y;
        row.forEach(function (it) {
          var hgt = (it.size * scale) / thickness;
          out.push({ node: it, x: rect.x, y: cy, w: thickness, h: hgt });
          cy += hgt;
        });
        rect = { x: rect.x + thickness, y: rect.y, w: rect.w - thickness, h: rect.h };
      } else {
        var cx = rect.x;
        row.forEach(function (it) {
          var wid = (it.size * scale) / thickness;
          out.push({ node: it, x: cx, y: rect.y, w: wid, h: thickness });
          cx += wid;
        });
        rect = { x: rect.x, y: rect.y + thickness, w: rect.w, h: rect.h - thickness };
      }
      if (rect.w <= 0.5 || rect.h <= 0.5) break;
    }
    return out;
  }

  /* ------------------------------------------------------------ tree input */

  // The payload carries one treemap tree per dimension: asset → group, sizes
  // only. The full module list arrives separately (inlined for small graphs, or
  // in a companion script for large ones) and is used for filtering, the
  // inspector, and drill-down.
  function treeFor(dim) {
    var trees = DATA.trees || [];
    for (var i = 0; i < trees.length; i++) {
      if (trees[i].dimension === dim) return trees[i].tree;
    }
    return { name: "bundle", size: 0, children: [] };
  }

  function detailModules() {
    if (DETAIL && DETAIL.modules) return DETAIL.modules;
    return DATA.modules || {};
  }

  function flatten(node, out, path) {
    out = out || [];
    path = path || [];
    var here = path.concat([node.name]);
    out.push({ node: node, path: here, depth: path.length });
    (node.children || []).forEach(function (c) { flatten(c, out, here); });
    return out;
  }

  /* ---------------------------------------------------------------- render */

  function resize() {
    var dpr = window.devicePixelRatio || 1;
    var r = el.canvas.parentElement.getBoundingClientRect();
    el.canvas.width = Math.max(1, Math.floor(r.width * dpr));
    el.canvas.height = Math.max(1, Math.floor(r.height * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  }

  function currentLevel() {
    return state.focus ? state.focus.node : treeFor(state.dim);
  }

  function draw() {
    var r = el.canvas.getBoundingClientRect();
    var w = r.width;
    var h = r.height;

    ctx.clearRect(0, 0, w, h);
    hitboxes = [];

    var level = currentLevel();
    var items = (level.children || []).filter(function (c) { return c.size > 0; });
    el.empty.hidden = items.length > 0;

    var leaves = squarify(items, 0, 0, w, h);
    nodes = leaves;

    var depth = state.focus ? state.focus.depth + 1 : 1;
    var total = items.reduce(function (a, b) { return a + b.size; }, 0);

    leaves.forEach(function (L) {
      var gap = 1;
      var x = L.x + gap;
      var y = L.y + gap;
      var bw = Math.max(0, L.w - gap * 2);
      var bh = Math.max(0, L.h - gap * 2);
      if (bw < 1 || bh < 1) return;

      var frac = total ? L.node.size / total : 0;
      ctx.fillStyle = colourFor(L.node.name, depth);
      ctx.fillRect(x, y, bw, bh);

      // A 1px inner rule instead of a drop shadow: cheaper to draw, and it
      // keeps the block edges crisp at any zoom.
      ctx.strokeStyle = isDark() ? "rgba(0,0,0,0.35)" : "rgba(0,0,0,0.16)";
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, bw - 1, bh - 1);

      var selected = state.selected && sameNode(state.selected, L.node);
      if (selected) {
        ctx.strokeStyle = cssVar("--accent");
        ctx.lineWidth = 2;
        ctx.strokeRect(x + 1, y + 1, bw - 2, bh - 2);
      }

      hitboxes.push({ x: x, y: y, w: bw, h: bh, node: L.node, depth: depth });

      // label culling: only label what can be read
      if (bw > 54 && bh > 26) {
        var fs = Math.min(12, Math.max(9, Math.floor(bh / 3.2)));
        ctx.font = (depth === 1 ? "500 " : "") + fs + "px " + fontStack();
        ctx.fillStyle = isDark() ? "rgba(255,255,255,0.94)" : "rgba(0,0,0,0.88)";
        ctx.textBaseline = "top";
        var text = fitText(ctx, L.node.name, bw - 10);
        ctx.fillText(text, x + 5, y + 4);
        if (bh > 40 && frac > 0.02) {
          ctx.font = fs - 1 + "px " + fontStack();
          ctx.fillStyle = isDark() ? "rgba(255,255,255,0.7)" : "rgba(0,0,0,0.66)";
          ctx.fillText(fitText(ctx, bytes(L.node.size), bw - 10), x + 5, y + 6 + fs);
        }
      }
    });

    drawCrumbs();
  }

  function fontStack() {
    return 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace';
  }

  function fitText(c, text, maxW) {
    if (maxW <= 8) return "";
    if (c.measureText(text).width <= maxW) return text;
    var lo = 0;
    var hi = text.length;
    while (lo < hi) {
      var mid = (lo + hi + 1) >> 1;
      if (c.measureText(text.slice(0, mid) + "…").width <= maxW) lo = mid;
      else hi = mid - 1;
    }
    return text.slice(0, lo) + "…";
  }

  function cssVar(name) {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || "#e0803c";
  }

  function sameNode(a, b) {
    if (!a || !b) return false;
    if (a.id && b.id) return a.id === b.id;
    return a.name === b.name && a.kind === b.kind;
  }

  /* ------------------------------------------------------------- chrome: bar */

  function renderSummary() {
    var t = DATA.totals || {};
    var parts = [
      ["total", bytes(t.total_size)],
      ["assets", String(t.asset_count || 0)],
      ["modules", String(t.module_count || 0)],
      ["packages", String(t.package_count || 0)],
    ];
    if (DATA.sizeDimensionNote) parts.push(["dimension", DATA.sizeDimensionNote]);
    el.summary.innerHTML = parts
      .map(function (p) {
        return '<div class="stat"><span class="stat__k">' + esc(p[0]) + '</span><span class="stat__v">' + esc(p[1]) + "</span></div>";
      })
      .join("");
  }

  function renderDiagnostics() {
    var d = DATA.diagnostics || [];
    el.diag.innerHTML = d.length
      ? d
          .slice(0, 4)
          .map(function (x) {
            return (
              '<div class="diag"><code>' + esc(x.code) + "</code><span>" + esc(x.message) + "</span></div>"
            );
          })
          .join("") + (d.length > 4 ? '<div class="diag">+' + (d.length - 4) + " more</div>" : "")
      : '<div class="diag"><span>no diagnostics</span></div>';
  }

  function renderList() {
    var rows = indexFor(state.dim).filter(function (r) {
      if (r.depth === 0) return false; // the synthetic root
      return matchesFilter(r.node.name);
    });
    var max = rows.length ? rows[0].node.size : 1;
    var shown = rows.slice(0, 400);

    el.results.innerHTML = shown
      .map(function (r) {
        var it = r.node;
        var active = state.selected && sameNode(state.selected, it);
        var pct = max ? Math.round((it.size / max) * 100) : 0;
        var indent = Math.min(3, r.depth - 1) * 10;
        return (
          '<button type="button" class="row' + (active ? " is-active" : "") + '" role="option" aria-selected="' +
          (active ? "true" : "false") + '" data-name="' + esc(it.name) + '" style="padding-left:' + (10 + indent) + 'px">' +
          '<span class="row__name">' + esc(it.name) + "</span>" +
          '<span class="row__bar"><i style="width:' + pct + '%"></i></span>' +
          '<span class="row__size">' + esc(bytes(it.size)) + "</span></button>"
        );
      })
      .join("");

    if (rows.length > shown.length) {
      el.results.innerHTML +=
        '<div class="side__more">+' + (rows.length - shown.length) + ' more — refine the filter</div>';
    }

    Array.prototype.forEach.call(el.results.querySelectorAll(".row"), function (row) {
      row.addEventListener("click", function () {
        var r = indexFor(state.dim).filter(function (x) { return x.node.name === row.dataset.name; })[0];
        if (r && r.node.children && r.node.children.length) zoomTo(r);
        else if (r) select(r.node);
      });
    });
  }

  function renderCrumbs() {
    var path = [];
    var f = state.focus;
    while (f) {
      path.unshift(f);
      f = f.parent;
    }
    var html = path
      .map(function (f, i) {
        var n = f.node;
        return i === path.length - 1
          ? "<b>" + esc(n.name) + "</b>"
          : '<button type="button" data-i="' + i + '">' + esc(n.name) + "</button>";
      })
      .join(' <i>/</i> ');
    el.crumbs.innerHTML = html || "<b>" + esc(DATA.target || "bundle") + "</b>";
    Array.prototype.forEach.call(el.crumbs.querySelectorAll("button"), function (b) {
      b.addEventListener("click", function () {
        var idx = Number(b.dataset.i);
        var f2 = state.focus;
        while (f2 && f2.depth > idx) f2 = f2.parent;
        state.focus = f2;
        state.selected = null;
        draw();
        renderList();
      });
    });
  }

  function renderInspector() {
    var node = state.selected;
    if (!node) {
      el.inspectEmpty.hidden = false;
      el.inspect.hidden = true;
      return;
    }
    el.inspectEmpty.hidden = true;
    el.inspect.hidden = false;

    var modules = detailModules();
    var total = (DATA.totals || {}).total_size;
    var share = total ? ((node.size / total) * 100).toFixed(1) + "%" : "—";

    var html = "";
    html += '<div><div class="ins__name">' + esc(node.name) + "</div>";
    html += '<div class="ins__kind">' + esc((DATA.sizeDimension || "parsed") + " bytes") + "</div></div>";

    html += '<dl class="kv">';
    html += "<dt>size</dt><dd>" + esc(bytes(node.size)) + "</dd>";
    html += "<dt>share</dt><dd>" + esc(share) + "</dd>";
    html += "<dt>modules</dt><dd>" + esc(String(node.module_count || 0)) + "</dd>";
    if (node.dropped) html += "<dt>capped</dt><dd>" + esc(String(node.dropped)) + "</dd>";
    html += "</dl>";

    // Module facts: the largest children we know the names of, and their
    // reasons/sources when the detail payload is available.
    var kids = (node.children || []).slice(0, 12);
    if (kids.length) {
      html +=
        '<div class="ins__section"><div class="ins__title">largest inside</div><ul class="ins__list">' +
        kids
          .map(function (c) {
            return "<li>" + esc(c.name) + " · " + esc(bytes(c.size)) + "</li>";
          })
          .join("") +
        "</ul></div>";
    }

    var detailIds = [];
    for (var id in modules) {
      var m = modules[id];
      if (m && m.name === node.name) detailIds.push(m);
    }
    if (detailIds.length && detailIds[0].reasons && detailIds[0].reasons.length) {
      html +=
        '<div class="ins__section"><div class="ins__title">pulled in by</div><ul class="ins__list">' +
        detailIds[0].reasons
          .slice(0, 8)
          .map(function (r) { return "<li>" + esc(r) + "</li>"; })
          .join("") +
        "</ul></div>";
    }

    el.inspect.innerHTML = html;
  }

  /* ------------------------------------------------------------ interaction */

  function select(node) {
    state.selected = node;
    draw();
    renderList();
    renderInspector();
  }

  // Zoom to a node found in the flat index, rebuilding the focus chain from
  // the tree so the breadcrumb stays truthful.
  function zoomTo(row) {
    var chain = [];
    var want = row.node;
    function find(node, path) {
      if (node === want) {
        chain = path.concat([node]);
        return true;
      }
      var kids = node.children || [];
      for (var i = 0; i < kids.length; i++) {
        if (find(kids[i], path.concat([node]))) return true;
      }
      return false;
    }
    find(treeFor(state.dim), []);
    if (!chain.length) return;

    var focus = null;
    for (var i = chain.length - 1; i >= 0; i--) {
      focus = { node: chain[i], parent: focus, depth: i - 1 };
    }
    state.focus = focus;
    state.selected = null;
    draw();
    renderList();
    renderInspector();
  }

  function zoom(node) {
    if (!node || !node.children || !node.children.length) return;
    state.focus = { node: node, parent: state.focus, depth: state.focus ? state.focus.depth + 1 : 0 };
    state.selected = null;
    draw();
    renderList();
    renderInspector();
  }

  el.canvas.addEventListener("mousemove", function (ev) {
    var r = el.canvas.getBoundingClientRect();
    var x = ev.clientX - r.left;
    var y = ev.clientY - r.top;
    var hit = hitTest(x, y);
    if (!hit) {
      el.tip.hidden = true;
      el.canvas.style.cursor = "default";
      return;
    }
    el.canvas.style.cursor = hit.node.children && hit.node.children.length ? "zoom-in" : "default";
    el.tip.hidden = false;
    el.tip.innerHTML =
      "<b>" + esc(hit.node.name) + "</b><span>" + esc(bytes(hit.node.size)) + "</span>";
    var tw = 240;
    el.tip.style.left = Math.min(Math.max(6, x + 12), r.width - tw - 6) + "px";
    el.tip.style.top = Math.max(6, y - 44) + "px";
  });

  el.canvas.addEventListener("mouseleave", function () { el.tip.hidden = true; });

  el.canvas.addEventListener("click", function (ev) {
    var r = el.canvas.getBoundingClientRect();
    var hit = hitTest(ev.clientX - r.left, ev.clientY - r.top);
    if (!hit) return;
    if (hit.node.children && hit.node.children.length) zoom(hit.node);
    else select(hit.node);
  });

  function hitTest(x, y) {
    for (var i = hitboxes.length - 1; i >= 0; i--) {
      var b = hitboxes[i];
      if (x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h) return b;
    }
    return null;
  }

  var filterTimer = null;
  el.search.addEventListener("input", function () {
    clearTimeout(filterTimer);
    filterTimer = setTimeout(function () {
      state.filter = el.search.value.trim().toLowerCase();
      el.clear.hidden = !state.filter;
      state.focus = null;
      state.selected = null;
      draw();
      renderList();
      renderInspector();
    }, 90);
  });

  el.search.addEventListener("keydown", function (ev) {
    if (ev.key === "Escape") {
      el.search.value = "";
      state.filter = "";
      el.clear.hidden = true;
      draw();
      renderList();
    }
    if (ev.key === "Enter") {
      var first = el.results.querySelector(".row");
      if (first) first.click();
    }
  });

  el.clear.addEventListener("click", function () {
    el.search.value = "";
    state.filter = "";
    el.clear.hidden = true;
    draw();
    renderList();
  });

  el.dims.forEach(function (btn) {
    btn.addEventListener("click", function () {
      el.dims.forEach(function (b) { b.classList.toggle("is-active", b === btn); });
      state.dim = btn.dataset.dim;
      state.focus = null;
      state.selected = null;
      draw();
      renderList();
      renderInspector();
    });
  });

  Array.prototype.forEach.call(el.dimSeg.children, function (btn) {
    btn.addEventListener("click", function () {
      if (btn.disabled) return;
      Array.prototype.forEach.call(el.dimSeg.children, function (b) {
        b.setAttribute("aria-checked", String(b === btn));
      });
      state.measure = btn.dataset.dim;
      draw();
      renderList();
      renderInspector();
    });
  });

  document.getElementById("theme").addEventListener("click", function () {
    var el0 = document.documentElement;
    var order = ["auto", "light", "dark"];
    var cur = el0.getAttribute("data-theme") || "auto";
    el0.setAttribute("data-theme", order[(order.indexOf(cur) + 1) % order.length]);
    draw();
  });

  if (window.matchMedia) {
    var mq = window.matchMedia("(prefers-color-scheme: dark)");
    if (mq.addEventListener) mq.addEventListener("change", function () { if ((document.documentElement.getAttribute("data-theme") || "auto") === "auto") draw(); });
  }

  var ro = window.ResizeObserver ? new ResizeObserver(resize) : null;
  if (ro) ro.observe(el.canvas.parentElement);
  window.addEventListener("resize", resize);

  /* ------------------------------------------------------------------ boot */

  el.target.textContent = DATA.target || "bundle";
  renderSummary();
  renderDiagnostics();
  renderList();
  resize();
})();
