# Summary: Why Khmer Isn't Combining Properly

## The Problem

Vowels and diacritics aren't combining with consonants because **Swash (the shaping library Neovide uses) has limited complex script support**.

## What's Happening

1. ✅ Script detection works - Neovide detects Khmer
2. ✅ OpenType features are requested - The code asks for `pref`, `blwf`, `abvs`, etc.
3. ❌ **Swash can't apply them** - Swash v0.2.x doesn't fully implement Khmer shaping
4. ✅ Positioning is better - My fix prevents overlapping
5. ❌ **Shaping is still broken** - Characters don't combine visually

## Why Swash Can't Do It

Swash is a lightweight font library focused on:
- Basic Latin text rendering
- Simple ligatures
- Fast performance

It does NOT implement:
- Full OpenType GSUB/GPOS engines
- Complex script reordering (Khmer, Arabic, Devanagari, etc.)
- Script-specific shaping rules

## The Solution: Add HarfBuzz

**HarfBuzz** is the industry-standard text shaping engine used by:
- Chrome/Chromium
- Firefox
- Android
- LibreOffice
- WezTerm (which is why we looked at it)

To fix Khmer in Neovide properly, we need to:

1. **Add rustybuzz dependency** (Rust port of HarfBuzz) ✅ Done
2. **Create hybrid shaper:**
   - Use rustybuzz for complex scripts (Khmer, Arabic, etc.)
   - Keep Swash for simple Latin text (faster)
3. **Integrate into Neovide's font pipeline**
   - Detect complex scripts
   - Route to appropriate shaper
   - Convert results to Skia TextBlobs

## Complexity

This is a **significant architectural change** because:
- Rustybuzz and Swash have different APIs
- Need to load font data differently for rustybuzz
- Need to map between their glyph/cluster representations
- Need to ensure performance doesn't regress

**Estimate:** 2-3 days of work for someone familiar with both libraries

## Alternative: Wait for Neovide

The Neovide maintainers might already be working on this. Let me check their issues/PRs for HarfBuzz integration.

## Your Options

###  1. Submit Issue to Neovide
Report that Swash doesn't support Khmer and request HarfBuzz integration.

### 2. I Can Implement HarfBuzz Integration
But it will take significant time and testing. Should I continue?

### 3. Use a Different Editor
For now, use VS Code, Sublime, or another editor with proper Khmer support while editing Khmer text.

### 4. Use Terminal Neovim
Interestingly, if your terminal (like WezTerm with HarfBuzz) supports Khmer, running `nvim` in the terminal might actually work better than Neovide!

Which approach would you prefer?
