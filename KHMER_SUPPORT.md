# Khmer Script Support for Neovide

## Overview

This patch adds proper Khmer script support to Neovide, along with improved support for other complex scripts including Arabic, Thai, Devanagari, and more.

## Changes Made

### 1. Script Detection (`detect_script` function)

Added automatic script detection based on Unicode ranges:
- **Khmer** (U+1780 - U+17FF)
- **Arabic** (U+0600 - U+06FF)
- **Thai** (U+0E00 - U+0E7F)
- **Devanagari** (U+0900 - U+097F)
- **Bengali, Tamil, Telugu, Myanmar, Lao, Tibetan, Sinhala** and more

The function examines the first non-ASCII character in the text to determine the appropriate script for shaping.

### 2. OpenType Feature Support (`get_font_features_for_script`)

Added script-specific OpenType features that enable proper complex script rendering:

#### Khmer Features:
- `pref` - Pre-base forms
- `blwf` - Below-base forms
- `abvs` - Above-base substitutions
- `psts` - Post-base substitutions
- `cfar` - Contextual alternate forms

#### Arabic Features:
- `init`, `medi`, `fina` - Positional forms
- `liga` - Ligatures
- `calt` - Contextual alternates

#### Indic Scripts (Devanagari, Bengali, Tamil, Telugu):
- `nukt`, `akhn`, `rphf`, `half`, `pstf`, etc.

### 3. Complex Script Handling

The shaper now:
1. Detects the script from the input text
2. Passes the correct script to Swash's Parser
3. Enables appropriate OpenType features for that script
4. Allows natural glyph positioning for complex scripts

## Testing Khmer Support

### Test Text Examples

Create a test file with Khmer text:

```khmer
ក ខ គ ឃ ង ច ឆ ជ ឈ ញ ដ ឋ ឌ ឍ ណ
ត ថ ទ ធ ន ប ផ ព ភ ម យ រ ល វ
ស ហ ឡ អ

Vowels:
កា កិ កី កឹ កឺ កុ កូ កួ កើ កឿ កៀ កេ កែ កៃ កោ កៅ

Complex clusters:
ក្ខ ក្គ ក្ឃ ក្ង ក្ច ក្ឆ ក្ជ ក្ញ ក្ដ
ក្ឋ ក្ឌ ក្ឍ ក្ណ ក្ត ក្ថ ក្ទ ក្ធ ក្ន
ក្ប ក្ផ ក្ព ក្ភ ក្ម ក្យ ក្រ ក្ល ក្វ
ក្ស ក្ហ អ្អ

Words:
កម្ពុជា ភ្នំពេញ សៀមរាប កំពង់ចាម
ខ្មែរ ភាសា សៀវភៅ កុំព្យូទ័រ អ្នក
```

### Running Tests

1. Build Neovide with the changes:
   ```bash
   cd neovide
   cargo build --release
   ```

2. Run Neovide:
   ```bash
   ./target/release/neovide
   ```

3. In Neovim, open a test file:
   ```
   :e test_khmer.txt
   ```

4. Type or paste Khmer text to verify:
   - ✓ Vowels and diacritics position correctly above/below consonants
   - ✓ Complex consonant clusters render properly
   - ✓ Cursor moves logically through the text
   - ✓ No rendering artifacts or crashes

### Expected Results

**Before this patch:**
- Khmer characters render as separate glyphs
- Vowels and diacritics don't position correctly
- Complex clusters appear broken
- Text is difficult to read

**After this patch:**
- Proper shaping with correct vowel positioning
- Complex consonant clusters render as unified glyphs
- Natural appearance matching native Khmer text rendering
- Improved (though still grid-constrained) layout

## Technical Details

### How It Works

1. **Script Detection**: When `build_clusters` is called, the text is scanned to detect its script
2. **Parser Initialization**: The Swash `Parser` is initialized with the detected script instead of always using `Script::Latin`
3. **Feature Selection**: `get_font_features_for_script` adds the appropriate OpenType features for the detected script
4. **Shaping**: Swash's shaper applies the features and produces properly shaped glyphs
5. **Rendering**: Glyphs are converted to Skia TextBlobs and cached for efficient rendering

### Limitations

1. **Grid Constraint**: Neovide still uses a fixed-width grid inherited from Neovim's terminal model. This means:
   - Very wide Khmer clusters may appear compressed
   - Some glyphs may overlap slightly with adjacent cells
   - Perfect proportional layout is not possible

2. **Font Dependency**: Proper rendering requires a font with:
   - Complete Khmer Unicode coverage (U+1780 - U+17FF)
   - Proper OpenType GSUB/GPOS tables
   - Recommended fonts: Noto Sans Khmer, Khmer OS, Khmer Mondulkiri

3. **Mixed Scripts**: Text with multiple scripts may have suboptimal rendering at script boundaries

### Future Improvements

Potential enhancements:
1. Add configuration option for per-script cell width multipliers
2. Implement dynamic cell width allocation based on actual glyph metrics
3. Add font family suggestions for various scripts
4. Improve script detection for mixed-script text
5. Add support for right-to-left (RTL) scripts with bidirectional text algorithm

## Configuration

### Recommended Font Settings

Add to your Neovide config or init.vim:

```lua
-- For Neovim with Neovide
vim.opt.guifont = "Noto Sans Khmer:h12"
-- Or mixed with Latin:
vim.opt.guifont = "FiraCode NF:h12,Noto Sans Khmer:h12"
```

### Installing Khmer Fonts

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install fonts-noto-cjk fonts-khmeros

# Arch Linux
sudo pacman -S noto-fonts noto-fonts-extra
```

**macOS:**
```bash
brew install font-noto-sans-khmer
```

**Windows:**
Download from [Google Fonts](https://fonts.google.com/noto/specimen/Noto+Sans+Khmer)

## Contributing

To test or improve this implementation:

1. Test with various Khmer fonts and provide feedback
2. Report rendering issues with specific character combinations
3. Suggest additional OpenType features that improve rendering
4. Test with other complex scripts and report results

## References

- [Unicode Khmer Block](https://unicode.org/charts/PDF/U1780.pdf) (U+1780 - U+17FF)
- [OpenType Feature Tags](https://docs.microsoft.com/en-us/typography/opentype/spec/featuretags)
- [Swash Documentation](https://docs.rs/swash/)
- [Khmer Script on Wikipedia](https://en.wikipedia.org/wiki/Khmer_script)

## Credits

Implementation based on understanding of:
- WezTerm's complex script handling
- Swash shaper capabilities
- OpenType specification for Indic scripts
- Neovide's existing font rendering pipeline
