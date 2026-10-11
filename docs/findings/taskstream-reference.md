# Taskstream workspace reference

Inspected 11 October 2026 for the requested near-term design-system and homepage/docs tasks. The local repository is `/Users/me/src/me/todo-lofi`; its root JavaScript package is named `taskstream`. It has `apps/taskstream-web` and `apps/taskstream-desktop`, rather than a sibling checkout at `/Users/me/src/me/taskstream`.

## Observed structure

| Reference path | Purpose and useful pattern |
| --- | --- |
| [Root package](../../../todo-lofi/package.json) | pnpm workspace orchestration for codegen, site/app builds, typechecking, and tests. |
| [Design system](../../../todo-lofi/libs/web-design-system/README.md) | Separate TypeScript package: Panda tokens/recipes, Ark UI headless layer, Solid components, and Astro wrappers. Explicit component/preset/headless/style entry points. |
| [Docs/homepage app](../../../todo-lofi/apps/docs-web/README.md) | Separate Astro site consuming that design system, with common marketing/docs layouts. |
| [Docs navigation](../../../todo-lofi/apps/docs-web/src/data/docs.ts) | User and contributor guides, shared header tabs, and nested guide-specific sidebar structure driven by navigation data. |
| [Web app package](../../../todo-lofi/apps/taskstream-web/package.json) | Separate Solid web application consuming the shared package. |

The site separates `/` marketing, `/downloads`, `/roadmap`, and `/docs` user/contributor material. Its roadmap uses Now/Next/Later/Exploring. Tech-stack details live in architecture documentation rather than the homepage's primary product story.

## Proposed application here

The [master todo list](../todo/master-todo.md) plans a standalone TypeScript design-system package, a separate homepage/docs site consuming it, and shared components for the Solid web/Tauri frontend. Taskstream's package/site boundaries and navigation are the reference; Astro is an implementation candidate for the public site, not a new confirmed requirement from the interview.

Reproduce an appropriate workspace structure rather than depend on this external checkout at runtime. Review licensing and product-independent reuse before adapting code. Homepage content must come from Money Tracker's [own product questions and solutions](../spec/product-goals.md), with shipped versus provisional features explicit.
