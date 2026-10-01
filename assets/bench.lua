-- Pandoc filter used by `jig doc pdf` and `jig doc pack` when writing Typst.
--
-- * GitHub alerts (> [!NOTE]) become `#callout(kind)[...]`.
-- * Links jig rewrote to `#doc:<ID>` become `#doc-ref("<ID>")[...]`; other links
--   to Markdown files lose their target, since the path means nothing in a PDF.
-- * A paragraph holding only a rendered diagram (`diagram-*.svg`) becomes `#diagram(...)`.
-- * Heading ids and in-document links get a per-document prefix, so documents
--   combined into one package never share a label.

local prefix = ""
local alert_kinds = { note = true, tip = true, important = true, warning = true, caution = true }

local function typst_string(s)
  return '"' .. s:gsub('\\', '\\\\'):gsub('"', '\\"') .. '"'
end

local function read_meta(meta)
  if meta["bench-prefix"] then
    prefix = pandoc.utils.stringify(meta["bench-prefix"])
  end
end

local function header(el)
  if el.identifier ~= "" then
    el.identifier = prefix .. el.identifier
  end
  return el
end

local function link(el)
  local target = el.target
  local doc_id = target:match("^#doc:(.+)$")
  if doc_id then
    local out = { pandoc.RawInline("typst", "#doc-ref(" .. typst_string(doc_id) .. ")[") }
    for _, inline in ipairs(el.content) do
      table.insert(out, inline)
    end
    table.insert(out, pandoc.RawInline("typst", "]"))
    return out
  end
  if target:match("^#") then
    el.target = "#" .. prefix .. target:sub(2)
    return el
  end
  if target:match("%.md$") or target:match("%.md#") then
    return el.content
  end
  return el
end

local function div(el)
  for _, class in ipairs(el.classes) do
    if alert_kinds[class] then
      local out = { pandoc.RawBlock("typst", "#callout(" .. typst_string(class) .. ")[") }
      for _, block in ipairs(el.content) do
        if not (block.t == "Div" and block.classes:includes("title")) then
          table.insert(out, block)
        end
      end
      table.insert(out, pandoc.RawBlock("typst", "]"))
      return out
    end
  end
end

local function para(el)
  if #el.content == 1 and el.content[1].t == "Image" then
    local img = el.content[1]
    if img.src:match("^diagram%-.+%.svg$") then
      local alt = pandoc.utils.stringify(img.caption)
      return pandoc.RawBlock("typst", "#diagram(" .. typst_string(img.src) .. ", alt: " .. typst_string(alt) .. ")")
    end
  end
end

return {
  { Meta = read_meta },
  { Header = header, Link = link, Div = div, Para = para },
}
