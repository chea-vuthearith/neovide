# Why Swash Doesn't Work for Khmer - Technical Deep Dive

## Summary

**Swash v0.2.x has Khmer Unicode awareness but LACKS the OpenType shaping engine needed for proper Khmer text rendering.**

## What I Found

### 1. Swash HAS Khmer Support in Unicode Layer ✅

From `unicode_data.rs`:
- Khmer character properties are defined
- Unicode block U+1780-U+17FF is recognized
- Character classes are correct (VAbv, VBlw, VPre, etc.)

### 2. Swash HAS Basic Khmer Clustering ✅

From `complex.rs`:
- Special handling for Khmer coeng (U+17D2)
- Decomposition logic for some Khmer vowels (U+17BE-U+17BF, U+17C0, U+17C4-U+17C5)
- This groups characters into clusters

### 3. Swash LACKS Khmer OpenType Shaping ❌

From `at.rs` (Advanced Typography / OpenType):
- Only explicit script handling: `Script::Myanmar` and `Script::Hangul`
- **NO Khmer-specific shaping**
- **NO Universal Shaping Engine (USE) implementation**

## What This Means

### What Works:
1. **Character recognition** - Swash knows what Khmer characters are
2. **Basic clustering** - Characters are grouped (e.g., "ក" + "ុ" + "ំ" form one cluster)
3. **Feature requests** - When we ask for `pref`, `blwf`, `abvs`, etc., Swash doesn't error

### What Doesn't Work:
1. **OpenType GSUB** - No glyph substitution (subscript forms, ligatures)
2. **OpenType GPOS** - No glyph positioning (vowel/mark placement)
3. **Proper shaping** - Vowels stay as separate glyphs, don't combine visually

## Why Neovide Shows "കុ" Instead of "കു"

Let's trace what happens when you type "ក" (base) + "ុ" (vowel):

### Step 1: Parser (text → clusters)
```
Input: "កុ" (2 Unicode codepoints)
Swash Parser with Script::Khmer: ✅ Groups them into 1 cluster
Output: Cluster["កុ"]
```

### Step 2: Font Lookup (clusters → glyph IDs)
```
Input: Cluster["កុ"]
Swash charmap.map(): ✅ Finds glyphs
Output: [glyph_id_1780, glyph_id_17BB]  (two separate glyphs)
```

### Step 3: Feature Application (glyph IDs → shaped glyphs)
```
Input: [glyph_id_1780, glyph_id_17BB]
Input Features: ["pref", "blwf", "abvs", ...]

EXPECTED (with HarfBuzz):
- GSUB: Replace glyph combo with special forms
- GPOS: Position vowel above base consonant
Output: Combined glyph at position (x, y_base-10)

ACTUAL (with Swash):
- GSUB: ❌ NOT IMPLEMENTED for Khmer
- GPOS: ❌ NOT IMPLEMENTED for Khmer
Output: Two separate glyphs at (x, y) and (x+advance, y)
```

### Step 4: Rendering
```
Swash output: Two glyphs side-by-side
Result: "ក ុ" (separated, overlapping)
```

## What HarfBuzz Does Differently

HarfBuzz has a **full OpenType shaping engine**:

```rust
// HarfBuzz (rustybuzz) pipeline:
1. Parse text → clusters (similar to Swash)
2. **Run GSUB (Glyph Substitution)**
   - Apply pref: Prebase substitutions
   - Apply blwf: Replace with subscript forms
   - Apply abvs: Replace with combined forms
3. **Run GPOS (Glyph Positioning)**
   - Calculate vowel offset relative to base
   - Apply mark-to-base positioning
   - Stack multiple marks correctly
4. Output: Properly positioned combined glyphs
```

## Swash's Design Limitations

From Swash's README:
> **Implementation of the Universal Shaping Engine for complex scripts such as Devanagari, Malayalam, etc.**

Notice: It says "such as" but doesn't list Khmer!

Looking at the code:
- **Myanmar**: Explicitly handled (line in at.rs)
- **Hangul**: Explicitly handled (line in at.rs)
- **Khmer**: ❌ NOT explicitly handled

Swash implements OpenType features **generically** (ligatures, kerning, etc.) but complex scripts need **script-specific shaping logic** which Swash only provides for a few scripts.

## Why This Matters

### The OpenType Khmer Shaping Spec Requires:

1. **Reordering**: Move pre-base vowels before base
2. **Coeng handling**: Transform subscript consonants
3. **Mark positioning**: Stack multiple combining marks
4. **Feature sequencing**: Apply features in specific order

**Swash doesn't implement these rules for Khmer.**

## The Evidence

From my testing:
- ✅ Script detection works (Swash knows it's Khmer)
- ✅ Features are requested (code asks for pref, blwf, etc.)
- ✅ Clustering works (characters are grouped)
- ❌ **Shaping doesn't work** (glyphs don't combine)

This proves Swash receives the right inputs but can't process them.

## Comparison

| Feature | Swash | HarfBuzz |
|---------|-------|----------|
| Khmer Unicode data | ✅ Complete | ✅ Complete |
| Khmer clustering | ✅ Basic | ✅ Full |
| OpenType GSUB | ❌ Generic only | ✅ Full |
| OpenType GPOS | ❌ Generic only | ✅ Full |
| Script-specific rules | ❌ Myanmar & Hangul only | ✅ All scripts |
| Khmer shaping | ❌ **Broken** | ✅ **Works** |

## Conclusion

**Swash can't do Khmer shaping** because it doesn't implement the OpenType shaping engine for Khmer script. It has the pieces (Unicode data, clustering) but lacks the critical shaping logic (GSUB/GPOS rules).

**To fix Khmer in Neovide, we must:**
1. Add rustybuzz (HarfBuzz) as a dependency ✅ Done
2. Route Khmer text to rustybuzz instead of Swash
3. Convert rustybuzz output to Neovide's glyph format
4. Integrate into the rendering pipeline

This is a **significant architectural change** but it's the **only way** to get proper Khmer rendering in Neovide.

## References

- Swash source: https://github.com/dfrg/swash
- HarfBuzz OpenType spec: https://harfbuzz.github.io/
- Khmer OpenType spec: https://learn.microsoft.com/en-us/typography/script-development/khmer
- USE (Universal Shaping Engine): https://github.com/harfbuzz/harfbuzz/blob/main/docs/usermanual-clusters.md

---

**Bottom line:** Swash tries to be a lightweight alternative to HarfBuzz, but this means it doesn't support all scripts. Khmer is one of them.
