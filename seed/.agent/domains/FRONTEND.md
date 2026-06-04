# Frontend domain overlay

Pulled in when the work is user-facing UI (web app, component library, design
work). Copy only if the repo actually ships a UI.

## Accessibility is a requirement, not a polish step
- Semantic HTML first; ARIA only to fill gaps semantics can't. Every interactive element is keyboard-reachable and has a visible focus state.
- Meet WCAG AA: color contrast, labels on inputs, alt text, no information conveyed by color alone.

## Verify in a browser, not just in your head
- A UI change is not done until you have seen it render and behave: the happy path, an error state, an empty state, and a loading state.
- Check responsive behavior at small and large widths. Confirm it works with the keyboard, not only the mouse.

## State & data
- Keep server state and UI state separate. Don't duplicate server data into ad-hoc local copies that drift.
- Handle the four states of every async view explicitly: loading, empty, error, success.

## Performance & safety
- Mind bundle size; lazy-load heavy routes. Avoid layout-thrash and unbounded re-renders.
- Never interpolate untrusted data into HTML; rely on the framework's escaping and treat `dangerouslySetInnerHTML`/`v-html` as a red flag that needs justification.
