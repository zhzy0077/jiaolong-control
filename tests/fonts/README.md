`gpui-var-test.ttf` is the authored rectangle-glyph variable font from Zed PR #62832,
commit 522b0e2ac290da1f6e101f09eae3bb6958940b15, licensed under Apache-2.0.
Source generator: https://github.com/ravi-arnan/zed/blob/522b0e2ac290da1f6e101f09eae3bb6958940b15/crates/gpui_wgpu/test_data/generate.py

`gpui-static-test.ttf` derives from that fixture at wght=400, renamed GpuiStaticTest,
with only space, A, and m retained so Chinese text must use the variable fallback.
These fonts are used by the rasterization regression tests.
