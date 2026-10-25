# SciCanvas 第三方版权与许可证

本文件随 Windows x64 发行包一起提供。应用自有部分沿用 MIT OR Apache-2.0；下列第三方代码与字体保留各自许可。

清单来自 Cargo.lock 和 Windows normal/build 完整特性依赖树，保守包含构建期工具；不是对每个包都被链接进 exe 的判断。原始版权、许可和 NOTICE 文本保存在对应目录，许可 OR/AND 表达式保留原样。

第三方软件包的原始许可和版权声明按 Cargo.lock 一并提供。egui 内嵌字体另保留原始字体许可和字体 name 表声明，包含 Hack 的 Bitstream Vera 声明。

Windows 系统字体通过用户电脑读取，本发行包不附加微软雅黑/Arial 字体文件；系统 DLL 同样不随包复制。Rust 标准库版权材料在发行包的 licenses/rust-runtime 中，按实际构建工具链附带。

## 包清单

| 包 / 版本 | 声明许可 | 原始声明文件 |
| --- | --- | --- |
| ab_glyph 0.2.32 | Apache-2.0 | [LICENSE](licenses/third-party/ab_glyph-0.2.32/LICENSE) |
| ab_glyph_rasterizer 0.1.10 | Apache-2.0 | [LICENSE](licenses/third-party/ab_glyph_rasterizer-0.1.10/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/ab_glyph_rasterizer-0.1.10/SOURCE_COPYRIGHT_HEADERS.txt) |
| accesskit 0.16.3 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/accesskit-0.16.3/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE-APACHE](licenses/third-party/accesskit-0.16.3/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/accesskit-0.16.3/upstream/LICENSE-MIT) |
| accesskit_consumer 0.24.3 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/accesskit_consumer-0.24.3/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE-APACHE](licenses/third-party/accesskit_consumer-0.24.3/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/accesskit_consumer-0.24.3/upstream/LICENSE-MIT) |
| accesskit_windows 0.23.2 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/accesskit_windows-0.23.2/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE-APACHE](licenses/third-party/accesskit_windows-0.23.2/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/accesskit_windows-0.23.2/upstream/LICENSE-MIT) |
| accesskit_winit 0.22.4 | Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/accesskit_winit-0.22.4/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE-APACHE](licenses/third-party/accesskit_winit-0.22.4/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/accesskit_winit-0.22.4/upstream/LICENSE-MIT) |
| adler2 2.0.1 | 0BSD OR MIT OR Apache-2.0 | [LICENSE-0BSD](licenses/third-party/adler2-2.0.1/LICENSE-0BSD), [LICENSE-APACHE](licenses/third-party/adler2-2.0.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/adler2-2.0.1/LICENSE-MIT) |
| ahash 0.8.12 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/ahash-0.8.12/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/ahash-0.8.12/LICENSE-MIT) |
| arboard 3.6.1 | MIT OR Apache-2.0 | [LICENSE-APACHE.txt](licenses/third-party/arboard-3.6.1/LICENSE-APACHE.txt), [LICENSE-MIT.txt](licenses/third-party/arboard-3.6.1/LICENSE-MIT.txt), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/arboard-3.6.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| arrayref 0.3.9 | BSD-2-Clause | [LICENSE](licenses/third-party/arrayref-0.3.9/LICENSE) |
| arrayvec 0.7.8 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/arrayvec-0.7.8/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/arrayvec-0.7.8/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/arrayvec-0.7.8/SOURCE_COPYRIGHT_HEADERS.txt) |
| ash 0.38.0+1.3.281 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/ash-0.38.0+1.3.281/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/ash-0.38.0+1.3.281/LICENSE-MIT) |
| autocfg 1.5.1 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/autocfg-1.5.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/autocfg-1.5.1/LICENSE-MIT) |
| base64 0.21.7 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/base64-0.21.7/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/base64-0.21.7/LICENSE-MIT) |
| base64 0.22.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/base64-0.22.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/base64-0.22.1/LICENSE-MIT) |
| bit-set 0.6.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/bit-set-0.6.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bit-set-0.6.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/bit-set-0.6.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| bit-vec 0.7.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/bit-vec-0.7.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bit-vec-0.7.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/bit-vec-0.7.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| bitflags 1.3.2 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/bitflags-1.3.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bitflags-1.3.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/bitflags-1.3.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| bitflags 2.13.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/bitflags-2.13.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bitflags-2.13.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/bitflags-2.13.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| bytemuck 1.25.2 | Zlib OR Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/bytemuck-1.25.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bytemuck-1.25.2/LICENSE-MIT), [LICENSE-ZLIB](licenses/third-party/bytemuck-1.25.2/LICENSE-ZLIB) |
| bytemuck_derive 1.12.1 | Zlib OR Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/bytemuck_derive-1.12.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/bytemuck_derive-1.12.1/LICENSE-MIT), [LICENSE-ZLIB](licenses/third-party/bytemuck_derive-1.12.1/LICENSE-ZLIB) |
| byteorder-lite 0.1.0 | Unlicense OR MIT | [LICENSE-MIT](licenses/third-party/byteorder-lite-0.1.0/LICENSE-MIT), [UNLICENSE](licenses/third-party/byteorder-lite-0.1.0/UNLICENSE) |
| cfg-if 1.0.5 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/cfg-if-1.0.5/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/cfg-if-1.0.5/LICENSE-MIT) |
| cfg_aliases 0.1.1 | MIT | [LICENSE](licenses/third-party/cfg_aliases-0.1.1/LICENSE), [NOTICES.md](licenses/third-party/cfg_aliases-0.1.1/NOTICES.md) |
| cfg_aliases 0.2.2 | MIT | [LICENSE](licenses/third-party/cfg_aliases-0.2.2/LICENSE), [NOTICES.md](licenses/third-party/cfg_aliases-0.2.2/NOTICES.md) |
| chrono 0.4.45 | MIT OR Apache-2.0 | [LICENSE.txt](licenses/third-party/chrono-0.4.45/LICENSE.txt), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/chrono-0.4.45/SOURCE_COPYRIGHT_HEADERS.txt) |
| clipboard-win 5.4.3 | BSL-1.0 | [LICENSE](licenses/third-party/clipboard-win-5.4.3/upstream/LICENSE) |
| codespan-reporting 0.11.1 | Apache-2.0 | [LICENSE](licenses/third-party/codespan-reporting-0.11.1/upstream/LICENSE) |
| com 0.6.0 | MIT | [LICENSE](licenses/third-party/com-0.6.0/LICENSE), [LICENSE](licenses/third-party/com-0.6.0/upstream/LICENSE) |
| com_macros 0.6.0 | MIT | [LICENSE](licenses/third-party/com_macros-0.6.0/upstream/LICENSE) |
| com_macros_support 0.6.0 | MIT | [LICENSE](licenses/third-party/com_macros_support-0.6.0/upstream/LICENSE) |
| core_detect 1.0.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/core_detect-1.0.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/core_detect-1.0.0/LICENSE-MIT) |
| core_maths 0.1.1 | MIT | [LICENSE](licenses/third-party/core_maths-0.1.1/LICENSE) |
| crc32fast 1.5.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/crc32fast-1.5.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/crc32fast-1.5.2/LICENSE-MIT) |
| cursor-icon 1.2.0 | MIT OR Apache-2.0 OR Zlib | [LICENSE-APACHE](licenses/third-party/cursor-icon-1.2.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/cursor-icon-1.2.0/LICENSE-MIT), [LICENSE-ZLIB](licenses/third-party/cursor-icon-1.2.0/LICENSE-ZLIB), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/cursor-icon-1.2.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| data-url 0.3.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/data-url-0.3.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/data-url-0.3.2/LICENSE-MIT) |
| displaydoc 0.2.7 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/displaydoc-0.2.7/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/displaydoc-0.2.7/LICENSE-MIT) |
| document-features 0.2.12 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/document-features-0.2.12/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/document-features-0.2.12/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/document-features-0.2.12/SOURCE_COPYRIGHT_HEADERS.txt) |
| dpi 0.1.2 | Apache-2.0 AND MIT | [LICENSE](licenses/third-party/dpi-0.1.2/LICENSE), [LICENSE-LIBM-MIT](licenses/third-party/dpi-0.1.2/LICENSE-LIBM-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/dpi-0.1.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| ecolor 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/ecolor-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/ecolor-0.29.1/upstream/LICENSE-MIT) |
| eframe 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/eframe-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/eframe-0.29.1/upstream/LICENSE-MIT) |
| egui 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/egui-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/egui-0.29.1/upstream/LICENSE-MIT) |
| egui-wgpu 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/egui-wgpu-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/egui-wgpu-0.29.1/upstream/LICENSE-MIT) |
| egui-winit 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/egui-winit-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/egui-winit-0.29.1/upstream/LICENSE-MIT) |
| egui_extras 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/egui_extras-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/egui_extras-0.29.1/upstream/LICENSE-MIT) |
| egui_glow 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/egui_glow-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/egui_glow-0.29.1/upstream/LICENSE-MIT) |
| emath 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/emath-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/emath-0.29.1/upstream/LICENSE-MIT) |
| encoding_rs 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | [COPYRIGHT](licenses/third-party/encoding_rs-0.8.42/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/encoding_rs-0.8.42/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/encoding_rs-0.8.42/LICENSE-MIT), [LICENSE-WHATWG](licenses/third-party/encoding_rs-0.8.42/LICENSE-WHATWG), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/encoding_rs-0.8.42/SOURCE_COPYRIGHT_HEADERS.txt) |
| enum-map 2.7.3 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/enum-map-2.7.3/SOURCE_COPYRIGHT_HEADERS.txt), [Apache-2.0.txt](licenses/third-party/enum-map-2.7.3/upstream/LICENSES/Apache-2.0.txt), [MIT.txt](licenses/third-party/enum-map-2.7.3/upstream/LICENSES/MIT.txt) |
| enum-map-derive 0.17.0 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/enum-map-derive-0.17.0/SOURCE_COPYRIGHT_HEADERS.txt), [Apache-2.0.txt](licenses/third-party/enum-map-derive-0.17.0/upstream/LICENSES/Apache-2.0.txt), [MIT.txt](licenses/third-party/enum-map-derive-0.17.0/upstream/LICENSES/MIT.txt) |
| epaint 0.29.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/epaint-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/epaint-0.29.1/upstream/LICENSE-MIT) |
| epaint_default_fonts 0.29.1 | (MIT OR Apache-2.0) AND OFL-1.1 AND LicenseRef-UFL-1.0 | [Hack-Regular.txt](licenses/third-party/epaint_default_fonts-0.29.1/fonts/Hack-Regular.txt), [OFL.txt](licenses/third-party/epaint_default_fonts-0.29.1/fonts/OFL.txt), [UFL.txt](licenses/third-party/epaint_default_fonts-0.29.1/fonts/UFL.txt), [emoji-icon-font-mit-license.txt](licenses/third-party/epaint_default_fonts-0.29.1/fonts/emoji-icon-font-mit-license.txt), [LICENSE-APACHE](licenses/third-party/epaint_default_fonts-0.29.1/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/epaint_default_fonts-0.29.1/upstream/LICENSE-MIT) |
| equivalent 1.0.3 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/equivalent-1.0.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/equivalent-1.0.3/LICENSE-MIT) |
| error-code 3.4.0 | BSL-1.0 | [LICENSE](licenses/third-party/error-code-3.4.0/LICENSE) |
| fdeflate 0.3.7 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/fdeflate-0.3.7/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/fdeflate-0.3.7/LICENSE-MIT) |
| flate2 1.1.10 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/flate2-1.1.10/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/flate2-1.1.10/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/flate2-1.1.10/SOURCE_COPYRIGHT_HEADERS.txt) |
| float-cmp 0.9.0 | MIT | [LICENSE](licenses/third-party/float-cmp-0.9.0/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/float-cmp-0.9.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| foldhash 0.1.5 | Zlib | [LICENSE](licenses/third-party/foldhash-0.1.5/LICENSE) |
| font-types 0.11.3 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/font-types-0.11.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/font-types-0.11.3/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/font-types-0.11.3/SOURCE_COPYRIGHT_HEADERS.txt) |
| fontdb 0.23.0 | MIT | [LICENSE](licenses/third-party/fontdb-0.23.0/LICENSE), [LICENSE.txt](licenses/third-party/fontdb-0.23.0/tests/fonts/LICENSE.txt) |
| form_urlencoded 1.2.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/form_urlencoded-1.2.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/form_urlencoded-1.2.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/form_urlencoded-1.2.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| gl_generator 0.14.0 | Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/gl_generator-0.14.0/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE](licenses/third-party/gl_generator-0.14.0/upstream/LICENSE) |
| glow 0.13.1 | MIT OR Apache-2.0 OR Zlib | [LICENSE-APACHE](licenses/third-party/glow-0.13.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/glow-0.13.1/LICENSE-MIT), [LICENSE-ZLIB](licenses/third-party/glow-0.13.1/LICENSE-ZLIB) |
| glow 0.14.2 | MIT OR Apache-2.0 OR Zlib | [LICENSE-APACHE](licenses/third-party/glow-0.14.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/glow-0.14.2/LICENSE-MIT), [LICENSE-ZLIB](licenses/third-party/glow-0.14.2/LICENSE-ZLIB) |
| glutin 0.32.3 | Apache-2.0 | [LICENSE](licenses/third-party/glutin-0.32.3/LICENSE) |
| glutin-winit 0.5.0 | MIT | [LICENSE](licenses/third-party/glutin-winit-0.5.0/LICENSE) |
| glutin_egl_sys 0.7.1 | Apache-2.0 | [LICENSE](licenses/third-party/glutin_egl_sys-0.7.1/LICENSE) |
| glutin_wgl_sys 0.6.1 | Apache-2.0 | [LICENSE](licenses/third-party/glutin_wgl_sys-0.6.1/LICENSE) |
| gpu-alloc 0.6.2 | MIT OR Apache-2.0 | [COPYING](licenses/third-party/gpu-alloc-0.6.2/upstream/COPYING), [APACHE](licenses/third-party/gpu-alloc-0.6.2/upstream/license/APACHE), [MIT](licenses/third-party/gpu-alloc-0.6.2/upstream/license/MIT) |
| gpu-alloc-types 0.3.1 | MIT OR Apache-2.0 | [COPYING](licenses/third-party/gpu-alloc-types-0.3.1/upstream/COPYING), [APACHE](licenses/third-party/gpu-alloc-types-0.3.1/upstream/license/APACHE), [MIT](licenses/third-party/gpu-alloc-types-0.3.1/upstream/license/MIT) |
| gpu-allocator 0.26.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/gpu-allocator-0.26.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/gpu-allocator-0.26.0/LICENSE-MIT) |
| gpu-descriptor 0.3.2 | MIT OR Apache-2.0 | [COPYING](licenses/third-party/gpu-descriptor-0.3.2/upstream/COPYING), [APACHE](licenses/third-party/gpu-descriptor-0.3.2/upstream/license/APACHE), [MIT](licenses/third-party/gpu-descriptor-0.3.2/upstream/license/MIT) |
| gpu-descriptor-types 0.2.0 | MIT OR Apache-2.0 | [COPYING](licenses/third-party/gpu-descriptor-types-0.2.0/upstream/COPYING), [APACHE](licenses/third-party/gpu-descriptor-types-0.2.0/upstream/license/APACHE), [MIT](licenses/third-party/gpu-descriptor-types-0.2.0/upstream/license/MIT) |
| hashbrown 0.15.5 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/hashbrown-0.15.5/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/hashbrown-0.15.5/LICENSE-MIT) |
| hashbrown 0.17.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/hashbrown-0.17.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/hashbrown-0.17.1/LICENSE-MIT) |
| hassle-rs 0.11.0 | MIT | [LICENSE](licenses/third-party/hassle-rs-0.11.0/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/hassle-rs-0.11.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| hexf-parse 0.2.1 | CC0-1.0 | [DECLARATION.toml](licenses/third-party/hexf-parse-0.2.1/upstream/DECLARATION.toml), [LICENSE-CC0.txt](licenses/third-party/hexf-parse-0.2.1/upstream/LICENSE-CC0.txt) |
| icu_collections 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_collections-2.3.0/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/icu_collections-2.3.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| icu_locale_core 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_locale_core-2.3.0/LICENSE) |
| icu_normalizer 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_normalizer-2.3.0/LICENSE) |
| icu_normalizer_data 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_normalizer_data-2.3.0/LICENSE) |
| icu_properties 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_properties-2.3.0/LICENSE) |
| icu_properties_data 2.3.0 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_properties_data-2.3.0/LICENSE) |
| icu_provider 2.3.1 | Unicode-3.0 | [LICENSE](licenses/third-party/icu_provider-2.3.1/LICENSE) |
| idna 1.1.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/idna-1.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/idna-1.1.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/idna-1.1.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| idna_adapter 1.2.2 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/idna_adapter-1.2.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/idna_adapter-1.2.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/idna_adapter-1.2.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| image 0.25.10 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/image-0.25.10/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/image-0.25.10/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/image-0.25.10/SOURCE_COPYRIGHT_HEADERS.txt) |
| imagesize 0.13.0 | MIT | [LICENSE](licenses/third-party/imagesize-0.13.0/LICENSE) |
| immutable-chunkmap 2.1.4 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/immutable-chunkmap-2.1.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/immutable-chunkmap-2.1.4/LICENSE-MIT) |
| indexmap 2.14.2 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/indexmap-2.14.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/indexmap-2.14.2/LICENSE-MIT) |
| itoa 1.0.18 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/itoa-1.0.18/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/itoa-1.0.18/LICENSE-MIT) |
| khronos-egl 6.0.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/khronos-egl-6.0.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/khronos-egl-6.0.0/LICENSE-MIT) |
| khronos_api 3.1.0 | Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/khronos_api-3.1.0/SOURCE_COPYRIGHT_HEADERS.txt), [LICENSE](licenses/third-party/khronos_api-3.1.0/upstream/LICENSE) |
| kurbo 0.11.3 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/kurbo-0.11.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/kurbo-0.11.3/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/kurbo-0.11.3/SOURCE_COPYRIGHT_HEADERS.txt) |
| kurbo 0.13.1 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/kurbo-0.13.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/kurbo-0.13.1/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/kurbo-0.13.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| libc 0.2.190 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/libc-0.2.190/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/libc-0.2.190/LICENSE-MIT) |
| libloading 0.8.9 | ISC | [LICENSE](licenses/third-party/libloading-0.8.9/LICENSE) |
| libm 0.2.16 | MIT | [LICENSE.txt](licenses/third-party/libm-0.2.16/LICENSE.txt), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/libm-0.2.16/SOURCE_COPYRIGHT_HEADERS.txt) |
| litemap 0.8.3 | Unicode-3.0 | [LICENSE](licenses/third-party/litemap-0.8.3/LICENSE) |
| litrs 1.0.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/litrs-1.0.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/litrs-1.0.1/LICENSE-MIT) |
| lock_api 0.4.14 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/lock_api-0.4.14/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/lock_api-0.4.14/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/lock_api-0.4.14/SOURCE_COPYRIGHT_HEADERS.txt) |
| log 0.4.34 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/log-0.4.34/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/log-0.4.34/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/log-0.4.34/SOURCE_COPYRIGHT_HEADERS.txt) |
| memchr 2.8.3 | Unlicense OR MIT | [COPYING](licenses/third-party/memchr-2.8.3/COPYING), [LICENSE-MIT](licenses/third-party/memchr-2.8.3/LICENSE-MIT), [UNLICENSE](licenses/third-party/memchr-2.8.3/UNLICENSE) |
| memmap2 0.9.11 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/memmap2-0.9.11/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/memmap2-0.9.11/LICENSE-MIT) |
| memoffset 0.9.1 | MIT | [LICENSE](licenses/third-party/memoffset-0.9.1/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/memoffset-0.9.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| miniz_oxide 0.8.9 | MIT OR Zlib OR Apache-2.0 | [LICENSE](licenses/third-party/miniz_oxide-0.8.9/LICENSE), [LICENSE-APACHE.md](licenses/third-party/miniz_oxide-0.8.9/LICENSE-APACHE.md), [LICENSE-MIT.md](licenses/third-party/miniz_oxide-0.8.9/LICENSE-MIT.md), [LICENSE-ZLIB.md](licenses/third-party/miniz_oxide-0.8.9/LICENSE-ZLIB.md) |
| miniz_oxide 0.9.1 | MIT OR Zlib OR Apache-2.0 | [LICENSE](licenses/third-party/miniz_oxide-0.9.1/LICENSE), [LICENSE-APACHE.md](licenses/third-party/miniz_oxide-0.9.1/LICENSE-APACHE.md), [LICENSE-MIT.md](licenses/third-party/miniz_oxide-0.9.1/LICENSE-MIT.md), [LICENSE-ZLIB.md](licenses/third-party/miniz_oxide-0.9.1/LICENSE-ZLIB.md) |
| moxcms 0.8.1 | BSD-3-Clause OR Apache-2.0 | [LICENSE-APACHE.md](licenses/third-party/moxcms-0.8.1/LICENSE-APACHE.md), [LICENSE.md](licenses/third-party/moxcms-0.8.1/LICENSE.md), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/moxcms-0.8.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| multiversion_no_op 1.0.0 | Apache-2.0 OR MIT | [COPYRIGHT](licenses/third-party/multiversion_no_op-1.0.0/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/multiversion_no_op-1.0.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/multiversion_no_op-1.0.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/multiversion_no_op-1.0.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| naga 22.1.0 | MIT OR Apache-2.0 | [LICENSE.APACHE](licenses/third-party/naga-22.1.0/upstream/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/naga-22.1.0/upstream/LICENSE.MIT) |
| nohash-hasher 0.2.0 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/nohash-hasher-0.2.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/nohash-hasher-0.2.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/nohash-hasher-0.2.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| num-traits 0.2.19 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/num-traits-0.2.19/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/num-traits-0.2.19/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/num-traits-0.2.19/SOURCE_COPYRIGHT_HEADERS.txt) |
| once_cell 1.21.4 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/once_cell-1.21.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/once_cell-1.21.4/LICENSE-MIT) |
| owned_ttf_parser 0.25.1 | Apache-2.0 | [LICENSE](licenses/third-party/owned_ttf_parser-0.25.1/LICENSE) |
| parking_lot 0.12.5 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/parking_lot-0.12.5/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/parking_lot-0.12.5/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/parking_lot-0.12.5/SOURCE_COPYRIGHT_HEADERS.txt) |
| parking_lot_core 0.9.12 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/parking_lot_core-0.9.12/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/parking_lot_core-0.9.12/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/parking_lot_core-0.9.12/SOURCE_COPYRIGHT_HEADERS.txt) |
| paste 1.0.15 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/paste-1.0.15/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/paste-1.0.15/LICENSE-MIT) |
| pdf-writer 0.12.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/pdf-writer-0.12.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/pdf-writer-0.12.1/LICENSE-MIT) |
| percent-encoding 2.3.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/percent-encoding-2.3.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/percent-encoding-2.3.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/percent-encoding-2.3.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| pico-args 0.5.0 | MIT | [LICENSE](licenses/third-party/pico-args-0.5.0/LICENSE) |
| pin-project-lite 0.2.17 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/pin-project-lite-0.2.17/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/pin-project-lite-0.2.17/LICENSE-MIT) |
| png 0.17.16 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/png-0.17.16/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/png-0.17.16/LICENSE-MIT) |
| png 0.18.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/png-0.18.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/png-0.18.1/LICENSE-MIT) |
| pollster 0.3.0 | Apache-2.0/MIT | [LICENSE-APACHE](licenses/third-party/pollster-0.3.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/pollster-0.3.0/LICENSE-MIT) |
| polycool 0.4.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/polycool-0.4.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/polycool-0.4.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/polycool-0.4.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| potential_utf 0.1.6 | Unicode-3.0 | [LICENSE](licenses/third-party/potential_utf-0.1.6/LICENSE) |
| presser 0.3.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/presser-0.3.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/presser-0.3.1/LICENSE-MIT) |
| proc-macro2 1.0.107 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/proc-macro2-1.0.107/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/proc-macro2-1.0.107/LICENSE-MIT) |
| profiling 1.0.18 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/profiling-1.0.18/upstream/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/profiling-1.0.18/upstream/LICENSE-MIT) |
| pxfm 0.1.30 | BSD-3-Clause OR Apache-2.0 | [LICENSE-APACHE.md](licenses/third-party/pxfm-0.1.30/LICENSE-APACHE.md), [LICENSE.md](licenses/third-party/pxfm-0.1.30/LICENSE.md), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/pxfm-0.1.30/SOURCE_COPYRIGHT_HEADERS.txt) |
| quote 1.0.47 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/quote-1.0.47/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/quote-1.0.47/LICENSE-MIT) |
| raw-window-handle 0.6.2 | MIT OR Apache-2.0 OR Zlib | [LICENSE-APACHE.md](licenses/third-party/raw-window-handle-0.6.2/LICENSE-APACHE.md), [LICENSE-MIT.md](licenses/third-party/raw-window-handle-0.6.2/LICENSE-MIT.md), [LICENSE-ZLIB.md](licenses/third-party/raw-window-handle-0.6.2/LICENSE-ZLIB.md) |
| read-fonts 0.39.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/read-fonts-0.39.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/read-fonts-0.39.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/read-fonts-0.39.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| renderdoc-sys 1.1.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/renderdoc-sys-1.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/renderdoc-sys-1.1.0/LICENSE-MIT) |
| resvg 0.45.1 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/resvg-0.45.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/resvg-0.45.1/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/resvg-0.45.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| rfd 0.15.4 | MIT | [LICENSE](licenses/third-party/rfd-0.15.4/LICENSE) |
| rgb 0.8.53 | MIT | [LICENSE](licenses/third-party/rgb-0.8.53/LICENSE) |
| roxmltree 0.20.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/roxmltree-0.20.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/roxmltree-0.20.0/LICENSE-MIT) |
| rustc-hash 1.1.0 | Apache-2.0/MIT | [LICENSE-APACHE](licenses/third-party/rustc-hash-1.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/rustc-hash-1.1.0/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/rustc-hash-1.1.0/SOURCE_COPYRIGHT_HEADERS.txt) |
| rustc-hash 2.1.3 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/rustc-hash-2.1.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/rustc-hash-2.1.3/LICENSE-MIT) |
| rustversion 1.0.23 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/rustversion-1.0.23/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/rustversion-1.0.23/LICENSE-MIT) |
| rustybuzz 0.20.1 | MIT | [LICENSE](licenses/third-party/rustybuzz-0.20.1/LICENSE), [COPYING](licenses/third-party/rustybuzz-0.20.1/scripts/ms-use/COPYING) |
| ryu 1.0.23 | Apache-2.0 OR BSL-1.0 | [LICENSE-APACHE](licenses/third-party/ryu-1.0.23/LICENSE-APACHE), [LICENSE-BOOST](licenses/third-party/ryu-1.0.23/LICENSE-BOOST), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/ryu-1.0.23/SOURCE_COPYRIGHT_HEADERS.txt) |
| scopeguard 1.2.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/scopeguard-1.2.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/scopeguard-1.2.0/LICENSE-MIT) |
| serde 1.0.229 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/serde-1.0.229/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/serde-1.0.229/LICENSE-MIT) |
| serde_core 1.0.229 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/serde_core-1.0.229/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/serde_core-1.0.229/LICENSE-MIT) |
| serde_derive 1.0.229 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/serde_derive-1.0.229/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/serde_derive-1.0.229/LICENSE-MIT) |
| serde_json 1.0.154 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/serde_json-1.0.154/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/serde_json-1.0.154/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/serde_json-1.0.154/SOURCE_COPYRIGHT_HEADERS.txt) |
| simd-adler32 0.3.10 | MIT | [LICENSE.md](licenses/third-party/simd-adler32-0.3.10/LICENSE.md) |
| simdutf8 0.1.5 | MIT OR Apache-2.0 | [LICENSE-Apache](licenses/third-party/simdutf8-0.1.5/LICENSE-Apache), [LICENSE-MIT](licenses/third-party/simdutf8-0.1.5/LICENSE-MIT) |
| simplecss 0.2.2 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/simplecss-0.2.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/simplecss-0.2.2/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/simplecss-0.2.2/SOURCE_COPYRIGHT_HEADERS.txt) |
| siphasher 1.0.4 | MIT OR Apache-2.0 | [COPYING](licenses/third-party/siphasher-1.0.4/COPYING), [LICENSE-APACHE](licenses/third-party/siphasher-1.0.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/siphasher-1.0.4/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/siphasher-1.0.4/SOURCE_COPYRIGHT_HEADERS.txt) |
| skrifa 0.42.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/skrifa-0.42.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/skrifa-0.42.1/LICENSE-MIT) |
| slotmap 1.1.1 | Zlib | [LICENSE](licenses/third-party/slotmap-1.1.1/LICENSE) |
| smallvec 1.16.3 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/smallvec-1.16.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/smallvec-1.16.3/LICENSE-MIT) |
| smol_str 0.2.2 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/smol_str-0.2.2/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/smol_str-0.2.2/LICENSE-MIT) |
| spirv 0.3.0+sdk-1.3.268.0 | Apache-2.0 | [LICENSE](licenses/third-party/spirv-0.3.0+sdk-1.3.268.0/upstream/LICENSE) |
| stable_deref_trait 1.2.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/stable_deref_trait-1.2.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/stable_deref_trait-1.2.1/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/stable_deref_trait-1.2.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| static_assertions 1.1.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/static_assertions-1.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/static_assertions-1.1.0/LICENSE-MIT) |
| strict-num 0.1.1 | MIT | [LICENSE](licenses/third-party/strict-num-0.1.1/LICENSE) |
| subsetter 0.2.6 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/subsetter-0.2.6/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/subsetter-0.2.6/LICENSE-MIT), [NOTICE](licenses/third-party/subsetter-0.2.6/NOTICE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/subsetter-0.2.6/SOURCE_COPYRIGHT_HEADERS.txt) |
| svg2pdf 0.13.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/svg2pdf-0.13.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/svg2pdf-0.13.0/LICENSE-MIT), [NOTICE](licenses/third-party/svg2pdf-0.13.0/NOTICE) |
| svgtypes 0.15.3 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/svgtypes-0.15.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/svgtypes-0.15.3/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/svgtypes-0.15.3/SOURCE_COPYRIGHT_HEADERS.txt) |
| syn 1.0.109 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/syn-1.0.109/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/syn-1.0.109/LICENSE-MIT) |
| syn 2.0.119 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/syn-2.0.119/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/syn-2.0.119/LICENSE-MIT) |
| syn 3.0.9 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/syn-3.0.9/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/syn-3.0.9/LICENSE-MIT) |
| synstructure 0.14.0 | MIT | [LICENSE](licenses/third-party/synstructure-0.14.0/LICENSE) |
| termcolor 1.4.1 | Unlicense OR MIT | [COPYING](licenses/third-party/termcolor-1.4.1/COPYING), [LICENSE-MIT](licenses/third-party/termcolor-1.4.1/LICENSE-MIT), [UNLICENSE](licenses/third-party/termcolor-1.4.1/UNLICENSE) |
| thiserror 1.0.69 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/thiserror-1.0.69/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/thiserror-1.0.69/LICENSE-MIT) |
| thiserror-impl 1.0.69 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/thiserror-impl-1.0.69/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/thiserror-impl-1.0.69/LICENSE-MIT) |
| tiny-skia 0.11.4 | BSD-3-Clause | [LICENSE](licenses/third-party/tiny-skia-0.11.4/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/tiny-skia-0.11.4/SOURCE_COPYRIGHT_HEADERS.txt) |
| tiny-skia-path 0.11.4 | BSD-3-Clause | [LICENSE](licenses/third-party/tiny-skia-path-0.11.4/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/tiny-skia-path-0.11.4/SOURCE_COPYRIGHT_HEADERS.txt) |
| tinystr 0.8.4 | Unicode-3.0 | [LICENSE](licenses/third-party/tinystr-0.8.4/LICENSE) |
| tinyvec 1.13.3 | Zlib OR Apache-2.0 OR MIT | [LICENSE-APACHE.md](licenses/third-party/tinyvec-1.13.3/LICENSE-APACHE.md), [LICENSE-MIT.md](licenses/third-party/tinyvec-1.13.3/LICENSE-MIT.md), [LICENSE-ZLIB.md](licenses/third-party/tinyvec-1.13.3/LICENSE-ZLIB.md) |
| tracing 0.1.44 | MIT | [LICENSE](licenses/third-party/tracing-0.1.44/LICENSE) |
| tracing-core 0.1.36 | MIT | [LICENSE](licenses/third-party/tracing-core-0.1.36/LICENSE), [LICENSE](licenses/third-party/tracing-core-0.1.36/src/spin/LICENSE) |
| ttf-parser 0.25.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/ttf-parser-0.25.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/ttf-parser-0.25.1/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/ttf-parser-0.25.1/SOURCE_COPYRIGHT_HEADERS.txt), [colr_1_LICENSE](licenses/third-party/ttf-parser-0.25.1/tests/fonts/colr_1_LICENSE) |
| type-map 0.5.1 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/type-map-0.5.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/type-map-0.5.1/LICENSE-MIT) |
| typed-path 0.12.3 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/typed-path-0.12.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/typed-path-0.12.3/LICENSE-MIT) |
| unicode-bidi 0.3.18 | MIT OR Apache-2.0 | [COPYRIGHT](licenses/third-party/unicode-bidi-0.3.18/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/unicode-bidi-0.3.18/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-bidi-0.3.18/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-bidi-0.3.18/SOURCE_COPYRIGHT_HEADERS.txt) |
| unicode-bidi-mirroring 0.4.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/unicode-bidi-mirroring-0.4.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-bidi-mirroring-0.4.0/LICENSE-MIT) |
| unicode-ccc 0.4.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/unicode-ccc-0.4.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-ccc-0.4.0/LICENSE-MIT) |
| unicode-ident 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | [LICENSE-APACHE](licenses/third-party/unicode-ident-1.0.27/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-ident-1.0.27/LICENSE-MIT), [LICENSE-UNICODE](licenses/third-party/unicode-ident-1.0.27/LICENSE-UNICODE) |
| unicode-properties 0.1.4 | MIT/Apache-2.0 | [COPYRIGHT](licenses/third-party/unicode-properties-0.1.4/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/unicode-properties-0.1.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-properties-0.1.4/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-properties-0.1.4/SOURCE_COPYRIGHT_HEADERS.txt) |
| unicode-script 0.5.8 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/unicode-script-0.5.8/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-script-0.5.8/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-script-0.5.8/SOURCE_COPYRIGHT_HEADERS.txt) |
| unicode-segmentation 1.13.3 | MIT OR Apache-2.0 | [COPYRIGHT](licenses/third-party/unicode-segmentation-1.13.3/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/unicode-segmentation-1.13.3/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-segmentation-1.13.3/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-segmentation-1.13.3/SOURCE_COPYRIGHT_HEADERS.txt) |
| unicode-vo 0.1.0 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/unicode-vo-0.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-vo-0.1.0/LICENSE-MIT) |
| unicode-width 0.1.14 | MIT OR Apache-2.0 | [COPYRIGHT](licenses/third-party/unicode-width-0.1.14/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/unicode-width-0.1.14/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-width-0.1.14/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-width-0.1.14/SOURCE_COPYRIGHT_HEADERS.txt) |
| unicode-xid 0.2.6 | MIT OR Apache-2.0 | [COPYRIGHT](licenses/third-party/unicode-xid-0.2.6/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/unicode-xid-0.2.6/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/unicode-xid-0.2.6/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/unicode-xid-0.2.6/SOURCE_COPYRIGHT_HEADERS.txt) |
| url 2.5.8 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/url-2.5.8/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/url-2.5.8/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/url-2.5.8/SOURCE_COPYRIGHT_HEADERS.txt) |
| usvg 0.45.1 | Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/usvg-0.45.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/usvg-0.45.1/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/usvg-0.45.1/SOURCE_COPYRIGHT_HEADERS.txt) |
| utf8_iter 1.0.4 | Apache-2.0 OR MIT | [COPYRIGHT](licenses/third-party/utf8_iter-1.0.4/COPYRIGHT), [LICENSE-APACHE](licenses/third-party/utf8_iter-1.0.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/utf8_iter-1.0.4/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/utf8_iter-1.0.4/SOURCE_COPYRIGHT_HEADERS.txt) |
| version_check 0.9.5 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/version_check-0.9.5/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/version_check-0.9.5/LICENSE-MIT) |
| web-time 1.1.0 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/web-time-1.1.0/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/web-time-1.1.0/LICENSE-MIT) |
| webbrowser 1.2.4 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/webbrowser-1.2.4/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/webbrowser-1.2.4/LICENSE-MIT) |
| wgpu 22.1.0 | MIT OR Apache-2.0 | [LICENSE.APACHE](licenses/third-party/wgpu-22.1.0/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-22.1.0/LICENSE.MIT), [LICENSE.APACHE](licenses/third-party/wgpu-22.1.0/upstream/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-22.1.0/upstream/LICENSE.MIT) |
| wgpu-core 22.1.0 | MIT OR Apache-2.0 | [LICENSE.APACHE](licenses/third-party/wgpu-core-22.1.0/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-core-22.1.0/LICENSE.MIT), [LICENSE.APACHE](licenses/third-party/wgpu-core-22.1.0/upstream/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-core-22.1.0/upstream/LICENSE.MIT) |
| wgpu-hal 22.0.0 | MIT OR Apache-2.0 | [LICENSE.APACHE](licenses/third-party/wgpu-hal-22.0.0/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-hal-22.0.0/LICENSE.MIT) |
| wgpu-types 22.0.0 | MIT OR Apache-2.0 | [LICENSE.APACHE](licenses/third-party/wgpu-types-22.0.0/LICENSE.APACHE), [LICENSE.MIT](licenses/third-party/wgpu-types-22.0.0/LICENSE.MIT) |
| widestring 1.2.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/widestring-1.2.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/widestring-1.2.1/LICENSE-MIT) |
| winapi 0.3.9 | MIT/Apache-2.0 | [LICENSE-APACHE](licenses/third-party/winapi-0.3.9/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/winapi-0.3.9/LICENSE-MIT) |
| winapi-util 0.1.11 | Unlicense OR MIT | [COPYING](licenses/third-party/winapi-util-0.1.11/COPYING), [LICENSE-MIT](licenses/third-party/winapi-util-0.1.11/LICENSE-MIT), [UNLICENSE](licenses/third-party/winapi-util-0.1.11/UNLICENSE) |
| windows 0.52.0 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/windows-0.52.0/SOURCE_COPYRIGHT_HEADERS.txt), [license-apache-2.0](licenses/third-party/windows-0.52.0/license-apache-2.0), [license-mit](licenses/third-party/windows-0.52.0/license-mit) |
| windows 0.58.0 | MIT OR Apache-2.0 | [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/windows-0.58.0/SOURCE_COPYRIGHT_HEADERS.txt), [license-apache-2.0](licenses/third-party/windows-0.58.0/license-apache-2.0), [license-mit](licenses/third-party/windows-0.58.0/license-mit) |
| windows-core 0.52.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-core-0.52.0/license-apache-2.0), [license-mit](licenses/third-party/windows-core-0.52.0/license-mit) |
| windows-core 0.58.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-core-0.58.0/license-apache-2.0), [license-mit](licenses/third-party/windows-core-0.58.0/license-mit) |
| windows-implement 0.58.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-implement-0.58.0/license-apache-2.0), [license-mit](licenses/third-party/windows-implement-0.58.0/license-mit) |
| windows-interface 0.58.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-interface-0.58.0/license-apache-2.0), [license-mit](licenses/third-party/windows-interface-0.58.0/license-mit) |
| windows-link 0.2.1 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-link-0.2.1/license-apache-2.0), [license-mit](licenses/third-party/windows-link-0.2.1/license-mit) |
| windows-result 0.2.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-result-0.2.0/license-apache-2.0), [license-mit](licenses/third-party/windows-result-0.2.0/license-mit) |
| windows-strings 0.1.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-strings-0.1.0/license-apache-2.0), [license-mit](licenses/third-party/windows-strings-0.1.0/license-mit) |
| windows-sys 0.52.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-sys-0.52.0/license-apache-2.0), [license-mit](licenses/third-party/windows-sys-0.52.0/license-mit) |
| windows-sys 0.59.0 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-sys-0.59.0/license-apache-2.0), [license-mit](licenses/third-party/windows-sys-0.59.0/license-mit) |
| windows-sys 0.60.2 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-sys-0.60.2/license-apache-2.0), [license-mit](licenses/third-party/windows-sys-0.60.2/license-mit) |
| windows-sys 0.61.2 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-sys-0.61.2/license-apache-2.0), [license-mit](licenses/third-party/windows-sys-0.61.2/license-mit) |
| windows-targets 0.52.6 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-targets-0.52.6/license-apache-2.0), [license-mit](licenses/third-party/windows-targets-0.52.6/license-mit) |
| windows-targets 0.53.5 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows-targets-0.53.5/license-apache-2.0), [license-mit](licenses/third-party/windows-targets-0.53.5/license-mit) |
| windows_x86_64_msvc 0.52.6 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows_x86_64_msvc-0.52.6/license-apache-2.0), [license-mit](licenses/third-party/windows_x86_64_msvc-0.52.6/license-mit) |
| windows_x86_64_msvc 0.53.1 | MIT OR Apache-2.0 | [license-apache-2.0](licenses/third-party/windows_x86_64_msvc-0.53.1/license-apache-2.0), [license-mit](licenses/third-party/windows_x86_64_msvc-0.53.1/license-mit) |
| winit 0.30.13 | Apache-2.0 | [LICENSE](licenses/third-party/winit-0.30.13/LICENSE), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/winit-0.30.13/SOURCE_COPYRIGHT_HEADERS.txt) |
| write-fonts 0.48.1 | MIT OR Apache-2.0 | [LICENSE-APACHE](licenses/third-party/write-fonts-0.48.1/LICENSE-APACHE), [LICENSE-MIT](licenses/third-party/write-fonts-0.48.1/LICENSE-MIT) |
| writeable 0.6.4 | Unicode-3.0 | [LICENSE](licenses/third-party/writeable-0.6.4/LICENSE) |
| xml-rs 0.8.29 | MIT | [LICENSE](licenses/third-party/xml-rs-0.8.29/LICENSE) |
| xmlwriter 0.1.0 | MIT | [LICENSE](licenses/third-party/xmlwriter-0.1.0/LICENSE) |
| yoke 0.8.3 | Unicode-3.0 | [LICENSE](licenses/third-party/yoke-0.8.3/LICENSE) |
| yoke-derive 0.8.4 | Unicode-3.0 | [LICENSE](licenses/third-party/yoke-derive-0.8.4/LICENSE) |
| zerocopy 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | [LICENSE-APACHE](licenses/third-party/zerocopy-0.8.63/LICENSE-APACHE), [LICENSE-BSD](licenses/third-party/zerocopy-0.8.63/LICENSE-BSD), [LICENSE-MIT](licenses/third-party/zerocopy-0.8.63/LICENSE-MIT), [SOURCE_COPYRIGHT_HEADERS.txt](licenses/third-party/zerocopy-0.8.63/SOURCE_COPYRIGHT_HEADERS.txt) |
| zerofrom 0.1.8 | Unicode-3.0 | [LICENSE](licenses/third-party/zerofrom-0.1.8/LICENSE) |
| zerofrom-derive 0.1.8 | Unicode-3.0 | [LICENSE](licenses/third-party/zerofrom-derive-0.1.8/LICENSE) |
| zerotrie 0.2.5 | Unicode-3.0 | [LICENSE](licenses/third-party/zerotrie-0.2.5/LICENSE) |
| zerovec 0.11.8 | Unicode-3.0 | [LICENSE](licenses/third-party/zerovec-0.11.8/LICENSE) |
| zerovec-derive 0.11.6 | Unicode-3.0 | [LICENSE](licenses/third-party/zerovec-derive-0.11.6/LICENSE) |
| zip 9.0.5 | MIT | [LICENSE](licenses/third-party/zip-9.0.5/LICENSE) |
| zmij 1.0.23 | MIT | [LICENSE-MIT](licenses/third-party/zmij-1.0.23/LICENSE-MIT) |

## 算法参考与代码来源复核

统计实现中的 AS R94 来源复核见 [版权检查记录](docs/COPYRIGHT_AUDIT.md)。保守保留 [SciPy BSD 声明](licenses/supplemental/scipy-1.17.1/LICENSE.txt)，该声明不代表 SciPy/NumPy 作为运行依赖被打包。
