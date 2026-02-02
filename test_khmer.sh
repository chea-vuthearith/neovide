#!/bin/bash

# Test Khmer rendering in Neovide with the fix

echo "Testing Khmer support in Neovide..."
echo ""
echo "Creating test file..."

cat >/tmp/khmer_test.txt <<'EOF'
Khmer Test:
ក ខ គ
កា កិ កី
ក្ខ ក្រ ក្ម
កុំ អ្នក ភាសា
EOF

echo "Test file created at /tmp/khmer_test.txt"
echo ""
echo "Starting Neovide with Khmer font..."
echo "Run this command:"
echo ""
echo "  cd neovide && nix-shell --run './target/release/neovide --guifont=\"Noto Sans Khmer:h14\" /tmp/khmer_test.txt'"
echo ""
echo "What to check:"
echo "1. Do vowels (ា ិ ី) appear above/below consonants correctly?"
echo "2. Do subscript clusters (ក្ខ) render without overlapping?"
echo "3. Does the text look readable?"
echo ""
echo "Known issue with this fix:"
echo "- Cursor position may not match visual position perfectly"
echo "- This is because Neovim doesn't know about the actual glyph widths"
echo ""
