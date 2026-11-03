# Jiaolong GPUI design

Editable source: `jiaolong-fan-control.op`. Ten 1008 × 704 artboards cover game, office, connection recovery, feature reservations, and app settings in Adwaita light and dark themes. PNG exports use matching names.

## Theme and layout

Adwaita blue accents (#3584E4), light background #FAFAFA, dark background #242424, rounded cards, quiet borders, a 208 px Chinese / 280 px English navigation column, and a 44 px header. The application uses the system fontconfig sans-serif default and system glyph fallback. `tokens.json` contains both palettes and spacing. In GPUI, apply the selected palette to the entire window. Switches have white thumbs, blue checked tracks, and gray unchecked tracks.

## Fan interactions

CPU and GPU RPM are sample values. CPU percentage follows the original RPM / 60 calculation. The fixed 2200–5800 RPM slider highlights the current mode’s permitted range; dragging previews a 100 RPM target and release writes it to firmware. Minus and plus change the target by 100 RPM and enable maximum-fan control. Refresh uses the Linux firmware interface; automatic polling runs every 10 seconds. Saved targets restore when the firmware mode changes.

Game range: 3500–5000 RPM, default 4300. Office range: 2200–3500 RPM, default 2700. Performance range: 5000–5800 RPM, default 5300. Custom mode uses performance limits. Recovery state shows unavailable readings and disabled controls.

## Original EXE feature reservations

The sidebar and expansion boards reserve separate pages for the following functions, identified from the original application:

- Hardware overview: CPU/GPU temperature, frequency and utilization; CPU name/core count; memory usage/frequency; storage usage.
- Performance tuning: office/game/performance/custom modes; CPU thermal wall (`CPUTempWallSlider`), SPL/SPPT power limits, GPU core clock offset (`GPUCoreClockSlider`).
- Keyboard and lighting: brightness, RGB color/effects, ambient lighting (`PCAmbientlight`), Fn/Num/Caps lock state.
- Device settings: discrete graphics direct mode (`IndependentConnection`), touchpad toggle (`TPLockButton`), OSD icon toggle (`OSDIconButton`).

Reserved entries carry the 预留 label. Their cards establish space for later implementation. The fan page provides an office/game/performance picker and displays the active mode and RPM limits.

## App settings

A bottom sidebar entry opens application preferences, separate from hardware device settings. Both light and dark artboards include three exclusive appearance choices: System, Light, and Dark. The sample selects System and displays the currently resolved desktop appearance. A language dropdown offers 简体中文 and English, with 简体中文 selected in the design.

Intended GPUI behavior: apply changes immediately and persist them across launches. System mode follows desktop appearance changes; explicit Light/Dark overrides desktop appearance. Language changes update the interface. Store application preferences separately from per-mode fan targets.
