"use strict";
const panes = Object.fromEntries(["pdf", "accepted", "working"].map(id => [id, document.getElementById(id)]));
let state = null, selected = null, hovered = null, leader = "working", changeIndex = -1;
let pointerX = 0, headingCommand = null, loading = false, requested = false;
const collapsed = new Set(), suppressed = new Map();
let nodes = {pdf: [], accepted: [], working: []};

function elementIndex(snapshot) {
  const result = new Map();
  for (const chapter of snapshot?.chapters || []) for (const element of chapter.elements) {
    result.set(element.id, {element, chapter});
  }
  return result;
}
function boxStyle(node, bbox, width, height) {
  const [x0,y0,x1,y1] = bbox;
  Object.assign(node.style, {left: `${100*x0/width}%`, top: `${100*y0/height}%`,
    width: `${100*(x1-x0)/width}%`, height: `${100*(y1-y0)/height}%`});
}
function imageHolder(image, width, height, className) {
  const holder = document.createElement("div"); holder.className = className;
  holder.style.aspectRatio = `${width}/${Math.max(1,height)}`;
  const img = document.createElement("img"); img.src = image; img.loading = "lazy";
  img.width = width; img.height = height; img.alt = "";
  holder.append(img); return holder;
}
function overlay(holder, id, bbox, width, height, pane, source = null) {
  if (height <= 0) return;
  const node = document.createElement("div"); node.className = "overlay";
  node.dataset.id = id; node.title = id;
  boxStyle(node,bbox,width,height); holder.append(node);
  nodes[pane].push({id, node, source});
  node.addEventListener("pointerenter", () => { hovered = id; highlight(); });
  node.addEventListener("pointerleave", () => { hovered = null; highlight(); });
  node.addEventListener("click", event => {
    event.stopPropagation(); selected = id; highlight(); selectionText();
    const entry = elementIndex(state?.working).get(id);
    if (pane === "working" && entry && state.heading_types.includes(entry.element.type)) editHeading(entry.element);
  });
}
function buildMarkdown(version) {
  const pane = panes[version], snapshot = state[version], index = elementIndex(snapshot);
  const fragment = document.createDocumentFragment();
  for (const layout of snapshot?.render?.chapters || []) {
    const chapter = snapshot.chapters.find(c => c.id === layout.id);
    const section = document.createElement("div"); section.dataset.chapter = layout.id;
    const content = document.createElement("div"); content.className = "chapter-content";
    if (chapter) {
      const toggle = document.createElement("button"); toggle.className = "chapter-toggle";
      toggle.textContent = `${collapsed.has(layout.id) ? "▸" : "▾"} ${chapter.title}`;
      toggle.addEventListener("click", () => {
        if (collapsed.has(layout.id)) collapsed.delete(layout.id); else collapsed.add(layout.id);
        applyCollapse(layout.id); sync(leader);
      });
      section.append(toggle); content.hidden = collapsed.has(layout.id);
    }
    for (const strip of layout.strips) {
      if (!strip.image || strip.height <= 0) continue;
      const holder = imageHolder(`/files/${version}/${strip.image}`,strip.width,strip.height,"strip");
      holder.dataset.id = strip.id;
      if (index.has(strip.id)) overlay(holder,strip.id,strip.bbox,strip.width,strip.height,version);
      else if (strip.id === "generated-toc") nodes[version].push({id: strip.id, node: holder, source: null});
      for (const link of strip.links) {
        const hit = document.createElement("a"); hit.className = "toc-link";
        hit.href = `#${link.target}`; hit.title = `Go to ${link.target}`;
        boxStyle(hit,link.bbox,strip.width,strip.height);
        hit.addEventListener("click", event => {event.preventDefault(); navigate(link.target,version);});
        holder.append(hit);
      }
      content.append(holder);
    }
    section.append(content); fragment.append(section);
  }
  pane.replaceChildren(fragment);
}
function buildPdf() {
  const fragment = document.createDocumentFragment();
  const mappings = [];
  for (const version of ["working","accepted"]) for (const {element} of elementIndex(state[version]).values()) {
    element.pdf_regions.forEach((region,i) => mappings.push({element,region,i,version}));
  }
  mappings.sort((a,b) => Number(a.element.visible === false)-Number(b.element.visible === false));
  for (const page of state.pdf_pages) {
    const holder = imageHolder(`/files/${page.image}`,page.width,page.height,"page");
    holder.dataset.page = page.page;
    const seen = new Set();
    for (const item of mappings.filter(m => m.region.page === page.page)) {
      const key = `${item.element.id}:${JSON.stringify(item.region.bbox)}`;
      if (seen.has(key)) continue; seen.add(key);
      overlay(holder,item.element.id,item.region.bbox,page.width,page.height,"pdf",item);
    }
    fragment.append(holder);
  }
  panes.pdf.replaceChildren(fragment);
}
function applyCollapse(id) {
  for (const version of ["accepted","working"]) {
    for (const section of panes[version].querySelectorAll("[data-chapter]")) {
      if (section.dataset.chapter !== id || id === "toc") continue;
      section.querySelector(".chapter-content").hidden = collapsed.has(id);
      const chapter = state[version]?.chapters.find(c => c.id === id);
      section.querySelector("button").textContent = `${collapsed.has(id) ? "▸" : "▾"} ${chapter.title}`;
    }
  }
}
function expand(id) {
  for (const version of ["working","accepted"]) {
    const entry = elementIndex(state[version]).get(id);
    if (entry) { collapsed.delete(entry.chapter.id); applyCollapse(entry.chapter.id); }
  }
}
function visibleNodes(pane, id = null) {
  return nodes[pane].filter(entry => (!id || entry.id === id) && entry.node.getClientRects().length);
}
function highlight() {
  const changed = new Set((state?.changes || []).map(c => c.id));
  for (const entries of Object.values(nodes)) for (const entry of entries) {
    if(entry.source) entry.node.style.visibility=entry.source.element.visible === false && entry.id !== selected ? "hidden" : "visible";
    entry.node.classList.toggle("selected",entry.id === selected);
    entry.node.classList.toggle("hover",entry.id === hovered);
    entry.node.classList.toggle("changed",changed.has(entry.id));
  }
  document.getElementById("copy").disabled = !selected || selected === "generated-toc";
}
function selectionText() {
  if (!selected) return;
  if(selected === "generated-toc") {
    document.getElementById("selection").textContent=`Generated Table of Contents · accepted: ${state.accepted?.toc.enabled ? "visible" : "hidden"} · working: ${state.working?.toc.enabled ? "visible" : "hidden"}`;
    return;
  }
  const statuses = ["accepted","working"].map(version => {
    const entry = elementIndex(state[version]).get(selected);
    return `${version}: ${!entry ? "missing" : entry.element.visible === false ? "hidden" : entry.element.type}`;
  });
  document.getElementById("selection").textContent = `${selected} · ${statuses.join(" · ")}`;
}
function anchor(pane) {
  const viewport = panes[pane].getBoundingClientRect(), line = viewport.top + .25*panes[pane].clientHeight;
  let candidates = visibleNodes(pane);
  if (pane === "pdf") {
    candidates = candidates.filter(e => e.source.element.visible !== false);
    const horizontal = candidates.filter(e => {const r=e.node.getBoundingClientRect();return pointerX >= r.left && pointerX <= r.right;});
    if (horizontal.length) candidates = horizontal;
  }
  const distance = entry => {
    const r=entry.node.getBoundingClientRect(); return line < r.top ? r.top-line : line > r.bottom ? line-r.bottom : 0;
  };
  candidates.sort((a,b) => distance(a)-distance(b) || Number(a.source?.version !== "working")-Number(b.source?.version !== "working"));
  const entry = candidates[0];
  if (!entry) return null;
  const rect = entry.node.getBoundingClientRect();
  const local = Math.max(0,Math.min(1,(line-rect.top)/Math.max(1,rect.height)));
  const order=[...elementIndex(state[pane] || state.working).keys()];
  if (pane !== "pdf") return {id: entry.id, progress: local, order};
  const regions = entry.source.element.pdf_regions;
  const heights = regions.map(r => Math.max(0,r.bbox[3]-r.bbox[1]));
  const before = heights.slice(0,entry.source.i).reduce((a,b)=>a+b,0), total=heights.reduce((a,b)=>a+b,0);
  return {id:entry.id, progress:(before+local*heights[entry.source.i])/Math.max(1,total),order};
}
function position(pane, logical, fallback = true) {
  if (!logical) return;
  if(logical.id === "generated-toc" && pane === "pdf") return;
  let counterparts = visibleNodes(pane,logical.id), progress = logical.progress;
  if (!counterparts.length && fallback) {
    const snapshot = state[leader] || state.working;
    const order = logical.order || [...elementIndex(snapshot).keys()], index=order.indexOf(logical.id);
    for(let delta=1;delta<order.length && !counterparts.length;delta++) {
      for(const candidate of [order[index-delta],order[index+delta]]) {
        // Scroll fallback requires an actual common logical identity.
        if(candidate && elementIndex(state.working).has(candidate) && elementIndex(state.accepted).has(candidate)) {
          counterparts=visibleNodes(pane,candidate); if(counterparts.length) {progress=0;break;}
        }
      }
    }
  }
  if (!counterparts.length) return;
  let entry=counterparts[0], local=progress;
  if(pane === "pdf") {
    // Bind to the leader version's own source corrections.
    const preferred = leader === "accepted" ? "accepted" : "working";
    const model = elementIndex(state[preferred]).get(entry.id)?.element || entry.source.element;
    const regions = model.pdf_regions, heights=regions.map(r=>Math.max(0,r.bbox[3]-r.bbox[1]));
    let distance=progress*heights.reduce((a,b)=>a+b,0), chosen=regions.length-1;
    for(let i=0;i<heights.length;i++) {if(distance<=heights[i]){chosen=i;break;}distance-=heights[i];}
    const region=regions[chosen]; if(!region) return;
    entry=counterparts.find(e=>e.source.region.page===region.page && JSON.stringify(e.source.region.bbox)===JSON.stringify(region.bbox));
    if(!entry) return;
    local=distance/Math.max(1,heights[chosen]);
  }
  const rect=entry.node.getBoundingClientRect(), viewport=panes[pane].getBoundingClientRect();
  const top=panes[pane].scrollTop+rect.top-viewport.top+local*rect.height-.25*panes[pane].clientHeight;
  const target=Math.max(0,Math.min(top,panes[pane].scrollHeight-panes[pane].clientHeight));
  if(Math.abs(panes[pane].scrollTop-target)>.5) {suppressed.set(pane,target);panes[pane].scrollTop=target;}
}
function sync(pane) {
  const logical=anchor(pane);
  for(const follower of Object.keys(panes)) if(follower!==pane) position(follower,logical);
}
function navigate(id, preferred="working") {
  expand(id); selected=id; highlight();selectionText();
  leader=visibleNodes(preferred,id).length ? preferred : visibleNodes("accepted",id).length ? "accepted" : "pdf";
  for(const pane of Object.keys(panes)) position(pane,{id,progress:0},false);
}
async function post(url, command) {
  const response=await fetch(url,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(command)});
  const result=await response.json(); if(!response.ok) throw new Error(result.error);
  return result;
}
function showError(message) {
  const status=document.getElementById("status");status.textContent=message;status.classList.add("error");
}
function editHeading(element) {
  headingCommand={id:element.id,expected_revision:state.authored_revision};
  document.getElementById("heading-id").textContent=element.id;
  document.getElementById("level").value=element.heading_level;
  document.getElementById("heading-dialog").showModal();
}
async function refresh() {
  requested=true; if(loading) return; loading=true;
  try {
    while(requested) {
      requested=false;
      const response=await fetch("/api/status"); if(!response.ok) throw new Error("Cannot load review state");
      const next=await response.json();
      const changed=!state || next.working?.revision!==state.working?.revision || next.accepted?.revision!==state.accepted?.revision;
      // Capture after the request, so a scroll while awaiting it is retained.
      const logical=state ? anchor(leader) : null;
      state=next;
      if(changed) {
        nodes={pdf:[],accepted:[],working:[]};buildMarkdown("accepted");buildMarkdown("working");buildPdf();
        const chapter=document.getElementById("chapter"), old=chapter.value;chapter.replaceChildren(new Option("Choose…",""));
        const known=new Set();
        for(const snapshot of [state.working,state.accepted]) for(const c of snapshot?.chapters || []) {
          if(!known.has(c.id)){known.add(c.id);chapter.append(new Option(c.title,c.id));}
        }
        chapter.value=old;
        if(logical) {position(leader,logical);sync(leader);}
      }
      document.getElementById("accept").disabled=!state.can_accept;
      document.getElementById("toc-enabled").checked=state.working?.toc.enabled ?? true;
      document.getElementById("toc-depth").value=state.working?.toc.depth ?? 2;
      const status=document.getElementById("status");status.classList.toggle("error",Boolean(state.error));
      status.textContent=state.error || (state.pending ? "Rendering pending…" : `${state.changes.length} changes · Ready`);
      highlight();selectionText();
    }
  } catch(error) {showError(error.message);} finally {loading=false;}
}
for(const [name,pane] of Object.entries(panes)) {
  pane.addEventListener("pointermove",event=>{pointerX=event.clientX;});
  for(const type of ["wheel","touchstart","pointerdown","keydown"]) pane.addEventListener(type,()=>{leader=name;suppressed.delete(name);},{passive:true});
  pane.addEventListener("scroll",()=>{
    if(suppressed.has(name) && Math.abs(pane.scrollTop-suppressed.get(name))<1) {suppressed.delete(name);return;}
    suppressed.delete(name);leader=name;sync(name);
  },{passive:true});
}
document.getElementById("chapter").addEventListener("change",event=>{
  const chapter=state.working?.chapters.find(c=>c.id===event.target.value) || state.accepted?.chapters.find(c=>c.id===event.target.value);
  if(chapter) {collapsed.delete(chapter.id);applyCollapse(chapter.id);const e=chapter.elements.find(e=>e.visible!==false);if(e)navigate(e.id);}
});
function nextChange(delta) {
  if(!state?.changes.length)return;
  changeIndex=(changeIndex+delta+state.changes.length)%state.changes.length;
  navigate(state.changes[changeIndex].id);
}
document.getElementById("previous").onclick=()=>nextChange(-1);
document.getElementById("next").onclick=()=>nextChange(1);
document.getElementById("copy").onclick=async()=>{try{await navigator.clipboard.writeText(selected);}catch(error){showError(error.message);}};
async function saveToc() {
  try {await post("/api/toc",{enabled:document.getElementById("toc-enabled").checked,
    depth:Number(document.getElementById("toc-depth").value),expected_revision:state.authored_revision});await refresh();}
  catch(error){showError(error.message);}
}
document.getElementById("toc-enabled").onchange=saveToc;
document.getElementById("toc-depth").onchange=saveToc;
document.getElementById("accept").onclick=async()=>{try{await post("/api/accept",{revision:state.working.revision});await refresh();}catch(error){showError(error.message);}};
document.getElementById("save-heading").onclick=async()=>{
  try{await post("/api/heading",{...headingCommand,level:Number(document.getElementById("level").value)});document.getElementById("heading-dialog").close();await refresh();}
  catch(error){showError(error.message);document.getElementById("heading-dialog").close();await refresh();showError(error.message);}
};
const events=new EventSource("/api/events");events.onmessage=refresh;events.onopen=refresh;
events.onerror=()=>showError("Connection interrupted; reconnecting…");
new ResizeObserver(()=>{if(state)sync(leader);}).observe(document.querySelector("main"));
refresh();
