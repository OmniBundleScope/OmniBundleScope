/* OmniBundle report shell — WS-5.
 *
 * Squarified treemap (Bruls, Huizing & van Wijk) on a 2D canvas, drawn with
 * label culling so 10k nodes stay responsive: anything that cannot fit a legible
 * label is painted as a block only. No dependencies, no network, no framework.
 */
(function () {
  "use strict";

  var DATA = readPayload();
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
    if (!raw) return { nodes: [], modules: {}, assets: [], totals: {}, diagnostics: [] };
    try {
      return JSON.parse(raw);
    } catch (e) {
      return { nodes: [], modules: {}, assets: [], totals: {}, diagnostics: [], parseError: String(e) };
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

  function measureOf(item) {
    var s = item.sizes || {};
    var v = state.measure === "stat" ? s.stat : state.measure === "gzip" ? s.gzip : s.parsed;
    if (v == null || v === 0) v = s.stat || 0;
    if (state.measure === "attributed" && s.attributed != null) v = s.attributed;
    return v || 0;
  }

  // Deterministic hue per group: a curated, desaturated set so a treemap reads
  // as data rather than as a rainbow (the AI-slop failure mode).
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

  function keyOfModule(m) {
    if (state.dim === "package") return (m.package && m.package.name) || "<app>";
    if (state.dim === "source") return m.sources && m.sources.length ? m.sources[0].file : m.name;
    return m.chunks && m.chunks.length ? "chunk " + m.chunks.join("+") : "no chunk";
  }

  function matchesFilter(text) {
    if (!state.filter) return true;
    return fuzzy(state.filter, text.toLowerCase());
  }

  // Subsequence match, cheap and good enough for a filter box: every query
  // character must appear in order. Returns false on a miss, true on a hit.
  function fuzzy(needle, hay) {
    var i = 0;
    for (var j = 0; j < hay.length && i < needle.length; j++) {
      if (hay.charAt(j) === needle.charAt(i)) i++;
    }
    return i === needle.length;
  }

  function buildTree() {
    var root = { name: DATA.target || "bundle", size: 0, children: [] };
    var byGroup = Object.create(null);

    var modules = DATA.modules || {};
    Object.keys(modules).forEach(function (id) {
      var m = modules[id];
      var g = keyOfModule(m);
      var size = measureOf(m);
      if (!matchesFilter(g) && !matchesFilter(m.name.toLowerCase())) return;
      if (!byGroup[g]) byGroup[g] = { name: g, size: 0, children: [], kind: state.dim };
      byGroup[g].size += size;
      byGroup[g].children.push({ name: m.name, size: size, id: id, kind: "module" });
    });

    root.children = Object.keys(byGroup)
      .map(function (k) { return byGroup[k]; })
      .sort(function (a, b) { return b.size - a.size; });

    // assets form the outer level: a module's bytes live inside an asset, and
    // showing the asset structure is what makes chunk boundaries visible.
    var assets = (DATA.assets || []).slice();
    var assetTotal = assets.reduce(function (a, b) { return a + (b.size || 0); }, 0);
    root.size = assetTotal || root.children.reduce(function (a, c) { return a + c.size; }, 0);
    root.children = assets.map(function (a) {
      var copy = JSON.parse(JSON.stringify(a));
      copy.kind = "asset";
      copy.sizes = a.sizes || { stat: a.size };
      copy.children = root.children;
      return copy;
    });
    return root;
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

  /* ---------------------------------------------------------------- render */

  function resize() {
    var dpr = window.devicePixelRatio || 1;
    var r = el.canvas.parentElement.getBoundingClientRect();
    el.canvas.width = Math.max(1, Math.floor(r.width * dpr));
    el.canvas.height = Math.max(1, Math.floor(r.height * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  }

  function draw() {
    var r = el.canvas.getBoundingClientRect();
    var tree = buildTree();
    var w = r.width;
    var h = r.height;

    ctx.clearRect(0, 0, w, h);
    hitboxes = [];

    var level = state.focus ? state.focus.node : tree;
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
    var tree = buildTree();
    var level = state.focus ? state.focus.node : tree;
    var items = (level.children || []).slice().sort(function (a, b) { return b.size - a.size; });
    var max = items.length ? items[0].size : 1;

    el.results.innerHTML = items
      .map(function (it) {
        var active = state.selected && sameNode(state.selected, it);
        var pct = max ? Math.round((it.size / max) * 100) : 0;
        return (
          '<button type="button" class="row' + (active ? " is-active" : "") + '" role="option" aria-selected="' +
          (active ? "true" : "false") + '" data-name="' + esc(it.name) + '" data-kind="' + esc(it.kind) + '">' +
          '<span class="row__name">' + esc(it.name) + "</span>" +
          '<span class="row__bar"><i style="width:' + pct + '%"></i></span>' +
          '<span class="row__size">' + esc(bytes(it.size)) + "</span></button>"
        );
      })
      .join("");

    Array.prototype.forEach.call(el.results.children, function (row) {
      row.addEventListener("click", function () {
        select({ name: row.dataset.name, kind: row.dataset.kind });
      });
    });
  }

  function renderCrumbs() {
    var path = [];
    var f = state.focus;
    while (f) {
      path.unshift(f.node);
      f = f.parent;
    }
    var html = path
      .map(function (n, i) {
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

    var s = node.sizes || {};
    var m = node.id ? (DATA.modules || {})[node.id] : null;
    var rows = [
      ["stat", bytes(s.stat)],
      ["parsed", bytes(s.parsed)],
      ["gzip", bytes(s.gzip)],
    ];
    if (s.attributed != null) rows.push(["attributed", bytes(s.attributed)]);

    var share = (DATA.totals || {}).total_size
      ? ((node.size / DATA.totals.total_size) * 100).toFixed(1) + "%"
      : "—";

    var html = "";
    html += '<div><div class="ins__name">' + esc(node.name) + "</div>";
    html += '<div class="ins__kind">' + esc(node.kind || "group") + "</div></div>";

    html += '<dl class="kv">';
    html += "<dt>size</dt><dd>" + esc(bytes(node.size)) + "</dd>";
    html += "<dt>share</dt><dd>" + esc(share) + "</dd>";
    rows.forEach(function (r) {
      html += "<dt>" + r[0] + "</dt><dd>" + esc(r[1]) + "</dd>";
    });
    html += "</dl>";

    if (m) {
      if (m.package) {
        html +=
          '<div class="ins__section"><div class="ins__title">package</div><ul class="ins__list"><li>' +
          esc(m.package.name) +
          (m.package.version ? " " + esc(m.package.version) : "") +
          "</li></ul></div>";
      }
      if (m.reasons && m.reasons.length) {
        html +=
          '<div class="ins__section"><div class="ins__title">pulled in by</div><ul class="ins__list">' +
          m.reasons
            .slice(0, 8)
            .map(function (r) { return "<li>" + esc(r) + "</li>"; })
            .join("") +
          "</ul></div>";
      }
      if (m.sources && m.sources.length) {
        html +=
          '<div class="ins__section"><div class="ins__title">mapped sources</div><ul class="ins__list">' +
          m.sources
            .slice(0, 8)
            .map(function (s2) { return "<li>" + esc(s2.file) + "</li>"; })
            .join("") +
          "</ul></div>";
      }
      if (m.attribution_delta) {
        var d = m.attribution_delta;
        html +=
          '<div><span class="badge' + (d > 0 ? " badge--warn" : "") + '">attribution ' +
          (d > 0 ? "+" : "") + bytes(Math.abs(d)) + "</span></div>";
      }
    } else if (node.children && node.children.length) {
      html +=
        '<div class="ins__section"><div class="ins__title">contains</div><ul class="ins__list">' +
        node.children
          .slice(0, 10)
          .map(function (c) { return "<li>" + esc(c.name) + " · " + esc(bytes(c.size)) + "</li>"; })
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
