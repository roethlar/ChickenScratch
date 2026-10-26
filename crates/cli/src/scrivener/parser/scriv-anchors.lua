-- ChickenScratch Scrivener import: Scrivener's comment, footnote and
-- annotation anchors are marked inside Pandoc's own parse.
--
-- Scrivener links a comment or footnote to its text with a hyperlink field
-- whose target is `scrivcmt://<id>`; the importer's own lifted inline notes
-- use `scrivfn://` and `scrivannot://`. Written out as Markdown links they
-- had to be found again by scanning the Markdown, in competition with
-- ordinary links, code spans and escapes around them (review finding s4-5:
-- six rounds each left a case behind). Here, while the document is still
-- Pandoc's structure, each such Link becomes its content wrapped in
-- private-use characters that never occur in prose:
--
--   U+E000 <scheme> ":" <id> U+E001 <content…> U+E002
--
-- The importer resolves exactly these markers, whatever Markdown surrounds
-- them; an ordinary link is left as a link.

local SCHEMES = { scrivcmt = true, scrivfn = true, scrivannot = true }

-- An ordinary link's destination is written so that it reads back as the
-- same link: Pandoc's Markdown writer leaves parentheses, spaces, angle
-- brackets and backticks raw, and a destination such as
-- `https://e.com/search?q=((x` then swallows the prose after it (a footnote
-- included) when the Markdown is read again at compile time (review finding
-- s4-5, eighth repair). Percent-encoding keeps the URL the same for a
-- browser and unambiguous for every Markdown reader; what is already
-- percent-encoded is left alone.
local function safe_destination(url)
  return (url:gsub("[%(%)%s<>`\\]", function(c)
    return string.format("%%%02X", c:byte())
  end))
end

function Link(el)
  local scheme, id = el.target:match("^(%a+)://(.+)$")
  if scheme == nil or not SCHEMES[scheme] then
    el.target = safe_destination(el.target)
    return el
  end
  local out = pandoc.List()
  out:insert(pandoc.Str("\u{E000}" .. scheme .. ":" .. id .. "\u{E001}"))
  out:extend(el.content)
  out:insert(pandoc.Str("\u{E002}"))
  return out
end
