(() => {
  const toastEl = document.getElementById('toast');
  let toastTimer = null;
  const toast = (msg) => {
    toastEl.textContent = msg;
    toastEl.style.display = 'block';
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastEl.style.display = 'none';
    }, 2000);
  };

  const decodeBase64Utf8 = (value) => {
    const bytes = Uint8Array.from(atob(value.trim()), (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  };

  Function(decodeBase64Utf8(document.getElementById('vis-network-data').textContent))();

  const nodes = new vis.DataSet(JSON.parse(document.getElementById('nodes-data').textContent));
  const edges = new vis.DataSet(JSON.parse(document.getElementById('edges-data').textContent));
  const container = document.getElementById('graph');
  const data = { nodes: nodes, edges: edges };
  const options = {
    layout: {
      hierarchical: {
        direction: 'LR',
        sortMethod: 'directed',
        levelSeparation: 250,
        nodeSpacing: 100,
      },
    },
    physics: { enabled: false },
    interaction: { hover: true, navigationButtons: true, keyboard: true, zoomView: true },
    edges: { smooth: { type: 'cubicBezier', roundness: 0.6 } },
  };
  const network = new vis.Network(container, data, options);
  const centerId = Number(document.body.dataset.centerId);

  const initialViewport = () => {
    // After CSS/layout changes, the container can briefly be 0px tall/wide on first draw.
    // A tiny delay + redraw makes the initial graph reliably visible.
    network.redraw();
    network.fit({ animation: false });
    if (!Number.isNaN(centerId)) {
      network.focus(centerId, { scale: 1.0, animation: true });
    }
  };

  // Run once after first paint, and once after vis has had a chance to draw.
  setTimeout(initialViewport, 0);
  network.once('afterDrawing', () => setTimeout(initialViewport, 0));

  window.addEventListener('resize', () => {
    network.redraw();
  });

  const buildEditorUri = (node) => {
    const encodedPath = encodeURI(node.file).replace(/#/g, '%23').replace(/\?/g, '%3F');
    return 'vscode://file/' + encodedPath + ':' + node.line + ':' + (node.col || 0);
  };

  const side = document.getElementById('side');
  const sideEmpty = document.getElementById('sideEmpty');
  const sideBody = document.getElementById('sideBody');
  const sideBadges = document.getElementById('sideBadges');
  const insName = document.getElementById('insName');
  const insLoc = document.getElementById('insLoc');
  const insWhyRow = document.getElementById('insWhyRow');
  const insWhy = document.getElementById('insWhy');
  const btnCopy = document.getElementById('btnCopy');
  const btnOpen = document.getElementById('btnOpen');
  let currentSelection = null;

  const setBadges = (badges) => {
    sideBadges.innerHTML = '';
    for (const b of badges) {
      const el = document.createElement('div');
      el.className = 'badge';
      el.innerHTML = `<span class="dot" style="background:${b.color}"></span>${b.text}`;
      sideBadges.appendChild(el);
    }
  };

  const showInspector = (payload) => {
    currentSelection = payload;
    sideEmpty.style.display = 'none';
    sideBody.style.display = 'grid';
    insName.textContent = payload.title;
    insLoc.textContent = payload.location;
    if (payload.why) {
      insWhyRow.style.display = 'grid';
      insWhy.textContent = payload.why;
    } else {
      insWhyRow.style.display = 'none';
      insWhy.textContent = '';
    }
    setBadges(payload.badges || []);
  };

  const clearInspector = () => {
    currentSelection = null;
    sideEmpty.style.display = 'block';
    sideBody.style.display = 'none';
    sideBadges.innerHTML = '';
    insName.textContent = '';
    insLoc.textContent = '';
    insWhy.textContent = '';
    insWhyRow.style.display = 'none';
  };

  document.getElementById('btnHide').addEventListener('click', () => {
    side.style.display = 'none';
    toast('Inspector hidden (reload page to restore).');
  });
  document.getElementById('btnFit').addEventListener('click', () => network.fit({ animation: true }));
  document.getElementById('btnCenter').addEventListener('click', () => {
    if (!Number.isNaN(centerId)) network.focus(centerId, { scale: 1.0, animation: true });
  });
  document.getElementById('btnClear').addEventListener('click', () => {
    document.getElementById('search').value = '';
    clearInspector();
    network.unselectAll();
    toast('Cleared selection');
  });

  btnCopy.addEventListener('click', async () => {
    if (!currentSelection) return;
    try {
      await navigator.clipboard.writeText(currentSelection.copyText);
      toast('Copied');
    } catch {
      toast('Copy failed');
    }
  });
  btnOpen.addEventListener('click', () => {
    if (!currentSelection || !currentSelection.openUri) return;
    window.location.href = currentSelection.openUri;
  });

  network.on('doubleClick', function (params) {
    if (params.nodes.length > 0) {
      const node = nodes.get(params.nodes[0]);
      if (node && node.file) {
        window.location.href = buildEditorUri(node);
      }
    }
  });

  network.on('click', function (params) {
    if (params.nodes.length > 0) {
      const node = nodes.get(params.nodes[0]);
      const badges = [];
      if (node.kind) badges.push({ text: node.kind, color: node.color?.border || '#94a3b8' });
      if (node.changed) badges.push({ text: 'changed', color: '#ff6b6b' });
      const loc = node.file ? `${node.file}:${node.line}:${node.col || 0}` : '';
      showInspector({
        type: 'node',
        title: node.label || node.id,
        location: loc,
        why: node.title || '',
        copyText: node.label ? `${node.label}\n${loc}` : `${loc}`,
        openUri: node.file ? buildEditorUri(node) : null,
        badges,
      });
    } else if (params.edges.length > 0) {
      const edge = edges.get(params.edges[0]);
      const from = nodes.get(edge.from);
      const to = nodes.get(edge.to);
      const title = `${from?.label || edge.from} --${edge.label || ''}--> ${to?.label || edge.to}`;
      showInspector({
        type: 'edge',
        title,
        location: edge.title || '',
        why: edge.title || '',
        copyText: title + (edge.title ? `\n${edge.title}` : ''),
        openUri: null,
        badges: [{ text: edge.label || 'edge', color: edge.color?.color || '#94a3b8' }],
      });
    } else {
      clearInspector();
    }
  });

  const searchEl = document.getElementById('search');
  const findMatch = (q) => {
    const query = q.trim().toLowerCase();
    if (!query) return null;
    const all = nodes.get();
    let best = null;
    for (const n of all) {
      const hay = `${n.label || ''}\n${n.title || ''}`.toLowerCase();
      if (!hay.includes(query)) continue;
      best = n;
      if ((n.label || '').toLowerCase() === query) break;
    }
    return best;
  };
  const jumpTo = (node) => {
    network.selectNodes([node.id]);
    network.focus(node.id, { scale: 1.08, animation: true });
    const loc = node.file ? `${node.file}:${node.line}:${node.col || 0}` : '';
    showInspector({
      type: 'node',
      title: node.label || node.id,
      location: loc,
      why: node.title || '',
      copyText: node.label ? `${node.label}\n${loc}` : `${loc}`,
      openUri: node.file ? buildEditorUri(node) : null,
      badges: [
        ...(node.kind ? [{ text: node.kind, color: node.color?.border || '#94a3b8' }] : []),
        ...(node.changed ? [{ text: 'changed', color: '#ff6b6b' }] : []),
      ],
    });
  };

  const isEditableTarget = (el) => {
    if (!el) return false;
    if (el === searchEl) return true;
    const tag = (el.tagName || '').toLowerCase();
    return tag === 'input' || tag === 'textarea' || el.isContentEditable;
  };

  const focusSearch = () => {
    searchEl.focus();
    searchEl.select?.();
  };

  let liveTimer = null;
  const scheduleLiveJump = () => {
    clearTimeout(liveTimer);
    liveTimer = setTimeout(() => {
      const q = searchEl.value.trim();
      if (q.length < 2) return;
      const match = findMatch(q);
      if (match) jumpTo(match);
    }, 120);
  };

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !isEditableTarget(document.activeElement)) {
      e.preventDefault();
      focusSearch();
      toast('Search focused');
      return;
    }

    if (e.key === 'Escape') {
      if (document.activeElement === searchEl) {
        e.preventDefault();
        searchEl.value = '';
        clearInspector();
        network.unselectAll();
        searchEl.blur();
        toast('Cleared');
      }
    }
  });

  searchEl.addEventListener('input', () => scheduleLiveJump());
  searchEl.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      const match = findMatch(searchEl.value);
      if (!match) {
        toast('No match');
        return;
      }
      jumpTo(match);
    } else if (e.key === 'Escape') {
      searchEl.value = '';
      clearInspector();
      network.unselectAll();
      toast('Cleared');
    }
  });
})();

