-- ChickenScratch compile: the editor's footnote element becomes a Pandoc
-- footnote, inside Pandoc's own parse.
--
-- The editor stores a footnote as
--   <sup class="footnote" data-body="…">●</sup>
-- (ui/src/components/editor/FootnoteNode.ts). Pandoc reads the opening tag
-- as a RawInline, the marker as text and the closing tag as another
-- RawInline; this filter replaces that run with a Note whose body is the
-- attribute's text, written as plain text (no Markdown inside it).
--
-- Working on Pandoc's parsed document means anything Pandoc considers code
-- (a code span, a fenced or indented block, a verbatim <pre>) is never
-- touched, whatever it contains, and anything Pandoc considers prose (inside
-- a <div>, a blockquote, a list, …) is converted (review finding s4-8:
-- five rounds of re-implementing Markdown's code rules by hand each left a
-- case behind).

local function unescape(text)
  text = text:gsub("&quot;", '"')
  text = text:gsub("&#39;", "'")
  text = text:gsub("&#10;", "\n")
  text = text:gsub("&lt;", "<")
  text = text:gsub("&gt;", ">")
  text = text:gsub("&amp;", "&")
  return text
end

-- The body of a footnote opening tag, or nil when the tag is not one.
local function footnote_body(el)
  if el.t ~= "RawInline" or not el.format:match("html") then
    return nil
  end
  local tag = el.text
  if not tag:match("^<[sS][uU][pP][%s>]") then
    return nil
  end
  if not (tag:match('class="footnote"') or tag:match("class='footnote'")) then
    return nil
  end
  local body = tag:match('data%-body="([^"]*)"') or tag:match("data%-body='([^']*)'")
  if body == nil then
    return nil
  end
  return unescape(body)
end

local function is_closing_sup(el)
  return el.t == "RawInline" and el.format:match("html") and el.text:match("^</[sS][uU][pP]%s*>$") ~= nil
end

-- Plain text to blocks: blank lines separate paragraphs, line ends inside
-- a paragraph are soft breaks, words are separated by single spaces.
local function body_blocks(body)
  local blocks = pandoc.List()
  body = body:gsub("\r\n", "\n"):gsub("\r", "\n")
  for paragraph in (body .. "\n\n"):gmatch("(.-)\n%s*\n") do
    if paragraph:match("%S") then
      local inlines = pandoc.List()
      local first_line = true
      for line in (paragraph .. "\n"):gmatch("(.-)\n") do
        if not first_line then
          inlines:insert(pandoc.SoftBreak())
        end
        first_line = false
        local first_word = true
        for word in line:gmatch("%S+") do
          if not first_word then
            inlines:insert(pandoc.Space())
          end
          first_word = false
          inlines:insert(pandoc.Str(word))
        end
      end
      blocks:insert(pandoc.Para(inlines))
    end
  end
  if #blocks == 0 then
    blocks:insert(pandoc.Para(pandoc.List()))
  end
  return blocks
end

function Inlines(inlines)
  local out = pandoc.List()
  local i = 1
  while i <= #inlines do
    local el = inlines[i]
    local body = footnote_body(el)
    if body ~= nil then
      local j = i + 1
      while j <= #inlines and not is_closing_sup(inlines[j]) do
        j = j + 1
      end
      if j <= #inlines then
        out:insert(pandoc.Note(body_blocks(body)))
        i = j + 1
      else
        -- No closing tag anywhere after this opener: nothing further in
        -- this run can be a footnote either, so the rest is copied as it is
        -- (review finding s4-8, fifth re-review: rescanning after every
        -- unclosed opener was quadratic and timed out a 10,000-opener
        -- paragraph).
        for k = i, #inlines do
          out:insert(inlines[k])
        end
        break
      end
    else
      out:insert(el)
      i = i + 1
    end
  end
  return out
end
