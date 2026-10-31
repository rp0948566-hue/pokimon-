# Accessibility Checklist (WCAG 2.1 AA Target)

All future UI components implemented in PokéRescue Sentinel must satisfy:

- [ ] **Keyboard Navigation**: All interactive elements (buttons, inputs, tabs, modals) accessible via `Tab` / `Shift+Tab` and triggerable via `Enter` / `Space`.
- [ ] **Focus Indication**: Clearly visible focus ring on every focused interactive element.
- [ ] **Color Contrast**: Minimum 4.5:1 contrast ratio for body text against backgrounds; 3:1 for large text and UI status indicators.
- [ ] **Color Independence**: Status information (Passable, Blocked, Stale) must use icons and text labels in addition to color.
- [ ] **Screen Readers**: Meaningful `aria-label`, `aria-expanded`, and `role` attributes on custom widgets.
- [ ] **Reduced Motion**: Respect `prefers-reduced-motion` media queries across all animations and transitions.
