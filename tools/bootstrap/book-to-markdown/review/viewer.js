"use strict";
const panes = Object.fromEntries(["pdf", "accepted", "working"].map(id => [id, document.getElementById(id)]));
let state = null, selected = null, hovered = null, leader = "working", changeIndex = -1;
let pointerX = 0, loading = false, requested = false, resizing = false;
const collapsed = new Set(), suppressed = new Map();
let nodes = {pdf: [], accepted: [], working: []};
let lastLogical = null, publishing = false;


const showHidden = document.getElementById("show-hidden");
const showTypes = document.getElementById("show-types");
const typeLabelContext=document.createElement("canvas").getContext("2d");
const actionsButton = document.getElementById("element-actions");
const elementMenu = document.getElementById("element-menu");
const visibilityButton = document.getElementById("menu-visibility");
const hidePageButton = document.getElementById("menu-hide-page");
const headingButton = document.getElementById("menu-heading");
const headingMenu = document.getElementById("heading-menu");
const typeButton = document.getElementById("menu-type");
const typeMenu = document.getElementById("type-menu");
const roleButton = document.getElementById("menu-role");
const roleMenu = document.getElementById("role-menu");
const markdownDialog = document.getElementById("markdown-dialog");
const markdownText = document.getElementById("markdown-text");
const markdownSecond = document.getElementById("markdown-second");
const codexDialog = document.getElementById("codex-dialog");
let codexCommand=null, codexRunning=false, codexScope="element";
let codexAutoCloseJob=null, codexAutoClosePending=false;
let textCommand=null, splitting=false, merging=false, activeText=markdownText;
const discardElementButton = document.getElementById("menu-discard");
let selectedPane="working", selectedPage=null, menuCommand=null, pageCommand=null, menuReturnFocus=null, menuScrollPosition={};
const chapterTitle=chapter=>chapter.toc_title || chapter.resolved_title || chapter.title;
function closeMenu(restoreFocus=false) {
  const wasOpen=!elementMenu.hidden;
  elementMenu.hidden=true; actionsButton.setAttribute("aria-expanded","false");
  headingMenu.hidden=true; headingButton.setAttribute("aria-expanded","false");
  typeMenu.hidden=true; typeButton.setAttribute("aria-expanded","false");
  roleMenu.hidden=true; roleButton.setAttribute("aria-expanded","false");
  if(restoreFocus && wasOpen && menuReturnFocus?.isConnected) menuReturnFocus.focus({preventScroll:true});
}
function openMenu(x,y) {
  if((!selected && selectedPage === null) || !state) return;
   menuReturnFocus=document.activeElement;
  menuScrollPosition=Object.fromEntries(Object.entries(panes).map(([name,pane])=>[name,pane.scrollTop]));
  const element=elementIndex(state.working).get(selected)?.element;
  const acceptedElement=elementIndex(state.accepted).get(selected)?.element;
  const typeLabel=document.getElementById("menu-current-type");
  typeLabel.hidden=!element && !acceptedElement;
  const types=acceptedElement && element && acceptedElement.type !== element.type ?
    `${acceptedElement.type} / ${element.type}` : element?.type ?? acceptedElement?.type;
  typeLabel.textContent=types ? `Type: ${types}` : "Type:";
  const roleLabel=document.getElementById("menu-current-role");
  roleLabel.hidden=!element && !acceptedElement;
  const roleName=value=>value ?? "None";
  const roles=acceptedElement && element && acceptedElement.role !== element.role ?
    `${roleName(acceptedElement.role)} / ${roleName(element.role)}` : roleName((element ?? acceptedElement)?.role);
  roleLabel.textContent=`Role: ${roles}`;
  const editable=element && !state.pending && !loading;
  document.getElementById("menu-codex").hidden=!element;
  document.getElementById("menu-codex").disabled=!editable || codexRunning;
  document.getElementById("menu-codex-working").disabled=!state.authored_revision;
  const newChapterButton=document.getElementById("menu-new-chapter");
  newChapterButton.hidden=!element;
  newChapterButton.disabled=!editable || elementIndex(state.working).get(element?.id)?.chapter.elements[0]?.id === element?.id;
  newChapterButton.title="Start a chapter at this element and move the rest of its current chapter into it";
  menuCommand=element ? {id:element.id,expected_revision:state.authored_revision,
    accepted_revision:state.accepted?.revision,visible:element.visible === false} : null;
  visibilityButton.hidden=!element; visibilityButton.disabled=!editable;
  visibilityButton.textContent=element?.visible === false ? "Unhide Element" : "Hide Element";
  pageCommand={page:selectedPage,expected_revision:state.authored_revision};
  hidePageButton.disabled=selectedPage === null || state.pending || loading ||
    ![...elementIndex(state.working).values()].some(({element})=>element.visible !== false &&
      element.pdf_regions.some(region=>region.page === selectedPage));
  hidePageButton.title=selectedPage === null ? "This element has no PDF source page" :
    `Hide all Working elements from PDF page ${selectedPage}`;
  document.getElementById("menu-copy").disabled=!selected || selected === "generated-toc";
  headingButton.hidden=!element || !state.heading_types.includes(element.type);
  headingButton.disabled=!editable;
  headingButton.textContent=element?.heading_level ? `Heading H${element.heading_level} ▸` : "Heading ▸";
  typeButton.hidden=!element;
  const imageTarget=element?.type === "table" && !element.asset && element.pdf_regions.length > 0;
  typeButton.disabled=!editable || (!imageTarget && (state.protected_types.includes(element.type) || !!element.asset ||
    typeof element.md_text !== "string" || !element.md_text.trim()));
  typeMenu.replaceChildren();
  for(const kind of imageTarget ? ["graphic"] : state.editable_types) {
    const button=document.createElement("button");button.type="button";button.dataset.type=kind;
    button.textContent=kind === "graphic" ? "Image" : kind;button.setAttribute("role","menuitemradio");
    button.setAttribute("aria-checked",String(kind === element?.type));
    button.disabled=typeButton.disabled || kind === element?.type;
    button.onclick=async()=>{
      const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision,type:kind};
      closeMenu(true);
      try{await post("/api/type",command);await refresh();}
      catch(error){await refresh();showError(error.message);}
    };
    typeMenu.append(button);
  }
  roleButton.hidden=!element;
  roleButton.disabled=!editable;
  roleMenu.replaceChildren();
  for(const role of state.roles) {
    const button=document.createElement("button");button.type="button";
    button.textContent=roleName(role);button.setAttribute("role","menuitemradio");
    button.setAttribute("aria-checked",String(role === element?.role));
    button.disabled=roleButton.disabled || role === element?.role;
    button.onclick=async()=>{
      const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision,role};
      closeMenu(true);
      try{await post("/api/role",command);await refresh();}
      catch(error){await refresh();showError(error.message);}
    };
    roleMenu.append(button);
  }
  for(const id of ["menu-bold","menu-remove-bold","menu-markdown","menu-split"]) {
    const button=document.getElementById(id);
    button.hidden=!element || typeof element.md_text !== "string" || !!element.asset;
    button.disabled=!editable;
  }
  const next=nextElement(element?.id);
  const duplicateButton=document.getElementById("menu-duplicate");
  duplicateButton.hidden=!element;
  duplicateButton.disabled=!editable;
  const mergeButton=document.getElementById("menu-merge");
  mergeButton.hidden=!element || typeof element.md_text !== "string" || !!element.asset;
  mergeButton.disabled=!editable || !next || typeof next.md_text !== "string" || !!next.asset;
  const mergeGraphicsButton=document.getElementById("menu-merge-graphics");
  mergeGraphicsButton.hidden=!element || element.type !== "graphic";
  mergeGraphicsButton.disabled=!editable || !next || next.type !== "graphic" ||
    !!next.md_text || !element.pdf_regions.length || !next.pdf_regions.length ||
    [...element.pdf_regions,...next.pdf_regions].some(region=>region.page !== element.pdf_regions[0].page);
  const mergeTablesButton=document.getElementById("menu-merge-tables");
  mergeTablesButton.hidden=!element || element.type !== "table";
  mergeTablesButton.disabled=!editable;
  for(const [id,offset] of [["menu-move-up",-1],["menu-move-down",1]]) {
    const button=document.getElementById(id);
    button.hidden=!element;
    button.disabled=!editable || !adjacentElement(element?.id,offset);
  }
  for(const button of headingMenu.children)button.setAttribute("aria-checked",String(Number(button.dataset.level) === element?.heading_level));
  discardElementButton.hidden=!element;
  discardElementButton.disabled=!editable || !state.discardable_elements?.includes(element.id);
  discardElementButton.title="Restore this element's text and properties from Accepted";
  elementMenu.hidden=false; actionsButton.setAttribute("aria-expanded","true");
  const rect=elementMenu.getBoundingClientRect();
  elementMenu.style.left=`${Math.max(8,Math.min(x,innerWidth-rect.width-8))}px`;
  elementMenu.style.top=`${Math.max(8,Math.min(y,innerHeight-rect.height-8))}px`;
  elementMenu.querySelector("button:not([hidden]):not(:disabled)")?.focus({preventScroll:true});
}
actionsButton.onclick=()=>{
  if(!elementMenu.hidden){closeMenu(true);return;}
  const rect=actionsButton.getBoundingClientRect();openMenu(rect.left,rect.bottom+4);
};
document.addEventListener("pointerdown",event=>{
  if(!event.composedPath().includes(elementMenu) && !event.composedPath().includes(actionsButton))closeMenu();
});
elementMenu.addEventListener("keydown",event=>{
  const buttons=[...elementMenu.querySelectorAll("button:not([hidden]):not(:disabled)")].filter(button=>!button.closest("[hidden]"));
  const index=buttons.indexOf(document.activeElement);
  if(["ArrowDown","ArrowUp","Home","End"].includes(event.key)) {
    event.preventDefault();
    const next=event.key === "Home" ? 0 : event.key === "End" ? buttons.length-1 :
      (index+(event.key === "ArrowDown" ? 1 : -1)+buttons.length)%buttons.length;
    buttons[next]?.focus();
  } else if(event.key === "ArrowRight" && [headingButton,typeButton,roleButton].includes(document.activeElement)) {
    const button=document.activeElement, menu=button === headingButton ? headingMenu : button === typeButton ? typeMenu : roleMenu;
    event.preventDefault();if(menu.hidden)button.click();menu.querySelector("button:not(:disabled)")?.focus();
  } else if(event.key === "ArrowLeft" && [headingMenu,typeMenu,roleMenu].some(menu=>menu.contains(document.activeElement))) {
    const menu=headingMenu.contains(document.activeElement) ? headingMenu : typeMenu.contains(document.activeElement) ? typeMenu : roleMenu;
    const button=menu === headingMenu ? headingButton : menu === typeMenu ? typeButton : roleButton;
    event.preventDefault();menu.hidden=true;button.setAttribute("aria-expanded","false");button.focus();
  } else if(event.key === "Tab") closeMenu();
});
function clearSelection() {
  selected=null;selectedPage=null;hovered=null;
  let focused=document.activeElement;
  while(focused?.shadowRoot?.activeElement)focused=focused.shadowRoot.activeElement;
  if(focused?.matches("[data-id],[data-element],.page"))focused.blur();
  closeMenu();highlight();
}
document.addEventListener("keydown",event=>{if(event.key === "Escape")clearSelection();});
document.addEventListener("click",event=>{
  if(event.composedPath().some(node=>node.matches?.(
    "[data-id],[data-element],a,summary,button,input,select,textarea,#element-menu,dialog")))return;
  if(!getSelection().isCollapsed)return;
  clearSelection();
});
visibilityButton.onclick=async()=>{
  const command=menuCommand;closeMenu(true);
  try{await post("/api/visibility",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
hidePageButton.onclick=async()=>{
  const command=pageCommand;closeMenu(true);
  try{await post("/api/hide-page",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
discardElementButton.onclick=async()=>{
  const command=menuCommand;closeMenu(true);
  try{await post("/api/discard-element",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
headingButton.onclick=()=>{
  roleMenu.hidden=true;roleButton.setAttribute("aria-expanded","false");
  typeMenu.hidden=true;typeButton.setAttribute("aria-expanded","false");
  headingMenu.hidden=!headingMenu.hidden;headingButton.setAttribute("aria-expanded",String(!headingMenu.hidden));
  const rect=elementMenu.getBoundingClientRect();
  elementMenu.style.top=`${Math.max(8,Math.min(rect.top,innerHeight-rect.height-8))}px`;
};
typeButton.onclick=()=>{
  roleMenu.hidden=true;roleButton.setAttribute("aria-expanded","false");
  headingMenu.hidden=true;headingButton.setAttribute("aria-expanded","false");
  typeMenu.hidden=!typeMenu.hidden;typeButton.setAttribute("aria-expanded",String(!typeMenu.hidden));
  const rect=elementMenu.getBoundingClientRect();
  elementMenu.style.top=`${Math.max(8,Math.min(rect.top,innerHeight-rect.height-8))}px`;
};
roleButton.onclick=()=>{
  headingMenu.hidden=true;headingButton.setAttribute("aria-expanded","false");
  typeMenu.hidden=true;typeButton.setAttribute("aria-expanded","false");
  roleMenu.hidden=!roleMenu.hidden;roleButton.setAttribute("aria-expanded",String(!roleMenu.hidden));
  const rect=elementMenu.getBoundingClientRect();
  elementMenu.style.top=`${Math.max(8,Math.min(rect.top,innerHeight-rect.height-8))}px`;
};
for(let level=1;level<=6;level++) {
  const button=document.createElement("button");button.type="button";button.dataset.level=level;
  button.textContent=`H${level}`;button.setAttribute("role","menuitemradio");
  button.onclick=async()=>{
    const command={...menuCommand,level};closeMenu(true);
    try{await post("/api/heading",command);await refresh();}
    catch(error){await refresh();showError(error.message);}
  };
  headingMenu.append(button);
}
for(const id of ["menu-markdown","menu-split"])document.getElementById(id).onclick=()=>{
  const element=elementIndex(state.working).get(menuCommand.id)?.element;
  textCommand={id:menuCommand.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);if(element)editMarkdown(element,id === "menu-split");
};
for(const id of ["menu-bold","menu-remove-bold"])document.getElementById(id).onclick=async()=>{
  const element=elementIndex(state.working).get(menuCommand.id)?.element;
  const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision,
    text:boldText(element.resolved_md_text ?? element.md_text,id === "menu-bold")};
  closeMenu(true);
  try{await post("/api/markdown",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
function adjacentElement(id,offset) {
  const ordered=[...elementIndex(state?.working).values()].map(entry=>entry.element);
  const index=ordered.findIndex(element=>element.id === id);
  return index < 0 ? null : ordered[index+offset];
}
function nextElement(id) {return adjacentElement(id,1);}

document.getElementById("menu-duplicate").onclick=async()=>{
  const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);
  try{await post("/api/duplicate",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};

document.getElementById("menu-merge-tables").onclick=async()=>{
  const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);
  try{await post("/api/merge-tables",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};

document.getElementById("menu-merge-graphics").onclick=async()=>{
  const next=nextElement(menuCommand.id);
  if(!next)return;
  const command={id:menuCommand.id,next_id:next.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);
  try{await post("/api/merge-graphics",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};

for(const direction of ["up","down"])document.getElementById(`menu-move-${direction}`).onclick=async()=>{
  const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision,direction};
  closeMenu(true);
  try{await post("/api/move",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
document.getElementById("menu-merge").onclick=()=>{
  const element=elementIndex(state.working).get(menuCommand.id)?.element;
  const next=nextElement(menuCommand.id);
  if(!element || !next)return;
  textCommand={id:element.id,next_id:next.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);editMarkdown(element,false,next);
};
document.getElementById("menu-copy").onclick=async()=>{
  closeMenu(true);try{await navigator.clipboard.writeText(selected);}catch(error){showError(error.message);}
};
showHidden.onchange=()=>{
  const logical=anchor(leader);closeMenu();hovered=null;highlight();
  positionTypeLabels();
  if(logical)position(leader,logical);sync(leader);
};
showTypes.onchange=()=>{
  document.body.classList.toggle("show-types",showTypes.checked);
  if(showTypes.checked) positionTypeLabels();
};
showTypes.onchange();

const pdfSplitter = document.getElementById("pdf-splitter");
const pdfWidthKey = "book-review.pdf-width";
const pdfPageKey = "book-review.pdf-page";
const minimumPdfWidth = 240;
let pdfWidth = 600, pdfDrag = null;
let pdfPageRestored = false;
try {
  const saved = Number(localStorage.getItem(pdfWidthKey));
  if (Number.isFinite(saved) && saved >= minimumPdfWidth) pdfWidth = saved;
} catch (_) { /* The layout remains usable when browser storage is unavailable. */ }
function setPdfWidth(width) {
  pdfWidth = Math.max(minimumPdfWidth, Math.round(width));
  document.documentElement.style.setProperty("--pdf-width", `${pdfWidth}px`);
  pdfSplitter.setAttribute("aria-valuemin", minimumPdfWidth);
  pdfSplitter.setAttribute("aria-valuenow", pdfWidth);
  pdfSplitter.setAttribute("aria-valuetext", `${pdfWidth} pixels`);
}
function savePdfWidth() {
  try { localStorage.setItem(pdfWidthKey, String(pdfWidth)); }
  catch (_) { /* Keep the selected width for this session. */ }
}
setPdfWidth(pdfWidth);
pdfSplitter.addEventListener("pointerdown", event => {
  if (event.button !== 0 || pdfDrag) return;
  pdfDrag = {pointerId: event.pointerId, x: event.clientX, width: pdfWidth};
  pdfSplitter.setPointerCapture(event.pointerId);
  document.body.classList.add("resizing-pdf");
  event.preventDefault();
});
pdfSplitter.addEventListener("pointermove", event => {
  if (pdfDrag?.pointerId !== event.pointerId) return;
  setPdfWidth(pdfDrag.width + event.clientX - pdfDrag.x);
});
function finishPdfDrag(event) {
  if (pdfDrag?.pointerId !== event.pointerId) return;
  pdfDrag = null;
  document.body.classList.remove("resizing-pdf");
  savePdfWidth();
}
for (const type of ["pointerup", "pointercancel", "lostpointercapture"]) {
  pdfSplitter.addEventListener(type, finishPdfDrag);
}
pdfSplitter.addEventListener("keydown", event => {
  if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
  setPdfWidth(pdfWidth + (event.key === "ArrowRight" ? 10 : -10));
  savePdfWidth();
  event.preventDefault();
});

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
function typeLabel(element) {
  const type=element.type === "heading" ? `heading ${element.heading_level}` : element.type.replaceAll("_"," ");
  return element.role ? `${type} · ${element.role}` : type;
}
function overlay(holder, id, bbox, width, height, pane, source = null) {
  if (height <= 0) return;
  const node = document.createElement("div"); node.className = "overlay";
  node.dataset.id = id;
  if(source) node.dataset.typeLabel=typeLabel(source.element);
  boxStyle(node,bbox,width,height); holder.append(node);
  nodes[pane].push({id, node, source});
  bindElement(node,id,pane);
}
function positionTypeLabels() {
  if(!showTypes.checked)return;
  typeLabelContext.font='600 11px "Segoe UI"';
  for(const {node} of nodes.pdf) {
    const page=node.closest(".page").getBoundingClientRect(), box=node.getBoundingClientRect();
    const right=page.right-box.right-3, left=box.left-page.left-3;
    const needed=typeLabelContext.measureText(node.dataset.typeLabel).width+10;
    const below=Math.max(left,right)<24;
    const useLeft=!below && right<needed && left>right;
    node.classList.toggle("type-label-left",useLeft);
    node.classList.toggle("type-label-below",below);
    node.style.setProperty("--type-label-space",`${Math.max(0,below ? page.width : useLeft ? left : right)}px`);
  }
}
function elementPage(node, id, pane, y) {
  if(pane === "pdf") return Number(node.closest(".page").dataset.page);
  const element=elementIndex(state?.[pane]).get(id)?.element;
  const regions=element?.pdf_regions || [];
  if(!regions.length)return null;
  const rect=node.getBoundingClientRect();
  const progress=Math.max(0,Math.min(1,(y-rect.top)/Math.max(1,rect.height)));
  const heights=regions.map(region=>Math.max(0,region.bbox[3]-region.bbox[1]));
  let distance=progress*heights.reduce((a,b)=>a+b,0);
  for(let i=0;i<regions.length;i++) {
    if(distance<=heights[i])return regions[i].page;
    distance-=heights[i];
  }
  return regions[regions.length-1].page;
}
function bindElement(node, id, pane) {
  node.dataset.id = id; node.tabIndex = 0;
  node.addEventListener("pointerenter", () => { hovered = id; highlight(); });
  node.addEventListener("pointerleave", () => { hovered = null; highlight(); });
  node.addEventListener("contextmenu", event => {
    event.preventDefault(); event.stopPropagation();
    selected=id; selectedPane=pane; selectedPage=elementPage(node,id,pane,event.clientY); highlight(); openMenu(event.clientX,event.clientY);
  });
  node.addEventListener("keydown", event => {
    if (event.target !== node) return;
    if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10") || event.key === "Enter") {
      event.preventDefault(); event.stopPropagation(); selected=id; selectedPane=pane; highlight();
      const rect=node.getBoundingClientRect(); selectedPage=elementPage(node,id,pane,rect.top); openMenu(rect.left,rect.top);
    }
  });
  node.addEventListener("click", event => {
    if (event.target.closest("a,summary,button,input,select,textarea")) return;
    // A drag selection copies ordinary document text without opening an edit menu.
    if (!getSelection().isCollapsed) return;
    event.stopPropagation();
    if(selected === id){clearSelection();return;}
    selected = id; highlight();
    selectedPane = pane; selectedPage=elementPage(node,id,pane,event.clientY); closeMenu();
  });
}
async function buildMarkdown(version, snapshot) {
  const index = elementIndex(snapshot), entries = [];
  const stage = document.createElement("div"); stage.className = "render-stage";
  stage.style.width = `${panes[version].clientWidth}px`;
  document.body.append(stage);
  const ready = [], styles = [];
  const waitFor = promise => {
    // A later chapter fetch may still be pending when an earlier asset fails.
    promise.catch(()=>{}); ready.push(promise);
  };
  try {
  for (const layout of snapshot?.render?.chapters || []) {
    if (!layout.document) throw new Error("Preparing the HTML presentation; waiting for a completed revision");
    const response = await fetch(`/files/${version}/${layout.document}`);
    if (!response.ok) throw new Error(`Cannot load ${version} chapter ${layout.id}`);
    const markup = await response.text();
    const chapter = snapshot.chapters.find(c => c.id === layout.id);
    const section = document.createElement("div"); section.dataset.chapter = layout.id;
    const content = document.createElement("div"); content.className = "chapter-content";
    if (chapter) {
      const toggle = document.createElement("button"); toggle.className = "chapter-toggle";
      toggle.textContent = `${collapsed.has(layout.id) ? "▸" : "▾"} ${chapterTitle(chapter)}`;
      toggle.addEventListener("click", () => {
        if (collapsed.has(layout.id)) collapsed.delete(layout.id); else collapsed.add(layout.id);
        applyCollapse(layout.id); sync(leader);
      });
      section.append(toggle);
    }
    // Shadow DOM isolates book typography/tables from the application's controls.
    const shadow = content.attachShadow({mode:"open"});
    for (const url of [new URL("document.css", response.url).href, "/katex/katex.min.css"]) {
      const link = document.createElement("link"); link.rel = "stylesheet"; link.href = url;
      waitFor(new Promise((resolve,reject) => {
        link.onload = resolve; link.onerror = () => reject(new Error(`Cannot load document styles: ${url}`));
      }));
      styles.push(link);shadow.append(link);
    }
    const documentRoot = document.createElement("div"); documentRoot.className = "document";
    documentRoot.innerHTML = markup;
    for (const image of documentRoot.querySelectorAll("img")) {
      image.src = new URL(image.getAttribute("src"),response.url).href;
      waitFor(image.decode());
    }
    for (const node of documentRoot.querySelectorAll("[data-element]")) {
      const id = node.dataset.element;
      if (index.has(id) || id === "generated-toc") {
        bindElement(node,id,version); entries.push({id,node,source:null,element:index.get(id)?.element});
      }
    }
    for (const link of documentRoot.querySelectorAll("a[href]")) {
      const href = link.getAttribute("href"), hash = href.split("#")[1];
      const target = link.dataset.target || (hash?.startsWith("element-") ? hash.slice(8) : null);
      if (target && index.has(target)) {
        link.addEventListener("click",event => {event.preventDefault();navigate(target,version);});
      } else if (href.startsWith("#")) {
        link.addEventListener("click",event => {
          event.preventDefault();
          const targetNode = shadow.getElementById(href.slice(1));
          targetNode?.scrollIntoView({block:"center"});
        });
      } else {
        link.href = new URL(href,response.url).href; link.target = "_blank"; link.rel = "noopener";
      }
    }
    documentRoot.addEventListener("toggle",()=>{
      if(!panes[version].contains(content))return;
      leader=version;sync(version);
    },true);
    shadow.append(documentRoot);
    section.append(content); stage.append(section);
  }
  await Promise.all(ready);
  await document.fonts.ready;
  return {stage,entries,styles};
  } catch(error) {stage.remove();throw error;}
}
function buildPdf() {
  const fragment = document.createDocumentFragment();
  const mappings = [];
  const visibleWorkingPages = new Set();
  for (const {element} of elementIndex(state.working).values()) {
    for (const region of element.pdf_regions) {
      if(element.visible !== false) visibleWorkingPages.add(region.page);
    }
  }
  for (const version of ["working","accepted"]) for (const {element} of elementIndex(state[version]).values()) {
    element.pdf_regions.forEach((region,i) => mappings.push({element,region,i,version}));
  }
  mappings.sort((a,b) => Number(a.version !== "working")-Number(b.version !== "working"));
  for (const page of state.pdf_pages) {
    const holder = imageHolder(`/files/${page.image}`,page.width,page.height,"page");
    holder.dataset.page = page.page;
    holder.tabIndex=0;
    const pageMenu=(x,y)=>{
      selected=null;hovered=null;selectedPane="pdf";selectedPage=page.page;
      highlight();openMenu(x,y);
    };
    holder.addEventListener("contextmenu",event=>{
      event.preventDefault();pageMenu(event.clientX,event.clientY);
    });
    holder.addEventListener("keydown",event=>{
      if(event.target!==holder)return;
      if(event.key === "ContextMenu" || (event.shiftKey && event.key === "F10") || event.key === "Enter") {
        event.preventDefault();const rect=holder.getBoundingClientRect();pageMenu(rect.left,rect.top);
      }
    });
    holder.dataset.allHidden = String(!visibleWorkingPages.has(page.page));
    holder.hidden=holder.dataset.allHidden === "true" && !showHidden.checked;
    const seen = new Set();
    for (const item of mappings.filter(m => m.region.page === page.page)) {
      const key = `${item.element.id}:${JSON.stringify(item.region.bbox)}`;
      if (seen.has(key)) continue; seen.add(key);
      overlay(holder,item.element.id,item.region.bbox,page.width,page.height,"pdf",item);
    }
    fragment.append(holder);
  }
  panes.pdf.replaceChildren(fragment);
  positionTypeLabels();
}
function applyCollapse(id) {
  for (const version of ["accepted","working"]) {
    for (const section of panes[version].querySelectorAll("[data-chapter]")) {
      if (section.dataset.chapter !== id || id === "toc") continue;
      section.querySelector(".chapter-content").hidden = collapsed.has(id);
      const chapter = state[version]?.chapters.find(c => c.id === id);
      section.querySelector("button").textContent = `${collapsed.has(id) ? "▸" : "▾"} ${chapterTitle(chapter)}`;
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
  return nodes[pane].filter(entry => (!id || entry.id === id) && entry.node.getClientRects().length && getComputedStyle(entry.node).visibility !== "hidden");
}
function highlight() {
  const changed = new Set((state?.changes || []).map(c => c.id));
  const changedChapters = new Set((state?.changes || []).flatMap(c=>c.chapter_ids || []));
  for(const version of ["accepted","working"]) {
    for(const section of panes[version].querySelectorAll("[data-chapter]")) {
      const toggle=section.querySelector(".chapter-toggle");
      if(toggle)toggle.classList.toggle("changed",changedChapters.has(section.dataset.chapter));
    }
  }
  for (const entries of Object.values(nodes)) for (const entry of entries) {
    const hidden=(entry.source?.element || entry.element)?.visible === false;
    entry.node.classList.toggle("hidden-element",hidden);
    if(entry.source) entry.node.style.visibility=hidden && !showHidden.checked ? "hidden" : "visible";
    else entry.node.hidden=hidden && !showHidden.checked;
    entry.node.classList.toggle("selected",entry.id === selected);
    entry.node.classList.toggle("hover",entry.id === hovered);
    entry.node.classList.toggle("changed",changed.has(entry.id));
  }
  for (const page of panes.pdf.querySelectorAll(".page")) {
    page.hidden=page.dataset.allHidden === "true" && !showHidden.checked;
  }
  document.getElementById("copy").disabled = !selected || selected === "generated-toc";
  actionsButton.disabled = !selected && selectedPage === null;
  updateChangeNavigation();
}
function anchor(pane) {
  const viewport = panes[pane].getBoundingClientRect(), line = viewport.top + .25*panes[pane].clientHeight;
  let candidates = visibleNodes(pane);
  if (pane === "pdf") {
    candidates = candidates.filter(e => showHidden.checked || e.source.element.visible !== false);
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
function headingPath(version, id, depth) {
  if(id === "generated-toc")return "Table of contents";
  const chapter=elementIndex(state[version]).get(id)?.chapter;
  if(!chapter)return "";
  const entries=new Map(nodes[version].map(entry=>[entry.id,entry.node]));
  const path=[];
  for(const element of chapter.elements) {
    if(element.visible !== false && state.heading_types.includes(element.type)) {
      const level=element.heading_level || 1;
      while(path.length && path[path.length-1].level>=level)path.pop();
      const title=entries.get(element.id)?.textContent.replace(/\s+/g," ").trim();
      if(title)path.push({level,title});
    }
    if(element.id === id)break;
  }
  return path.filter(heading=>heading.level<=depth).map(heading=>heading.title).join(" › ");
}
let locationFrame=null;
function scheduleLocations() {
  if(locationFrame!==null)return;
  locationFrame=requestAnimationFrame(()=>{
    locationFrame=null;
    if(!state)return;
    const viewport=panes.pdf.getBoundingClientRect(), line=viewport.top+.25*panes.pdf.clientHeight;
    const pages=[...panes.pdf.querySelectorAll(".page")].filter(page=>!page.hidden);
    const distance=page=>{
      const rect=page.getBoundingClientRect();return Math.max(rect.top-line,line-rect.bottom,0);
    };
    pages.sort((a,b)=>distance(a)-distance(b));
    document.getElementById("pdf-location").textContent=pages.length ? `Page ${pages[0].dataset.page}` : "";
    if(pdfPageRestored && pages.length) {
      try { localStorage.setItem(pdfPageKey,pages[0].dataset.page); }
      catch (_) { /* Keep navigation usable when browser storage is unavailable. */ }
    }
    const depth=Number(document.getElementById("toc-depth").value);
    for(const version of ["accepted","working"]) {
      const logical=anchor(version);
      document.getElementById(`${version}-location`).textContent=logical ? headingPath(version,logical.id,depth) : "";
    }
  });
}
function restorePdfPage() {
  if(pdfPageRestored)return;
  pdfPageRestored=true;
  let saved;
  try { saved=Number(localStorage.getItem(pdfPageKey)); }
  catch (_) { return; }
  if(!Number.isInteger(saved) || saved<1)return;
  const page=[...panes.pdf.querySelectorAll(".page")].find(node=>Number(node.dataset.page)===saved && !node.hidden);
  if(!page)return;
  const viewport=panes.pdf.getBoundingClientRect();
  panes.pdf.scrollTop+=page.getBoundingClientRect().top-viewport.top;
  leader="pdf";lastLogical=anchor("pdf");sync("pdf");
}
function position(pane, logical, fallback = true) {
  if (!logical) return;
  if(logical.id === "generated-toc" && pane === "pdf") return;
  let counterparts = visibleNodes(pane,logical.id), progress = logical.progress;
  if (!counterparts.length && fallback) {
    const snapshot = state[leader] || state.working;
    const order = logical.order || [...elementIndex(snapshot).keys()], index=order.indexOf(logical.id);
    for(const candidate of [...order.slice(index+1),...order.slice(0,index).reverse()]) {
      // Prefer the following content when the reading anchor was hidden.
      if(candidate && elementIndex(state.working).has(candidate) && elementIndex(state.accepted).has(candidate)) {
        counterparts=visibleNodes(pane,candidate); if(counterparts.length) {progress=0;break;}
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
  return {id:entry.id,progress,order:logical.order};
}
function sync(pane) {
  const logical=anchor(pane);
  if(pane===leader)lastLogical=logical;
  for(const follower of Object.keys(panes)) if(follower!==pane) position(follower,logical);
  scheduleLocations();
}
function navigate(id, preferred="working") {
  selectedPane=preferred;selectedPage=(elementIndex(state[preferred]).get(id) || elementIndex(state.accepted).get(id))?.element.pdf_regions[0]?.page ?? null;closeMenu();
  expand(id); selected=id; highlight();
  leader=visibleNodes(preferred,id).length ? preferred : visibleNodes("accepted",id).length ? "accepted" : "pdf";
  const snapshot=elementIndex(state.working).has(id) ? state.working : state.accepted;
  const logical={id,progress:0,order:[...elementIndex(snapshot).keys()]};
  for(const pane of Object.keys(panes)) position(pane,logical);
  lastLogical=logical;
  scheduleLocations();
}
async function post(url, command) {
  const response=await fetch(url,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(command)});
  if(response.status === 404) {
    throw new Error(`Server does not support ${url}. Restart Book Review server and reload this page.`);
  }
  const result=await response.json(); if(!response.ok) throw new Error(result.error);
  return result;
}
function showError(message) {
  const status=document.getElementById("status");status.textContent=message;status.classList.add("error");
}
function editMarkdown(element,split,next=null) {
  splitting=split;merging=!!next;activeText=markdownText;
  markdownText.value=(element.resolved_md_text ?? element.md_text)+(next ? `\n${next.resolved_md_text ?? next.md_text}` : "");markdownSecond.value="";
  document.getElementById("markdown-title").textContent=merging ? "Merge with next" : split ? "Split section" : "Edit Markdown";
  document.getElementById("markdown-id").textContent=merging ? `${element.id} + ${next.id}` : element.id;
  document.getElementById("markdown-help").textContent=merging ?
    "Edit the combined Markdown, then press OK. The merged element keeps the PDF pages and source boxes of both elements." : split ?
    "Place the cursor at the split, or select the opening text and split after it. Review and edit both sections, then press OK." :
    "Edit the Markdown source. Select text to add or remove bold; without a selection, the whole section is used.";
  document.getElementById("split-at-cursor").hidden=!split;
  document.getElementById("second-section").hidden=true;
  document.getElementById("first-label").textContent=split ? "First section" : "Markdown";
  document.getElementById("save-markdown").textContent=split || merging ? "OK" : "Save";
  document.getElementById("save-markdown").disabled=split;
  document.getElementById("markdown-error").hidden=true;
  markdownDialog.showModal();markdownText.focus();markdownText.setSelectionRange(0,0);
}
for(const field of [markdownText,markdownSecond])field.addEventListener("focus",()=>{activeText=field;});
function formatBold(add) {
  const field=activeText;
  let start=field.selectionStart,end=field.selectionEnd;
  if(start === end){start=0;end=field.value.length;}
  const marker=field.value.slice(start-2,start);
  if(["**","__"].includes(marker) && field.value.slice(end,end+2) === marker) {
    if(add)return;
    start-=2;end+=2;
  }
  field.setRangeText(boldText(field.value.slice(start,end),add),start,end,"select");field.focus();
}
function boldText(text,add) {
  return add ? text.split(/(\n[ \t]*\n)/).map(part=>part.replace(/^(#{1,6}[ \t]+)?(\s*)([\s\S]*?)(\s*)$/,(_,heading,leading,body,trailing)=>
    `${heading || ""}${leading}${!body || (body.startsWith("**") && body.endsWith("**")) || (body.startsWith("__") && body.endsWith("__")) ? body : `**${boldText(body,false)}**`}${trailing}`)).join("") :
    text.replace(/(`+)([\s\S]*?)\1|\*\*([\s\S]+?)\*\*|__([\s\S]+?)__|<(strong|b)\b[^>]*>([\s\S]*?)<\/\5>/gi,
      (match,code,ticks,stars,underscores,tag,html)=>code ? match : stars ?? underscores ?? html);
}
document.getElementById("add-bold").onclick=()=>formatBold(true);
document.getElementById("remove-bold").onclick=()=>formatBold(false);
document.getElementById("split-at-cursor").onclick=()=>{
  const cut=markdownText.selectionEnd;
  if(!markdownText.value.slice(0,cut).trim() || !markdownText.value.slice(cut).trim()) {
    const error=document.getElementById("markdown-error");error.textContent="Choose a split with text on both sides.";error.hidden=false;return;
  }
  markdownSecond.value=markdownText.value.slice(cut);markdownText.value=markdownText.value.slice(0,cut);
  document.getElementById("second-section").hidden=false;
  document.getElementById("split-at-cursor").hidden=true;
  document.getElementById("save-markdown").disabled=false;
  document.getElementById("markdown-error").hidden=true;
};
async function refresh() {
  requested=true; if(loading) return; loading=true;
  try {
    while(requested) {
      requested=false;
      const response=await fetch("/api/status"); if(!response.ok) throw new Error("Cannot load review state");
      const next=await response.json();
      const versions=["accepted","working"].filter(version=>!state || next[version]?.revision!==state[version]?.revision);
      const changed=versions.length>0;
      const pdfView=value=>[value.pdf_pages,...["working","accepted"].map(version=>
        [...elementIndex(value[version]).values()].map(({element:e})=>
          [e.id,e.visible!==false,e.pdf_regions,e.type,e.heading_level,e.role]))];
      const pdfChanged=!state || JSON.stringify(pdfView(state))!==JSON.stringify(pdfView(next));
      let prepared = [];
      if(changed) {
        document.getElementById("accept").disabled=true;
        document.getElementById("discard").disabled=true;
        document.getElementById("status").textContent="Loading HTML presentation…";
        try {
          // Prepare changed views before publishing a complete revision.
          for(const version of versions)prepared.push({version,...await buildMarkdown(version,next[version])});
          const logical=state ? anchor(leader) : null;
          publishing=true;
          state=next;
          for(const item of prepared) {
            nodes[item.version]=item.entries;
            panes[item.version].replaceChildren(...item.stage.children);
          }
          for(const id of collapsed)applyCollapse(id);
          // Moving Shadow DOM between containers reloads linked stylesheets.
          // Restore geometry only after the mounted documents have their styles.
          await Promise.all(prepared.flatMap(item=>item.styles).map(link=>link.sheet ? Promise.resolve() :
            new Promise((resolve,reject)=>{
              link.addEventListener("load",resolve,{once:true});
              link.addEventListener("error",()=>reject(new Error(`Cannot load document styles: ${link.href}`)),{once:true});
            })));
          await document.fonts.ready;
          if(pdfChanged){nodes.pdf=[];buildPdf();}
          else {
            const indexes={working:elementIndex(state.working),accepted:elementIndex(state.accepted)};
            for(const entry of nodes.pdf)if(entry.source) {
              entry.source.element=indexes[entry.source.version].get(entry.id).element;
            }
          }
          highlight();
          if(!pdfPageRestored)restorePdfPage();
          if(logical) {
            const restored=position(leader,logical) || logical;
            lastLogical=restored;
            for(const follower of Object.keys(panes)) if(follower!==leader)position(follower,restored);
          }
          else lastLogical=anchor(leader);
        } finally {publishing=false;for(const item of prepared) item.stage.remove();}
        const chapter=document.getElementById("chapter"), old=chapter.value;chapter.replaceChildren(new Option("Choose…",""));
        const known=new Set();
        for(const snapshot of [state.working,state.accepted]) for(const c of snapshot?.chapters || []) {
          if(!known.has(c.id)){known.add(c.id);chapter.append(new Option(chapterTitle(c),c.id));}
        }
        chapter.value=old;
      } else state=next;
      document.getElementById("accept").disabled=!state.can_accept;
      document.getElementById("discard").disabled=!state.can_discard;
      document.getElementById("codex-working").disabled=!state.authored_revision;
      updateCodexStatus();
      document.getElementById("toc-depth").value=state.working?.toc.depth ?? 2;
      const status=document.getElementById("status");status.classList.toggle("error",Boolean(state.error));
      status.textContent=state.error || (state.pending ? "Rendering pending…" : `${state.changes.length} ${state.changes.length===1 ? "change" : "changes"}`);
      closeMenu();highlight();
    }
  } catch(error) {showError(error.message);} finally {loading=false;scheduleLocations();}
}
for(const [name,pane] of Object.entries(panes)) {
  pane.addEventListener("pointermove",event=>{pointerX=event.clientX;});
  for(const type of ["wheel","touchstart","pointerdown","keydown"]) pane.addEventListener(type,()=>{
    leader=name;suppressed.delete(name);if(state)lastLogical=anchor(name);
  },{passive:true});
  pane.addEventListener("scroll",()=>{
    scheduleLocations();
    if(publishing)return;
    // A scroll queued before opening the menu must not close that new menu.
    if(elementMenu.hidden || pane.scrollTop !== menuScrollPosition[name])closeMenu();

    if(suppressed.has(name) && Math.abs(pane.scrollTop-suppressed.get(name))<1) {suppressed.delete(name);return;}
    suppressed.delete(name);leader=name;sync(name);
  },{passive:true});
}
document.getElementById("chapter").addEventListener("change",event=>{
  const chapter=state.working?.chapters.find(c=>c.id===event.target.value) || state.accepted?.chapters.find(c=>c.id===event.target.value);
  if(chapter) {collapsed.delete(chapter.id);applyCollapse(chapter.id);const e=chapter.elements.find(e=>e.visible!==false);if(e)navigate(e.id);}
});
function updateChangeNavigation() {
  const changes=state?.changes || [];
  changeIndex=changes.findIndex(change=>change.id===selected);
  document.getElementById("previous").disabled=!changes.length;
  document.getElementById("next").disabled=!changes.length;
}
function nextChange(delta) {
  if(!state?.changes.length)return;
  const index=changeIndex<0 ? (delta<0 ? state.changes.length-1 : 0) :
    (changeIndex+delta+state.changes.length)%state.changes.length;
  navigate(state.changes[index].id);
}
document.getElementById("previous").onclick=()=>nextChange(-1);
document.getElementById("next").onclick=()=>nextChange(1);
document.getElementById("copy").onclick=async()=>{try{await navigator.clipboard.writeText(selected);}catch(error){showError(error.message);}};
async function saveToc() {
  scheduleLocations();
  try {await post("/api/toc",{depth:Number(document.getElementById("toc-depth").value),
    expected_revision:state.authored_revision});await refresh();}
  catch(error){showError(error.message);}
}
document.getElementById("toc-depth").onchange=saveToc;
document.getElementById("accept").onclick=async()=>{try{await post("/api/accept",{revision:state.working.revision});await refresh();}catch(error){showError(error.message);}};
document.getElementById("discard").onclick=async()=>{
  const command={expected_revision:state.authored_revision,accepted_revision:state.accepted.revision};
  document.getElementById("discard").disabled=true;
  closeMenu();markdownDialog.close();
  try{await post("/api/discard",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
document.getElementById("save-markdown").onclick=async()=>{
  const button=document.getElementById("save-markdown");button.disabled=true;
  try {
    await post(merging ? "/api/merge" : splitting ? "/api/split" : "/api/markdown",{...textCommand,...(splitting ?
      {first:markdownText.value,second:markdownSecond.value} : {text:markdownText.value})});
    markdownDialog.close();await refresh();
  } catch(error) {
    const message=document.getElementById("markdown-error");message.textContent=error.message;message.hidden=false;
  } finally {button.disabled=false;}
};
document.getElementById("menu-new-chapter").onclick=async()=>{
  const command={id:menuCommand.id,expected_revision:menuCommand.expected_revision};
  closeMenu(true);
  try{await post("/api/new-chapter",command);await refresh();}
  catch(error){await refresh();showError(error.message);}
};
function openCodex(scope,id=null) {
  codexScope=scope;
  codexCommand={expected_revision:state.authored_revision,...(id ? {id} : {})};
  const preset=document.getElementById("codex-preset");
  document.getElementById("codex-preset-section").hidden=scope !== "working";
  document.getElementById("codex-format-section").hidden=scope !== "element";
  document.getElementById("codex-format-mode").value="preserve";
  updateCodexFormatGuidance();
  preset.value="";preset.disabled=false;
  document.getElementById("codex-title").textContent=scope === "working" ? "Codex Working" : "Codex on element";
  document.getElementById("codex-id").textContent=scope === "working" ?
    (id ? `Starting at ${id}; the whole Working book is available.` : "The whole Working book is available.") : id;
  document.getElementById("codex-help").textContent=scope === "working" ?
    "Codex can request more book context and edit Working. Accepted and PDF sources remain read-only." :
    "Changes are applied to Working after checking that words, numbers and substantive symbols are preserved. Dashes, commas, periods and spaces may change.";
  document.getElementById("codex-instruction").value=scope === "working" && state.codex_agent?.state === "running" ?
    state.codex_agent.instruction : "";
  document.getElementById("codex-error").hidden=true;
  document.getElementById("codex-status").hidden=true;
  document.getElementById("stop-codex-agent").hidden=true;
  document.getElementById("run-codex").disabled=false;
  document.getElementById("codex-instruction").disabled=false;
  codexDialog.showModal();updateCodexStatus();document.getElementById("codex-instruction").focus();
}
document.getElementById("menu-codex").onclick=()=>{
  const id=menuCommand.id;closeMenu(true);openCodex("element",id);
};
document.getElementById("menu-codex-working").onclick=()=>{
  const id=selected && elementIndex(state.working).has(selected) ? selected : null;
  closeMenu(true);openCodex("working",id);
};
document.getElementById("codex-working").onclick=()=>openCodex("working",
  selected && elementIndex(state.working).has(selected) ? selected : null);
function updateCodexStatus() {
  if(!codexDialog.open || codexScope !== "working")return;
  const job=state?.codex_agent;
  const running=job?.state === "running";
  const button=document.getElementById("run-codex"), stop=document.getElementById("stop-codex-agent");
  button.disabled=running || codexRunning;
  document.getElementById("codex-preset").disabled=running || codexRunning;
  document.getElementById("codex-instruction").disabled=running || codexRunning;
  stop.hidden=!running;stop.disabled=job?.action === "stopping";
  const status=document.getElementById("codex-status"), error=document.getElementById("codex-error");
  if(running){error.hidden=true;status.textContent=`Step ${job.step}: ${job.action}; ${job.edits} edits saved to Working.`;status.hidden=false;}
  else if(job?.state === "completed"){
    if(job.id === codexAutoCloseJob){codexDialog.close();return;}
    error.hidden=true;
    status.textContent=`Finished: ${job.summary} (${job.edits} edits).`;status.hidden=false;
  } else if(job?.state === "failed"){
    if(job.id === codexAutoCloseJob)codexAutoCloseJob=null;
    error.textContent=job.error;error.hidden=false;status.hidden=true;
  }
}
document.getElementById("stop-codex-agent").onclick=async()=>{
  try{await post("/api/codex-agent/stop",{id:state.codex_agent.id});await refresh();}
  catch(error){const field=document.getElementById("codex-error");field.textContent=error.message;field.hidden=false;}
};
codexDialog.addEventListener("cancel",event=>{if(codexRunning && codexScope === "element")event.preventDefault();});
codexDialog.addEventListener("close",()=>{codexAutoCloseJob=null;codexAutoClosePending=false;});
document.getElementById("codex-preset").onchange=event=>{
  if(event.target.value){
    const field=document.getElementById("codex-instruction");
    field.value=event.target.value;field.focus();
  }
};
document.getElementById("codex-instruction").oninput=event=>{
  const preset=document.getElementById("codex-preset");
  if(event.target.value !== preset.value)preset.value="";
};
function updateCodexFormatGuidance() {
  const mode=document.getElementById("codex-format-mode").value;
  document.getElementById("codex-format-guidance").value=state?.codex_format_guidance?.[mode] || "";
}
document.getElementById("codex-format-mode").onchange=updateCodexFormatGuidance;
document.getElementById("run-codex").onclick=async()=>{
  if(codexRunning)return;
  const field=document.getElementById("codex-instruction"), error=document.getElementById("codex-error");
  if(!field.value.trim()){error.textContent="Enter a Codex instruction.";error.hidden=false;field.focus();return;}
  const button=document.getElementById("run-codex"), cancel=document.getElementById("cancel-codex"), progress=document.getElementById("codex-status");
  codexAutoCloseJob=null;codexAutoClosePending=codexScope === "working";
  codexRunning=true;button.disabled=true;cancel.disabled=codexScope === "element";field.disabled=true;
  document.getElementById("codex-preset").disabled=true;error.hidden=true;
  progress.textContent=codexScope === "working" ? "Starting Codex Working task…" : "Running Codex and checking content…";progress.hidden=false;
  try {
    const result=await post(codexScope === "working" ? "/api/codex-agent" : "/api/codex",
      {...codexCommand,instruction:field.value,...(codexScope === "element" ?
        {format_mode:document.getElementById("codex-format-mode").value} : {})});
    if(codexAutoClosePending)codexAutoCloseJob=result.id;
    codexAutoClosePending=false;
    if(codexScope === "element")codexDialog.close();
    await refresh();
  } catch(failure) {
    codexAutoClosePending=false;codexAutoCloseJob=null;
    progress.hidden=true;error.textContent=failure.message;error.hidden=false;
  } finally {codexRunning=false;cancel.disabled=false;if(codexScope === "element") {button.disabled=false;field.disabled=false;} else updateCodexStatus();}
};
const events=new EventSource("/api/events");events.onmessage=refresh;events.onopen=refresh;
events.onerror=()=>showError("Connection interrupted; reconnecting…");
const sizes=new Map();
const resizeObserver=new ResizeObserver(entries=>{
  let changed=false;
  for(const entry of entries) {
    const size=`${entry.contentRect.width}:${entry.contentRect.height}`;
    if(sizes.get(entry.target)!==size) {sizes.set(entry.target,size);changed=true;}
  }
  if(!state || !changed || resizing || publishing)return;
  resizing=true;
  requestAnimationFrame(()=>{
    positionTypeLabels();
    const logical=lastLogical || anchor(leader);
    if(logical)position(leader,logical);
    sync(leader);resizing=false;
  });
});
for(const pane of Object.values(panes))resizeObserver.observe(pane);
refresh();
