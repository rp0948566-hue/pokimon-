# User Design Handoff Guide

Welcome! This document outlines where and how to integrate your visual designs into the PokéRescue Sentinel scaffold.

## 1. Where to Place Visual Design Assets
- **Design Tokens (Colors, Typography, Spacing)**: Add your CSS variables into `apps/desktop/src/styles/design-tokens.css`.
- **Global Styles & Resets**: Edit `apps/desktop/src/styles/index.css`.
- **Component Primitives**: Visual buttons, badges, modals, and cards belong under `apps/desktop/src/components/ui/`.
- **Icons**: Place SVG icons in `apps/desktop/src/assets/icons/`.
- **Images & Logos**: Place raster/vector brand assets in `apps/desktop/src/assets/images/`.

## 2. Page & Layout Slots
- **Command Center Layout**: `apps/desktop/src/layouts/CommandCenterLayout.tsx`.
- **Responder Field Layout**: `apps/desktop/src/layouts/ResponderLayout.tsx`.
- **Page Views**: Located under `apps/desktop/src/pages/<page-name>/`. Each folder contains a clear placeholder waiting for your layout.

## 3. Design Authority Rule
No agent will modify your styles, colors, or visual layouts without explicit instruction.
