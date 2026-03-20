local function has_class(classes, wanted)
  for _, class in ipairs(classes) do
    if class == wanted then
      return true
    end
  end
  return false
end

local function trim(text)
  return (text:gsub("^%s+", ""):gsub("%s+$", ""))
end

local function typst_escape_string(text)
  return text
    :gsub("\\", "\\\\")
    :gsub("\"", "\\\"")
    :gsub("\r", "")
    :gsub("\n", "\\n")
end

local function render_blocks(blocks)
  if #blocks == 0 then
    return ""
  end

  return trim(pandoc.write(pandoc.Pandoc(blocks), "typst"))
end

function Link(link)
  return link.content
end

function Div(div)
  if not has_class(div.classes, "listing") then
    return nil
  end

  local code = nil
  local caption_blocks = {}

  for _, block in ipairs(div.content) do
    if block.t == "CodeBlock" and code == nil then
      code = block
    else
      table.insert(caption_blocks, block)
    end
  end

  if code == nil then
    return div
  end

  local out = { "#book_listing(" }
  local number = div.attributes["number"]
  local file_name = div.attributes["file-name"]
  local caption = render_blocks(caption_blocks)
  local lang = code.classes[1] or ""

  if number ~= nil and number ~= "" then
    table.insert(out, '  number: "' .. typst_escape_string(number) .. '",')
  end

  if file_name ~= nil and file_name ~= "" then
    table.insert(out, '  file_name: "' .. typst_escape_string(file_name) .. '",')
  end

  if caption ~= "" then
    table.insert(out, "  caption: [" .. caption .. "],")
  end

  if lang ~= "" then
    table.insert(out, '  lang: "' .. typst_escape_string(lang) .. '",')
  end

  table.insert(out, '  code: "' .. typst_escape_string(code.text) .. '",')
  table.insert(out, ")")
  return pandoc.RawBlock("typst", table.concat(out, "\n"))
end
