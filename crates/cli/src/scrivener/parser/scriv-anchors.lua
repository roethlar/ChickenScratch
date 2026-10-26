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

function Link(el)
  local scheme, id = el.target:match("^(%a+)://(.+)$")
  if scheme == nil or not SCHEMES[scheme] then
    return nil
  end
  local out = pandoc.List()
  out:insert(pandoc.Str("\u{E000}" .. scheme .. ":" .. id .. "\u{E001}"))
  out:extend(el.content)
  out:insert(pandoc.Str("\u{E002}"))
  return out
end
