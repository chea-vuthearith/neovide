use std::{num::NonZeroUsize, rc::Rc};

use itertools::Itertools;
use log::{debug, error, info, trace};
use lru::LruCache;
use skia_safe::{
    graphics::{font_cache_limit, font_cache_used, set_font_cache_limit},
    TextBlob, TextBlobBuilder,
};
use swash::{
    shape::ShapeContext,
    text::{
        cluster::{CharCluster, Parser, Status, Token},
        Script,
    },
    Metrics,
};

use crate::{
    editor::Word,
    error_msg,
    profiling::tracy_zone,
    renderer::fonts::{font_loader::*, font_options::*},
    units::PixelSize,
};

#[derive(new, Clone, Hash, PartialEq, Eq, Debug)]
struct ShapeKey {
    pub text: String,
    pub style: CoarseStyle,
}

const FONT_CACHE_SIZE: usize = 8 * 1024 * 1024;

pub struct CachingShaper {
    options: FontOptions,
    font_loader: FontLoader,
    blob_cache: LruCache<ShapeKey, Vec<TextBlob>>,
    shape_context: ShapeContext,
    scale_factor: f32,
    linespace: f32,
    font_info: Option<(Metrics, f32)>,
}

impl CachingShaper {
    pub fn new(scale_factor: f32) -> CachingShaper {
        let options = FontOptions::default();
        let font_size = options.size * scale_factor;
        let mut shaper = CachingShaper {
            options,
            font_loader: FontLoader::new(font_size),
            blob_cache: LruCache::new(NonZeroUsize::new(10000).unwrap()),
            shape_context: ShapeContext::new(),
            scale_factor,
            linespace: 0.0,
            font_info: None,
        };
        shaper.reset_font_loader();
        shaper
    }

    fn current_font_pair(&mut self) -> Rc<FontPair> {
        self.font_loader
            .get_or_load(&FontKey {
                font_desc: self.options.primary_font(),
                hinting: self.options.hinting.clone(),
                edging: self.options.edging.clone(),
            })
            .unwrap_or_else(|| {
                self.font_loader
                    .get_or_load(&FontKey::default())
                    .expect("Could not load default font")
            })
    }

    pub fn current_size(&self) -> f32 {
        let min_font_size = 1.0;
        (self.options.size * self.scale_factor).max(min_font_size)
    }

    pub fn update_scale_factor(&mut self, scale_factor: f32) {
        debug!("scale_factor changed: {scale_factor:.2}");
        self.scale_factor = scale_factor;
        self.reset_font_loader();
    }

    pub fn update_font(&mut self, guifont_setting: &str) {
        debug!("Updating font: {guifont_setting}");

        let options = match FontOptions::parse(guifont_setting) {
            Ok(opt) => opt,
            Err(msg) => {
                error_msg!("Failed to parse guifont: {}", msg);
                return;
            }
        };

        self.update_font_options(options);
    }

    pub fn update_font_options(&mut self, options: FontOptions) {
        debug!("Updating font options: {options:?}");

        let keys = options
            .possible_fonts()
            .iter()
            .map(|desc| FontKey {
                font_desc: Some(desc.clone()),
                hinting: options.hinting.clone(),
                edging: options.edging.clone(),
            })
            .unique()
            .collect::<Vec<_>>();

        let failed_fonts = keys
            .iter()
            .filter(|key| self.font_loader.get_or_load(key).is_none())
            .collect_vec();

        if !failed_fonts.is_empty() {
            error_msg!(
                "Font can't be updated to: {:#?}\n\
                Following fonts couldn't be loaded: {}",
                options,
                failed_fonts.iter().join(",\n"),
            );
        }

        if failed_fonts.len() != keys.len() {
            debug!("Font updated to: {options:?}");
            self.options = options;
            self.reset_font_loader();
        }
    }

    pub fn update_linespace(&mut self, linespace: f32) {
        debug!("Updating linespace: {linespace}");

        let font_height = self.font_base_dimensions().height;
        let impossible_linespace = font_height + linespace <= 0.0;

        if !impossible_linespace {
            debug!("Linespace updated to: {linespace}");
            self.linespace = linespace;
            self.reset_font_loader();
        } else {
            let reason = if impossible_linespace {
                "Linespace too negative, would make font invisible"
            } else {
                "Font not found"
            };
            error!("Linespace can't be updated to {linespace} due to: {reason}");
        }
    }

    fn reset_font_loader(&mut self) {
        tracy_zone!("reset_font_loader");
        self.font_info = None;
        let font_size = self.current_size();

        self.font_loader = FontLoader::new(font_size);
        let (_, font_width) = self.info();
        info!("Reset Font Loader: font_size: {font_size:.2}px, font_width: {font_width:.2}px");

        self.blob_cache.clear();
    }

    pub fn font_names(&self) -> Vec<String> {
        self.font_loader.font_names()
    }

    fn info(&mut self) -> (Metrics, f32) {
        if let Some(info) = self.font_info {
            return info;
        }

        let font_pair = self.current_font_pair();
        let size = self.current_size();
        let mut shaper = self
            .shape_context
            .builder(font_pair.swash_font.as_ref())
            .size(size)
            .build();
        shaper.add_str("M");
        let metrics = shaper.metrics();
        let mut advance = metrics.average_width;
        shaper.shape_with(|cluster| {
            advance = cluster
                .glyphs
                .first()
                .map_or(metrics.average_width, |g| g.advance);
        });
        self.font_info = Some((metrics, advance));
        (metrics, advance)
    }

    fn metrics(&mut self) -> Metrics {
        tracy_zone!("font_metrics");
        self.info().0
    }

    pub fn font_base_dimensions(&mut self) -> PixelSize<f32> {
        let (metrics, glyph_advance) = self.info();

        let bare_font_height = metrics.ascent + metrics.descent + metrics.leading;
        // assuming that linespace is checked on receive for validity
        let font_height = (bare_font_height + self.linespace).ceil();
        let font_width = glyph_advance + self.options.width;

        (font_width, font_height).into()
    }

    pub fn underline_offset(&mut self) -> f32 {
        let metrics = self.metrics();
        if let Some(underline_offset) = self.options.underline_offset {
            -underline_offset
        } else if metrics.underline_offset != 0. {
            metrics.underline_offset
        } else {
            // If a font does not have an underline_offset, use the stroke_size as offset
            // A negative offset places the underline below the baseline
            -metrics.stroke_size
        }
    }

    pub fn stroke_size(&mut self) -> f32 {
        self.metrics().stroke_size
    }

    pub fn baseline_offset(&mut self) -> f32 {
        let metrics = self.metrics();
        // NOTE: leading is also called linegap and should be equally distributed on the top and
        // bottom, so it's centered like our linespace settings. That's how it works on the web,
        // but some desktop applications only use the top according to:
        // https://googlefonts.github.io/gf-guide/metrics.html#8-linegap-values-must-be-0
        metrics.ascent + (metrics.leading + self.linespace) / 2.0
    }

    /// Detect the script for a given text string by examining the first non-ASCII character
    fn detect_script(text: &str) -> Script {
        for ch in text.chars() {
            // Skip whitespace and common ASCII punctuation
            if ch.is_ascii() {
                continue;
            }
            
            // Check Unicode ranges for various scripts
            match ch {
                // Khmer script (U+1780 - U+17FF)
                '\u{1780}'..='\u{17FF}' => return Script::Khmer,
                
                // Arabic script (U+0600 - U+06FF)
                '\u{0600}'..='\u{06FF}' => return Script::Arabic,
                
                // Hebrew script (U+0590 - U+05FF)
                '\u{0590}'..='\u{05FF}' => return Script::Hebrew,
                
                // Thai script (U+0E00 - U+0E7F)
                '\u{0E00}'..='\u{0E7F}' => return Script::Thai,
                
                // Devanagari script (U+0900 - U+097F)
                '\u{0900}'..='\u{097F}' => return Script::Devanagari,
                
                // Bengali script (U+0980 - U+09FF)
                '\u{0980}'..='\u{09FF}' => return Script::Bengali,
                
                // Tamil script (U+0B80 - U+0BFF)
                '\u{0B80}'..='\u{0BFF}' => return Script::Tamil,
                
                // Telugu script (U+0C00 - U+0C7F)
                '\u{0C00}'..='\u{0C7F}' => return Script::Telugu,
                
                // Myanmar script (U+1000 - U+109F)
                '\u{1000}'..='\u{109F}' => return Script::Myanmar,
                
                // Lao script (U+0E80 - U+0EFF)
                '\u{0E80}'..='\u{0EFF}' => return Script::Lao,
                
                // Tibetan script (U+0F00 - U+0FFF)
                '\u{0F00}'..='\u{0FFF}' => return Script::Tibetan,
                
                // Sinhala script (U+0D80 - U+0DFF)
                '\u{0D80}'..='\u{0DFF}' => return Script::Sinhala,
                
                _ => continue,
            }
        }
        
        // Default to Latin if no specific script detected
        Script::Latin
    }

    fn build_clusters(
        &mut self,
        word: Word<'_>,
        style: CoarseStyle,
    ) -> Vec<(Vec<CharCluster>, Rc<FontPair>)> {
        let mut cluster = CharCluster::new();

        // Detect the script from the word text
        let script = Self::detect_script(word.text);
        
        // Debug: Log what script was detected
        if !matches!(script, Script::Latin) {
            log::debug!("Detected script: {:?} for text: {:?}", script, word.text);
        }

        // Enumerate the characters storing the glyph index in the user data so that we can position
        // glyphs according to Neovim's grid rules
        let mut parser = Parser::new(
            script,
            word.grapheme_clusters().flat_map(|(cell_index, cluster)| {
                // Debug: Log clusters from Neovim
                if !matches!(script, Script::Latin) {
                    log::debug!("  Cell {}: {:?}", cell_index, cluster);
                }
                cluster
                    .char_indices()
                    .map(move |(offset, character)| Token {
                        ch: character,
                        offset: offset as u32,
                        len: character.len_utf8() as u8,
                        info: character.into(),
                        data: cell_index as u32,
                    })
            }),
        );

        let mut results = Vec::new();
        'cluster: while parser.next(&mut cluster) {
            // TODO: Don't redo this work for every cluster. Save it some how
            // Create font fallback list
            let mut font_fallback_keys = Vec::new();

            // Add parsed fonts from guifont or config file
            font_fallback_keys.extend(
                self.options
                    .font_list(style)
                    .iter()
                    .map(|font_desc| FontKey {
                        font_desc: Some(font_desc.clone()),
                        hinting: self.options.hinting.clone(),
                        edging: self.options.edging.clone(),
                    })
                    .unique(),
            );

            // Add default font
            font_fallback_keys.push(FontKey {
                font_desc: None,
                hinting: self.options.hinting.clone(),
                edging: self.options.edging.clone(),
            });

            // Use the cluster.map function to select a viable font from the fallback list and loaded fonts

            let mut best = None;
            // Search through the configured and default fonts for a match
            for fallback_key in font_fallback_keys.iter() {
                if let Some(font_pair) = self.font_loader.get_or_load(fallback_key) {
                    let charmap = font_pair.swash_font.as_ref().charmap();
                    match cluster.map(|ch| charmap.map(ch)) {
                        Status::Complete => {
                            results.push((cluster.to_owned(), font_pair.clone()));
                            continue 'cluster;
                        }
                        Status::Keep => best = Some(font_pair),
                        Status::Discard => {}
                    }
                }
            }

            // Configured font/default didn't work. Search through currently loaded ones
            for loaded_font in self.font_loader.loaded_fonts() {
                let charmap = loaded_font.swash_font.as_ref().charmap();
                match cluster.map(|ch| charmap.map(ch)) {
                    Status::Complete => {
                        results.push((cluster.to_owned(), loaded_font.clone()));
                        self.font_loader.refresh(loaded_font.as_ref());
                        continue 'cluster;
                    }
                    Status::Keep => best = Some(loaded_font),
                    Status::Discard => {}
                }
            }

            if let Some(best) = best {
                results.push((cluster.to_owned(), best.clone()));
            } else {
                let fallback_character = cluster.chars()[0].ch;
                if let Some(fallback_font) = self
                    .font_loader
                    .load_font_for_character(style, fallback_character)
                {
                    results.push((cluster.to_owned(), fallback_font));
                } else {
                    // Last Resort covers all of the unicode space so we will always have a fallback
                    results.push((
                        cluster.to_owned(),
                        self.font_loader.get_or_load_last_resort().unwrap(),
                    ));
                }
            }
        }

        // Now we have to group clusters by the font used so that the shaper can actually form
        // ligatures across clusters
        let mut grouped_results = Vec::new();
        let mut current_group = Vec::new();
        let mut current_font_option = None;
        for (cluster, font) in results {
            if let Some(current_font) = current_font_option.clone() {
                if current_font == font {
                    current_group.push(cluster);
                } else {
                    grouped_results.push((current_group, current_font));
                    current_group = vec![cluster];
                    current_font_option = Some(font);
                }
            } else {
                current_group = vec![cluster];
                current_font_option = Some(font);
            }
        }

        if !current_group.is_empty() {
            grouped_results.push((current_group, current_font_option.unwrap()));
        }

        grouped_results
    }

    pub fn cleanup_font_cache(&self) {
        // Only purge if we are truly about to exhaust the cache.
        // See: https://github.com/neovide/neovide/issues/3299
        // On high-DPI displays the old unconditional purge invalidated
        // glyphs every frame, which forced the GPU to keep
        // re-uploading textures and tanked performance.
        let limit = font_cache_limit();
        let used = font_cache_used();

        if used > limit * 9 / 10 {
            tracy_zone!("purge_font_cache");
            set_font_cache_limit(FONT_CACHE_SIZE / 2);
            set_font_cache_limit(FONT_CACHE_SIZE);
        }
    }

    pub fn shape(&mut self, word: Word<'_>, style: CoarseStyle) -> Vec<TextBlob> {
        let current_size = self.current_size();
        let glyph_width = self.font_base_dimensions().width;
        
        // Detect script for the word
        let script = Self::detect_script(word.text);

        let mut resulting_blobs = Vec::new();

        for (cluster_group, font_pair) in self.build_clusters(word, style) {
            let features = self.get_font_features_for_script(
                font_pair
                    .as_ref()
                    .key
                    .font_desc
                    .as_ref()
                    .map(|desc| desc.family.as_str()),
                script,
            );

            let mut shaper = self
                .shape_context
                .builder(font_pair.swash_font.as_ref())
                .features(features.iter().map(|(name, value)| (name.as_ref(), *value)))
                .size(current_size)
                .build();

            let charmap = font_pair.swash_font.as_ref().charmap();
            for mut cluster in cluster_group {
                cluster.map(|ch| charmap.map(ch));
                shaper.add_cluster(&cluster);
            }

            let mut glyph_data = Vec::new();
            
            // Check if this is a complex script that benefits from slight overflow
            let is_complex_script = matches!(
                script,
                Script::Khmer
                    | Script::Arabic
                    | Script::Thai
                    | Script::Lao
                    | Script::Devanagari
                    | Script::Bengali
                    | Script::Tamil
                    | Script::Telugu
                    | Script::Myanmar
                    | Script::Tibetan
                    | Script::Sinhala
            );

            // Track the actual x position as we render glyphs within this cluster group
            // For complex scripts, we'll use continuous positioning
            let mut current_x_pos = None;

            shaper.shape_with(|glyph_cluster| {
                // Start position from cell index
                let cell_index = glyph_cluster.data as f32;
                let grid_x_offset = glyph_width * cell_index;
                
                // For complex scripts, use continuous positioning after the first cluster in this group
                let base_x_offset = if is_complex_script {
                    if let Some(pos) = current_x_pos {
                        // Continue from where the last glyph ended
                        pos
                    } else {
                        // First cluster in this group: use grid position
                        grid_x_offset
                    }
                } else {
                    // Non-complex script: always use grid position
                    grid_x_offset
                };
                
                let mut x_offset = base_x_offset;
                for glyph in glyph_cluster.glyphs {
                    let position = (x_offset + glyph.x, -glyph.y);
                    glyph_data.push((glyph.id, position));
                    x_offset += glyph.advance;
                }
                
                // Update continuous position for next cluster in this group
                current_x_pos = Some(x_offset);
            });

            if glyph_data.is_empty() {
                continue;
            }

            let mut blob_builder = TextBlobBuilder::new();
            let (glyphs, positions) =
                blob_builder.alloc_run_pos(&font_pair.skia_font, glyph_data.len(), None);
            for (i, (glyph_id, glyph_position)) in glyph_data.iter().enumerate() {
                glyphs[i] = *glyph_id;
                positions[i] = (*glyph_position).into();
            }

            let blob = blob_builder.make();
            resulting_blobs.push(blob.expect("Could not create textblob"));
        }

        resulting_blobs
    }

    pub fn shape_cached(&mut self, word: Word<'_>, style: CoarseStyle) -> &Vec<TextBlob> {
        tracy_zone!("shape_cached");
        let text = word.text;
        let key = ShapeKey::new(text.to_string(), style);

        if !self.blob_cache.contains(&key) {
            trace!("Shaping text: {text:?}");
            let blobs = self.shape(word, style);
            self.blob_cache.put(key.clone(), blobs);
        }

        self.blob_cache.get(&key).unwrap()
    }

    fn get_font_features(&self, name: Option<&str>) -> Vec<(String, u16)> {
        let features = if let Some(name) = name {
            self.options
                .features
                .get(name)
                .map(|features| {
                    features
                        .iter()
                        .map(|feature| (feature.0.clone(), feature.1))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        
        features
    }
    
    fn get_font_features_for_script(&self, name: Option<&str>, script: Script) -> Vec<(String, u16)> {
        let mut features = self.get_font_features(name);
        
        // Add script-specific OpenType features
        match script {
            Script::Khmer => {
                // Khmer-specific OpenType features for proper shaping
                // These features handle the complex positioning of vowels and diacritics
                features.extend(vec![
                    ("pref".to_string(), 1),  // Pre-base forms
                    ("blwf".to_string(), 1),  // Below-base forms
                    ("abvs".to_string(), 1),  // Above-base substitutions
                    ("psts".to_string(), 1),  // Post-base substitutions
                    ("cfar".to_string(), 1),  // Contextual alternate forms
                ]);
            }
            Script::Arabic => {
                // Arabic contextual forms
                features.extend(vec![
                    ("init".to_string(), 1),  // Initial forms
                    ("medi".to_string(), 1),  // Medial forms
                    ("fina".to_string(), 1),  // Final forms
                    ("liga".to_string(), 1),  // Ligatures
                    ("calt".to_string(), 1),  // Contextual alternates
                ]);
            }
            Script::Devanagari | Script::Bengali | Script::Tamil | Script::Telugu => {
                // Indic scripts
                features.extend(vec![
                    ("nukt".to_string(), 1),  // Nukta forms
                    ("akhn".to_string(), 1),  // Akhand forms
                    ("rphf".to_string(), 1),  // Reph forms
                    ("blwf".to_string(), 1),  // Below-base forms
                    ("half".to_string(), 1),  // Half forms
                    ("pstf".to_string(), 1),  // Post-base forms
                    ("pres".to_string(), 1),  // Pre-base substitutions
                    ("abvs".to_string(), 1),  // Above-base substitutions
                    ("blws".to_string(), 1),  // Below-base substitutions
                    ("psts".to_string(), 1),  // Post-base substitutions
                ]);
            }
            Script::Thai | Script::Lao => {
                // Thai and Lao tone marks and vowels
                features.extend(vec![
                    ("ccmp".to_string(), 1),  // Glyph composition/decomposition
                    ("liga".to_string(), 1),  // Ligatures
                ]);
            }
            Script::Myanmar => {
                // Myanmar (Burmese) script
                features.extend(vec![
                    ("rphf".to_string(), 1),  // Reph forms
                    ("pref".to_string(), 1),  // Pre-base forms
                    ("blwf".to_string(), 1),  // Below-base forms
                    ("pstf".to_string(), 1),  // Post-base forms
                ]);
            }
            _ => {
                // For other scripts, enable common ligatures
                if !features.iter().any(|(f, _)| f == "liga") {
                    features.push(("liga".to_string(), 1));
                }
            }
        }
        
        features
    }
}
