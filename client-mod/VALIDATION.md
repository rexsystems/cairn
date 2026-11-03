# Validation record

- Hard gate PASS: all six mod jars build; TypeScript/Vite build passes; 59 frontend tests and the shared Java interaction workflows pass. Java checks cover search, colors, per-widget size, presets, keyboard selection, scrolling, dragging, resize handles and viewport bounds. The Minecraft 26.3 menu was visually inspected with its normal font restored.
- Purpose gate PASS: the palette, solid surfaces, module list, inspector, font choice and spacing have written reasons in README.md. No decorative images, gradients, effects, invented testimonials or external navigation are used.
- Liveliness PASS: ENERGY 2 / RHYTHM 1 / MOTION 1; a repeated moss selection edge identifies Cairn, search leads module discovery, and the inspector separates configuration from activation.
- Craftsmanship PASS: each control has a state change or navigation action in the shared menu; settings are persisted; numeric inputs are bounded; settings pages are checked at twelve viewport sizes. Default muted text has at least 4.78:1 contrast on selected surfaces, and hovered controls use the brighter foreground. Pressed key labels choose black or white for contrast. Custom font assets and font-loader dependencies are absent from every bundled jar.

Native tests cover enabling/disabling while preserving user files, reuse of manually installed Cairn, foreign-file protection, compatibility of older instance JSON, target versions/bytecode, normal-font packaging and pre-launch checks using the next launch's mode without altering files.

Minecraft 26.3 was exercised in an isolated test directory; 1.21.1 was also launched with the mapped adapter. Forge 1.8/1.8.8/1.8.9 and Fabric 1.20.1 have compilation and artifact checks, not a completed gameplay test on this machine. Support is restricted to the six listed versions. No universal compatibility or in-game performance gain is claimed. The optional headless HUD CPU benchmark excludes Minecraft rendering and GPU work.
